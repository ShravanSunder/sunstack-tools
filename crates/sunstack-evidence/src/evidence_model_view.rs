use crate::{FieldName, FieldValue, ModelFieldValue, ModelView};
use std::collections::{BTreeMap, BTreeSet};

fn model_field(field: &FieldValue) -> ModelFieldValue {
    match field {
        FieldValue::Text { value, .. } => ModelFieldValue::Text(value.clone()),
        FieldValue::Texts { values, .. } => {
            ModelFieldValue::List(values.iter().cloned().map(ModelFieldValue::Text).collect())
        }
        FieldValue::Absent => ModelFieldValue::Absent,
        FieldValue::Unavailable { .. } => ModelFieldValue::Object(BTreeMap::from([(
            "unavailable".into(),
            ModelFieldValue::Boolean(true),
        )])),
        FieldValue::Targets { values } => ModelFieldValue::List(
            values
                .iter()
                .map(|target| {
                    let mut fields = BTreeMap::from([
                        ("name".into(), ModelFieldValue::Text(target.name.clone())),
                        (
                            "path".into(),
                            ModelFieldValue::Text(target.path.to_string_lossy().into_owned()),
                        ),
                        ("exists".into(), ModelFieldValue::Boolean(target.exists)),
                    ]);
                    if let Some(opening) = &target.opening {
                        fields.insert("opening".into(), ModelFieldValue::Text(opening.clone()));
                    }
                    if let Some(nested) = &target.fields {
                        fields.insert(
                            "fields".into(),
                            ModelFieldValue::Object(
                                nested
                                    .iter()
                                    .map(|(name, value)| (name.as_str().into(), model_field(value)))
                                    .collect(),
                            ),
                        );
                    }
                    if !target.candidates.is_empty() {
                        fields.insert(
                            "candidates".into(),
                            ModelFieldValue::List(
                                target
                                    .candidates
                                    .iter()
                                    .map(|p| {
                                        ModelFieldValue::Text(p.to_string_lossy().into_owned())
                                    })
                                    .collect(),
                            ),
                        );
                    }
                    ModelFieldValue::Object(fields)
                })
                .collect(),
        ),
    }
}

pub fn model_value_chars(value: &ModelFieldValue) -> usize {
    match value {
        ModelFieldValue::Text(value) => value.chars().count(),
        ModelFieldValue::List(values) => values.iter().map(model_value_chars).sum(),
        ModelFieldValue::Object(fields) => fields.values().map(model_value_chars).sum(),
        ModelFieldValue::Boolean(_) | ModelFieldValue::Absent => 0,
    }
}

pub fn project_model_view(raw: &BTreeMap<FieldName, FieldValue>, max_chars: usize) -> ModelView {
    let mut fields = raw
        .iter()
        .map(|(name, value)| (name.clone(), model_field(value)))
        .collect::<BTreeMap<_, _>>();
    let mut cut = BTreeSet::new();
    loop {
        let chars: usize = fields.values().map(model_value_chars).sum();
        if chars <= max_chars {
            return ModelView::Bounded {
                fields,
                cut: cut.into_iter().collect(),
            };
        }
        let mut candidates = Vec::new();
        for (field, value) in &fields {
            collect_strings(value, field, &mut Vec::new(), &mut candidates);
        }
        // Stable sorting keeps the first field and nested traversal order on ties.
        candidates.sort_by_key(|item| std::cmp::Reverse(item.2));
        let Some((field, path, length)) = candidates.into_iter().find(|(_, _, size)| *size > 200)
        else {
            break;
        };
        if let Some(ModelFieldValue::Text(text)) =
            fields.get_mut(&field).and_then(|v| value_at(v, &path))
        {
            let desired = length.saturating_sub(chars - max_chars).max(200);
            *text = cut_string(text, desired);
            cut.insert(field);
        }
    }
    // Each list is reduced as a single operation, preserving its prefix and one tail marker.
    loop {
        let chars: usize = fields.values().map(model_value_chars).sum();
        if chars <= max_chars {
            return ModelView::Bounded {
                fields,
                cut: cut.into_iter().collect(),
            };
        }
        let mut candidates = Vec::new();
        for (field, value) in &fields {
            collect_lists(value, field, &mut Vec::new(), &mut candidates);
        }
        candidates.sort_by_key(|item| std::cmp::Reverse(item.2));
        let Some((field, path, _)) = candidates.into_iter().find(|(_, _, size)| *size > 0) else {
            return ModelView::TooLarge {
                chars,
                limit: max_chars,
            };
        };
        if let Some(ModelFieldValue::List(values)) =
            fields.get_mut(&field).and_then(|v| value_at(v, &path))
        {
            let before = values.len();
            let mut removed = 0;
            let mut saved = 0usize;
            while let Some(value) = values.pop() {
                removed += 1;
                saved += model_value_chars(&value);
                if chars.saturating_sub(saved) + format!("[cut: {removed} more]").chars().count()
                    <= max_chars
                {
                    break;
                }
            }
            let marker = format!("[cut: {removed} more]");
            if saved <= marker.chars().count() {
                // No list can be reduced further without increasing the view.
                return ModelView::TooLarge {
                    chars,
                    limit: max_chars,
                };
            }
            debug_assert!(removed <= before);
            values.push(ModelFieldValue::Text(marker));
            cut.insert(field);
        }
    }
}

fn cut_string(text: &str, desired: usize) -> String {
    let prefix = (desired - 5).div_ceil(2);
    let suffix = desired - 5 - prefix;
    let tail: String = text
        .chars()
        .rev()
        .take(suffix)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!(
        "{}[cut]{tail}",
        text.chars().take(prefix).collect::<String>()
    )
}

#[derive(Clone)]
enum ValuePath {
    Field(String),
    Item(usize),
}
type ReductionCandidate = (FieldName, Vec<ValuePath>, usize);
fn collect_strings(
    value: &ModelFieldValue,
    field: &FieldName,
    path: &mut Vec<ValuePath>,
    output: &mut Vec<ReductionCandidate>,
) {
    match value {
        ModelFieldValue::Text(text) => {
            output.push((field.clone(), path.clone(), text.chars().count()))
        }
        ModelFieldValue::List(values) => {
            for (index, value) in values.iter().enumerate() {
                path.push(ValuePath::Item(index));
                collect_strings(value, field, path, output);
                path.pop();
            }
        }
        ModelFieldValue::Object(fields) => {
            for (name, value) in fields {
                path.push(ValuePath::Field(name.clone()));
                collect_strings(value, field, path, output);
                path.pop();
            }
        }
        ModelFieldValue::Boolean(_) | ModelFieldValue::Absent => {}
    }
}
fn collect_lists(
    value: &ModelFieldValue,
    field: &FieldName,
    path: &mut Vec<ValuePath>,
    output: &mut Vec<ReductionCandidate>,
) {
    match value {
        ModelFieldValue::List(values) => {
            // Already cut lists can still contain nested lists, but never replace their marker.
            if !values
                .last()
                .is_some_and(|v| matches!(v, ModelFieldValue::Text(s) if s.starts_with("[cut: ")))
                && values.iter().map(model_value_chars).sum::<usize>()
                    > format!("[cut: {} more]", values.len()).chars().count()
            {
                output.push((field.clone(), path.clone(), values.len()));
            }
            for (index, value) in values.iter().enumerate() {
                path.push(ValuePath::Item(index));
                collect_lists(value, field, path, output);
                path.pop();
            }
        }
        ModelFieldValue::Object(fields) => {
            for (name, value) in fields {
                path.push(ValuePath::Field(name.clone()));
                collect_lists(value, field, path, output);
                path.pop();
            }
        }
        ModelFieldValue::Text(_) | ModelFieldValue::Boolean(_) | ModelFieldValue::Absent => {}
    }
}
fn value_at<'a>(
    mut value: &'a mut ModelFieldValue,
    path: &[ValuePath],
) -> Option<&'a mut ModelFieldValue> {
    for step in path {
        value = match (step, value) {
            (ValuePath::Field(name), ModelFieldValue::Object(fields)) => fields.get_mut(name)?,
            (ValuePath::Item(index), ModelFieldValue::List(values)) => values.get_mut(*index)?,
            _ => return None,
        };
    }
    Some(value)
}

#[cfg(test)]
#[path = "evidence_model_view_test.rs"]
mod tests;

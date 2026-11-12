use crate::{
    EvidenceKindDefinition, EvidenceKindName, EvidenceLanguage, EvidenceProject, FieldName,
    FieldSource, FieldValue, KindMatcher, ModelView, project_model_view,
};
use std::collections::BTreeMap;

pub fn evidence_kind_language(
    project: &EvidenceProject,
    kind: &EvidenceKindName,
    definition: &EvidenceKindDefinition,
) -> Option<EvidenceLanguage> {
    match &definition.matcher {
        KindMatcher::HtmlUnits(_) => Some(EvidenceLanguage::Html),
        KindMatcher::MarkdownUnits(_) => Some(EvidenceLanguage::Markdown),
        KindMatcher::Named(_) | KindMatcher::Inline { .. } => definition
            .matcher
            .ast_rule_id(kind)
            .and_then(|rule| project.language_for_rule(&rule)),
    }
}

/// Project asking text before applying the common model-view budget; keep raw evidence intact.
pub fn project_kind_model_view(
    project: &EvidenceProject,
    kind: &EvidenceKindName,
    definition: &EvidenceKindDefinition,
    fields: &BTreeMap<FieldName, FieldValue>,
    max_chars: usize,
) -> ModelView {
    if fields.values().any(FieldValue::is_selection_pending) {
        return ModelView::SelectionPending;
    }
    let html = evidence_kind_language(project, kind, definition) == Some(EvidenceLanguage::Html);
    let projected = fields
        .iter()
        .map(|(name, value)| {
            let project_html = match definition.fields.get(name) {
                Some(FieldSource::Document { rule, .. }) => {
                    project.language_for_rule(rule) == Some(EvidenceLanguage::Html)
                }
                Some(FieldSource::Target { .. }) => true,
                _ => html,
            };
            (
                name.clone(),
                if project_html {
                    html_field_value(value)
                } else {
                    value.clone()
                },
            )
        })
        .collect();
    project_model_view(&projected, max_chars)
}

fn html_field_value(value: &FieldValue) -> FieldValue {
    use crate::document_evidence::html_content::{html_text_content, is_html_path};
    match value {
        FieldValue::Text { value, spans } => FieldValue::Text {
            value: html_text_content(value),
            spans: spans.clone(),
        },
        FieldValue::Texts { values, spans } => FieldValue::Texts {
            values: values
                .iter()
                .map(|value| html_text_content(value))
                .collect(),
            spans: spans.clone(),
        },
        FieldValue::Targets { values } => FieldValue::Targets {
            values: values
                .iter()
                .cloned()
                .map(|mut target| {
                    if is_html_path(&target.path) {
                        target.opening =
                            target.opening.as_ref().map(|text| html_text_content(text));
                        target.fields = target.fields.as_ref().map(|fields| {
                            fields
                                .iter()
                                .map(|(name, value)| (name.clone(), html_field_value(value)))
                                .collect()
                        });
                    }
                    target
                })
                .collect(),
        },
        FieldValue::Absent | FieldValue::Unavailable { .. } => value.clone(),
    }
}

#[cfg(test)]
#[path = "kind_model_view_test.rs"]
mod tests;

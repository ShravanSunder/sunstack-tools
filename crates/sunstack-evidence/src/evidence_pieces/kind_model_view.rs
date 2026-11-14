use crate::{
    EvidenceKindDefinition, EvidenceKindName, EvidenceLanguage, EvidenceProject, FieldName,
    FieldSource, FieldValue, KindMatcher, ModelView, project_model_view,
};
use std::collections::{BTreeMap, BTreeSet};

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
    let mut recovery_cut = BTreeSet::new();
    let projected = fields
        .iter()
        .map(|(name, value)| {
            let project_html = match definition.fields.get(name) {
                Some(
                    FieldSource::Section {
                        text: crate::SectionText::Heading,
                    }
                    | FieldSource::Enclosing {
                        text: crate::ContextText::Heading,
                        ..
                    },
                ) => false,
                Some(FieldSource::Document { rule, .. }) => {
                    project.language_for_rule(rule) == Some(EvidenceLanguage::Html)
                }
                Some(FieldSource::Target { .. }) => true,
                Some(FieldSource::ScopedInside { .. }) => false,
                _ => html,
            };
            let mut incomplete = false;
            let value = if project_html {
                html_field_value(value, &mut incomplete, &project.document_settings.html)
            } else {
                value.clone()
            };
            if incomplete {
                recovery_cut.insert(name.clone());
            }
            (name.clone(), value)
        })
        .collect();
    let mut view = project_model_view(&projected, max_chars);
    if let ModelView::Bounded { cut, .. } = &mut view {
        recovery_cut.extend(cut.iter().cloned());
        *cut = recovery_cut.into_iter().collect();
    }
    view
}

fn html_field_value(
    value: &FieldValue,
    incomplete: &mut bool,
    settings: &crate::HtmlEvidenceSettings,
) -> FieldValue {
    use crate::document_evidence::html_content::is_html_path;
    match value {
        FieldValue::Text { value, spans } => FieldValue::Text {
            value: project_html_asking_text(value, incomplete, settings),
            spans: spans.clone(),
        },
        FieldValue::Texts { values, spans } => FieldValue::Texts {
            values: values
                .iter()
                .map(|value| project_html_asking_text(value, incomplete, settings))
                .collect(),
            spans: spans.clone(),
        },
        FieldValue::Document {
            path,
            values,
            spans,
        } => FieldValue::Document {
            path: path.clone(),
            values: values
                .iter()
                .map(|value| project_html_asking_text(value, incomplete, settings))
                .collect(),
            spans: spans.clone(),
        },
        FieldValue::Targets { values } => FieldValue::Targets {
            values: values
                .iter()
                .cloned()
                .map(|mut target| {
                    if is_html_path(&target.path) {
                        target.opening = target
                            .opening
                            .as_ref()
                            .map(|text| project_html_asking_text(text, incomplete, settings));
                        target.fields = target.fields.as_ref().map(|fields| {
                            fields
                                .iter()
                                .map(|(name, value)| {
                                    (name.clone(), html_field_value(value, incomplete, settings))
                                })
                                .collect()
                        });
                    }
                    target
                })
                .collect(),
        },
        FieldValue::Absent
        | FieldValue::Unavailable { .. }
        | FieldValue::MissingDocument { .. } => value.clone(),
    }
}

fn project_html_asking_text(
    raw: &str,
    incomplete: &mut bool,
    settings: &crate::HtmlEvidenceSettings,
) -> String {
    use crate::document_evidence::html_content::{HtmlAskingText, html_asking_text};
    match html_asking_text(raw, settings) {
        HtmlAskingText::Complete(text) => text,
        HtmlAskingText::Incomplete { text, reason } => {
            *incomplete = true;
            format!("{text} {}", reason.cut_marker())
                .trim_start()
                .into()
        }
    }
}

#[cfg(test)]
#[path = "kind_model_view_test.rs"]
mod tests;

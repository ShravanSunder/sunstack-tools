use crate::{DecisionRoute, DecisionStep, ModelRequestDispatch, RulebookRule};
use std::collections::BTreeMap;
use sunstack_decisions::{
    ChoiceLabel, JudgmentQuestion, StructuredEntry, StructuredJson, StructuredJudgmentRequest,
};
use sunstack_evidence::{EvidencePiece, FieldName};

fn candidate_lines(piece: &EvidencePiece, field: &FieldName) -> BTreeMap<usize, String> {
    let Some(field) = piece.fields.get(field) else {
        return BTreeMap::new();
    };
    let mut lines = BTreeMap::new();
    let mut byte_start = piece.location.span.byte_start;
    for (position, text) in piece.text.split_inclusive('\n').enumerate() {
        let line = piece.location.first_line + position;
        if line > piece.location.last_line {
            break;
        }
        let byte_end = byte_start + text.len();
        if field.spans().iter().any(|span| {
            span.source == piece.location.span.source
                && span.byte_start < span.byte_end
                && span.byte_start < byte_end
                && byte_start < span.byte_end
        }) {
            lines.insert(line, text.trim_end_matches(['\r', '\n']).into());
        }
        byte_start = byte_end;
    }
    lines
}

pub(super) fn add_locators(
    rule: &RulebookRule,
    piece: &EvidencePiece,
    dispatcher: &ModelRequestDispatch,
    request: &mut StructuredJudgmentRequest,
) {
    for step in rule.decision.steps.values() {
        let routes: Vec<&DecisionRoute> = match step {
            DecisionStep::Mechanical { yes, no, .. } => vec![yes, no],
            DecisionStep::AskNoul { routes, .. }
            | DecisionStep::AskScore { routes, .. }
            | DecisionStep::Combine { routes, .. } => {
                vec![&routes.yes, &routes.no, &routes.uncertain]
            }
            DecisionStep::AskChoice {
                routes, uncertain, ..
            } => routes.values().chain(std::iter::once(uncertain)).collect(),
        };
        for route in routes {
            let DecisionRoute::Finding(finding) = route else {
                continue;
            };
            let (Some(locator), Some(field)) = (&finding.locate, &finding.at) else {
                continue;
            };
            let lines = candidate_lines(piece, field);
            if lines.is_empty() || lines.len() > dispatcher.settings.max_locate_lines {
                continue;
            }
            let criteria = lines
                .into_iter()
                .map(|(line, text)| {
                    ChoiceLabel::new(format!("L{line}"))
                        .ok()
                        .map(|label| (label, StructuredEntry::Text(text)))
                })
                .collect::<Option<BTreeMap<_, _>>>();
            let Some(criteria) = criteria else {
                continue;
            };
            let mut candidate = request.clone();
            candidate.questions.insert(
                locator.name.clone(),
                JudgmentQuestion::Choice {
                    instructions: StructuredEntry::Object(BTreeMap::from([
                        ("check".into(), StructuredJson::Text(field.as_str().into())),
                        ("task".into(), StructuredJson::Text(locator.task.clone())),
                    ])),
                    criteria,
                },
            );
            if dispatcher.assess_selection(&candidate).is_ok() {
                *request = candidate;
            }
        }
    }
}

#[cfg(test)]
mod tests;

use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

validated_name!(FieldPath, |value: &str| value.starts_with('/'));
validated_name!(OccurrenceId, |value: &str| !value.is_empty());

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceOccurrence {
    Record {
        record: PositionedRecordId,
        author: RecordAuthor,
        positions: Vec<SourcePosition>,
        matched_at: SourcePosition,
        field: FieldPath,
        span: TextSpan,
    },
}
impl SourceOccurrence {
    pub fn occurrence_identity(&self) -> EvidenceResult<OccurrenceId> {
        match self {
            Self::Record { record, .. } => OccurrenceId::new(record.as_str()),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MechanicalPass {
    pub predicate: RecordPredicate,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecordPredicate {
    Regex { field: FieldPath, pattern: String },
    Literal { field: FieldPath, text: String },
    Any { predicates: Vec<RecordPredicate> },
    All { predicates: Vec<RecordPredicate> },
}
#[derive(Clone, Debug)]
pub struct RecordMatch {
    pub occurrence: SourceOccurrence,
    pub record: PositionedRecord,
    pub witnesses: Vec<SourceOccurrence>,
}
#[derive(Clone, Debug, Default)]
pub struct Discovery {
    pub matches: Vec<RecordMatch>,
    pub rejected_sample: Vec<RecordMatch>,
}
pub trait CandidateSource {
    fn discover(
        &self,
        pass: Option<&MechanicalPass>,
        samples_per_pass: usize,
    ) -> EvidenceResult<Discovery>;
}
impl CandidateSource for [PositionedRecord] {
    fn discover(
        &self,
        pass: Option<&MechanicalPass>,
        samples_per_pass: usize,
    ) -> EvidenceResult<Discovery> {
        let predicate = pass
            .map(|pass| compile_predicate(&pass.predicate))
            .transpose()?;
        let mut discovery = Discovery::default();
        let mut dropped = Vec::new();
        for record in self {
            let value = serde_json::to_value(record).map_err(|_| EvidenceCode::InvalidSelector)?;
            let mut spans = match &predicate {
                Some(predicate) => matching_fields(predicate, &value)?,
                None => vec![(
                    FieldPath::new("/body")?,
                    TextSpan {
                        byte_start: 0,
                        byte_end: 0,
                    },
                )],
            };
            spans.sort_by(|(left_field, left_span), (right_field, right_span)| {
                left_field
                    .as_str()
                    .cmp(right_field.as_str())
                    .then(left_span.byte_start.cmp(&right_span.byte_start))
            });
            if spans.is_empty() {
                dropped.push(RecordMatch {
                    occurrence: occurrence(
                        record,
                        FieldPath::new("/body")?,
                        TextSpan {
                            byte_start: 0,
                            byte_end: 0,
                        },
                    )?,
                    record: record.clone(),
                    witnesses: vec![],
                });
            } else {
                let witnesses = spans
                    .into_iter()
                    .map(|(field, span)| occurrence(record, field, span))
                    .collect::<EvidenceResult<Vec<_>>>()?;
                if let Some(occurrence) = witnesses.first().cloned() {
                    discovery.matches.push(RecordMatch {
                        occurrence,
                        record: record.clone(),
                        witnesses,
                    });
                }
            }
        }
        let count = samples_per_pass.min(dropped.len());
        for sample in 0..count {
            discovery
                .rejected_sample
                .push(dropped[sample * dropped.len() / count].clone());
        }
        Ok(discovery)
    }
}
fn occurrence(
    record: &PositionedRecord,
    field: FieldPath,
    span: TextSpan,
) -> EvidenceResult<SourceOccurrence> {
    let matched_at = record
        .field_positions
        .iter()
        .filter(|(prefix, _)| {
            field.as_str() == prefix.as_str()
                || field
                    .as_str()
                    .strip_prefix(prefix.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|(prefix, _)| prefix.as_str().len())
        .map(|(_, position)| *position)
        .or_else(|| record.positions.first().copied())
        .ok_or(EvidenceCode::InvalidSelector)?;
    Ok(SourceOccurrence::Record {
        record: record.record_id.clone(),
        author: record.author,
        positions: record.positions.clone(),
        matched_at,
        field,
        span,
    })
}
enum CompiledRecordPredicate<'predicate> {
    Regex {
        field: &'predicate FieldPath,
        matcher: regex::Regex,
    },
    Literal {
        field: &'predicate FieldPath,
        text: &'predicate str,
    },
    Any {
        predicates: Vec<CompiledRecordPredicate<'predicate>>,
    },
    All {
        predicates: Vec<CompiledRecordPredicate<'predicate>>,
    },
}
fn compile_predicate(predicate: &RecordPredicate) -> EvidenceResult<CompiledRecordPredicate<'_>> {
    Ok(match predicate {
        RecordPredicate::Regex { field, pattern } => CompiledRecordPredicate::Regex {
            field,
            matcher: regex::Regex::new(pattern).map_err(|_| EvidenceCode::InvalidSelector)?,
        },
        RecordPredicate::Literal { field, text } => {
            if text.is_empty() {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            CompiledRecordPredicate::Literal { field, text }
        }
        RecordPredicate::Any { predicates } | RecordPredicate::All { predicates } => {
            if predicates.is_empty() {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            let compiled = predicates
                .iter()
                .map(compile_predicate)
                .collect::<EvidenceResult<Vec<_>>>()?;
            if matches!(predicate, RecordPredicate::Any { .. }) {
                CompiledRecordPredicate::Any {
                    predicates: compiled,
                }
            } else {
                CompiledRecordPredicate::All {
                    predicates: compiled,
                }
            }
        }
    })
}
fn matching_fields(
    predicate: &CompiledRecordPredicate<'_>,
    value: &serde_json::Value,
) -> EvidenceResult<Vec<(FieldPath, TextSpan)>> {
    match predicate {
        CompiledRecordPredicate::Regex { field, matcher } => Ok(value
            .pointer(field.as_str())
            .and_then(serde_json::Value::as_str)
            .map(|text| {
                matcher
                    .find_iter(text)
                    .map(|matched| {
                        (
                            (*field).clone(),
                            TextSpan {
                                byte_start: matched.start(),
                                byte_end: matched.end(),
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()),
        CompiledRecordPredicate::Literal { field, text } => Ok(value
            .pointer(field.as_str())
            .and_then(serde_json::Value::as_str)
            .map(|field_text| {
                field_text
                    .match_indices(*text)
                    .map(|(start, _)| {
                        (
                            (*field).clone(),
                            TextSpan {
                                byte_start: start,
                                byte_end: start + text.len(),
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()),
        CompiledRecordPredicate::Any { predicates }
        | CompiledRecordPredicate::All { predicates } => {
            let mut spans = Vec::new();
            for child in predicates {
                let matches = matching_fields(child, value)?;
                if matches.is_empty() && matches!(predicate, CompiledRecordPredicate::All { .. }) {
                    return Ok(vec![]);
                }
                for matched in matches {
                    if !spans.contains(&matched) {
                        spans.push(matched);
                    }
                }
            }
            Ok(spans)
        }
    }
}

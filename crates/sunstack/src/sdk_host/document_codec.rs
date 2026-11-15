//! Pure, revision-bound SDK document decoding. No admission or execution occurs here.
use super::{
    inspection_projection::{InspectionOutcome, InspectionReport, InspectionStatus},
    report_projection::{RunOutcome, SdkReport},
    sdk_contracts::{
        CONTRACT_REVISION, CheckOutcome, Configuration, LoadOutcome, LoadedConfiguration,
        SdkFailure,
    },
};
use serde::{Deserialize, Serialize, de};
use serde_json::value::RawValue;
use std::{collections::BTreeMap, fmt};
use sunstack_evidence::FileCount;
use sunstack_lint::{LintStatus, SelectionEntry, UnavailableCounts};

/// The SDK owns framing; callers own any file containing this document.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct SdkDocument<TPayload> {
    pub contract_revision: u32,
    pub payload: TPayload,
}

/// Explicit root selection disambiguates overlapping operation outcome tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[allow(clippy::large_enum_variant)]
pub enum SdkDocumentValue {
    Configuration(Configuration),
    LoadedConfiguration(LoadedConfiguration),
    LoadOutcome(LoadOutcome),
    CheckOutcome(CheckOutcome),
    RunOutcome(RunOutcome),
    InspectionOutcome(InspectionOutcome),
    Report(SdkReport<LintStatus>),
    InspectionReport(InspectionReport<InspectionStatus>),
}

/// Original document text crosses as data in an exact JSON string transport.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct DeserializeSdkRequest {
    pub json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct SdkDocumentDiagnostic {
    pub json_path: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum DocumentInvalidReason {
    MalformedJson,
    InvalidDocument,
    InvalidValue,
    UnsupportedNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[allow(clippy::large_enum_variant)]
pub enum DeserializationOutcome {
    Decoded {
        value: SdkDocumentValue,
    },
    Incompatible {
        expected_contract_revision: u32,
        actual_contract_revision: u32,
    },
    Invalid {
        reason: DocumentInvalidReason,
        diagnostics: Vec<SdkDocumentDiagnostic>,
    },
    Failed {
        failure: SdkFailure,
    },
}

fn invalid(
    reason: DocumentInvalidReason,
    diagnostic: SdkDocumentDiagnostic,
) -> DeserializationOutcome {
    DeserializationOutcome::Invalid {
        reason,
        diagnostics: vec![diagnostic],
    }
}

fn diagnostic(error: impl fmt::Display) -> SdkDocumentDiagnostic {
    SdkDocumentDiagnostic {
        json_path: vec![],
        message: error.to_string(),
    }
}

fn decode_at_path<'text, TValue: Deserialize<'text>>(
    text: &'text str,
) -> Result<TValue, SdkDocumentDiagnostic> {
    let mut decoder = serde_json::Deserializer::from_str(text);
    let value =
        serde_path_to_error::deserialize(&mut decoder).map_err(|error| SdkDocumentDiagnostic {
            json_path: error
                .path()
                .iter()
                .filter_map(|segment| match segment {
                    serde_path_to_error::Segment::Map { key } => Some(key.clone()),
                    serde_path_to_error::Segment::Seq { index } => Some(index.to_string()),
                    serde_path_to_error::Segment::Enum { variant } => Some(variant.clone()),
                    serde_path_to_error::Segment::Unknown => None,
                })
                .collect(),
            message: error.inner().to_string(),
        })?;
    decoder.end().map_err(diagnostic)?;
    Ok(value)
}

// Traverse every original token, including duplicate map keys. This numeric view is
// never the input to typed decoding, so it cannot erase a typed duplicate refusal.
struct SupportedNumbers;

impl<'de> Deserialize<'de> for SupportedNumbers {
    fn deserialize<TDecoder: de::Deserializer<'de>>(
        decoder: TDecoder,
    ) -> Result<Self, TDecoder::Error> {
        struct NumberVisitor;
        impl<'de> de::Visitor<'de> for NumberVisitor {
            type Value = SupportedNumbers;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .write_str("JSON data with exact i64/u64 integers and supported finite floats")
            }
            fn visit_bool<TError: de::Error>(self, _: bool) -> Result<Self::Value, TError> {
                Ok(SupportedNumbers)
            }
            fn visit_unit<TError: de::Error>(self) -> Result<Self::Value, TError> {
                Ok(SupportedNumbers)
            }
            fn visit_str<TError: de::Error>(self, _: &str) -> Result<Self::Value, TError> {
                Ok(SupportedNumbers)
            }
            fn visit_i64<TError: de::Error>(self, _: i64) -> Result<Self::Value, TError> {
                Ok(SupportedNumbers)
            }
            fn visit_u64<TError: de::Error>(self, _: u64) -> Result<Self::Value, TError> {
                Ok(SupportedNumbers)
            }
            fn visit_f64<TError: de::Error>(self, number: f64) -> Result<Self::Value, TError> {
                if !number.is_finite()
                    || (number.fract() == 0.0 && number.abs() > 9_007_199_254_740_991.0)
                {
                    return Err(TError::custom(
                        "unsupported number: unsafe integral float or integer outside i64/u64",
                    ));
                }
                Ok(SupportedNumbers)
            }
            fn visit_seq<TSequence: de::SeqAccess<'de>>(
                self,
                mut sequence: TSequence,
            ) -> Result<Self::Value, TSequence::Error> {
                while sequence.next_element::<SupportedNumbers>()?.is_some() {}
                Ok(SupportedNumbers)
            }
            fn visit_map<TMap: de::MapAccess<'de>>(
                self,
                mut map: TMap,
            ) -> Result<Self::Value, TMap::Error> {
                while map.next_key::<String>()?.is_some() {
                    map.next_value::<SupportedNumbers>()?;
                }
                Ok(SupportedNumbers)
            }
        }
        decoder.deserialize_any(NumberVisitor)
    }
}

fn validate_report_counts<TStatus>(
    report: &SdkReport<TStatus>,
    json_path: Vec<String>,
) -> Result<(), SdkDocumentDiagnostic> {
    let canonical = UnavailableCounts::from_facts(
        &report.results,
        &report.filter_answers,
        &report.selections,
        &report.incomplete,
    );
    if report.unavailable_counts != canonical {
        let mut json_path = json_path;
        json_path.push("unavailable_counts".into());
        return Err(SdkDocumentDiagnostic {
            json_path,
            message: "unavailable counts disagree with retained assessment and coverage facts"
                .into(),
        });
    }
    validate_selection_counts(&report.file_counts, &report.selections, &json_path)?;
    if let Some(listing) = &report.evidence_listing {
        let mut listing_path = json_path;
        listing_path.push("evidence_listing".into());
        validate_selection_counts(&listing.file_counts, &listing.selections, &listing_path)?;
    }
    Ok(())
}

fn validate_selection_counts(
    file_counts: &[FileCount],
    selections: &[SelectionEntry],
    json_path: &[String],
) -> Result<(), SdkDocumentDiagnostic> {
    let mut unavailable_by_scope = BTreeMap::new();
    for entry in selections {
        if matches!(
            entry.outcome,
            sunstack_rules::SelectionOutcome::Unavailable { .. }
        ) {
            *unavailable_by_scope
                .entry((&entry.location.file, &entry.kind))
                .or_insert(0usize) += 1;
        }
    }
    for (file_index, file_count) in file_counts.iter().enumerate() {
        for (kind, kind_count) in &file_count.kinds {
            let Some(selection_count) = &kind_count.selection else {
                continue;
            };
            let canonical = unavailable_by_scope
                .get(&(&file_count.file, kind))
                .copied()
                .unwrap_or(0);
            if selection_count.unavailable != canonical {
                let mut json_path = json_path.to_vec();
                json_path.extend([
                    "file_counts".into(),
                    file_index.to_string(),
                    "kinds".into(),
                    kind.as_str().to_owned(),
                    "selection".into(),
                    "unavailable".into(),
                ]);
                return Err(SdkDocumentDiagnostic {
                    json_path,
                    message:
                        "unavailable selection count disagrees with retained kind/file entries"
                            .into(),
                });
            }
        }
    }
    Ok(())
}

fn validate_inspection_counts<TStatus>(
    report: &InspectionReport<TStatus>,
    json_path: Vec<String>,
) -> Result<(), SdkDocumentDiagnostic> {
    validate_selection_counts(&report.file_counts, &report.selections, &json_path)
}

fn validate_document_counts(value: &SdkDocumentValue) -> Result<(), SdkDocumentDiagnostic> {
    let report_path = || vec!["payload".into(), "value".into(), "report".into()];
    match value {
        SdkDocumentValue::Report(report) => {
            validate_report_counts(report, vec!["payload".into(), "value".into()])
        }
        SdkDocumentValue::RunOutcome(outcome) => match outcome {
            RunOutcome::Completed { report } => validate_report_counts(report, report_path()),
            RunOutcome::Incomplete { report } => validate_report_counts(report, report_path()),
            RunOutcome::Invalid { report, .. } => report.as_ref().map_or(Ok(()), |report| {
                validate_report_counts(report, report_path())
            }),
            RunOutcome::Cancelled { report } => report.as_ref().map_or(Ok(()), |report| {
                validate_report_counts(report, report_path())
            }),
            RunOutcome::Failed { report, .. } => report.as_ref().map_or(Ok(()), |report| {
                validate_report_counts(report, report_path())
            }),
            RunOutcome::Unavailable { .. } => Ok(()),
        },
        SdkDocumentValue::InspectionReport(report) => {
            validate_inspection_counts(report, vec!["payload".into(), "value".into()])
        }
        SdkDocumentValue::InspectionOutcome(outcome) => match outcome {
            InspectionOutcome::Inspected { report } => {
                validate_inspection_counts(report, report_path())
            }
            InspectionOutcome::Incomplete { report } => {
                validate_inspection_counts(report, report_path())
            }
            InspectionOutcome::Cancelled { report } => report.as_ref().map_or(Ok(()), |report| {
                validate_inspection_counts(report, report_path())
            }),
            InspectionOutcome::Failed { report, .. } => report.as_ref().map_or(Ok(()), |report| {
                validate_inspection_counts(report, report_path())
            }),
            InspectionOutcome::Invalid { .. } | InspectionOutcome::Unavailable { .. } => Ok(()),
        },
        SdkDocumentValue::Configuration(_)
        | SdkDocumentValue::LoadedConfiguration(_)
        | SdkDocumentValue::LoadOutcome(_)
        | SdkDocumentValue::CheckOutcome(_) => Ok(()),
    }
}

/// Restore a known SDK value under the current revision, with no file or engine work.
pub fn deserialize_document(text: &str) -> DeserializationOutcome {
    // RawValue checks syntax without interpreting or rounding any number.
    if let Err(error) = serde_json::from_str::<Box<RawValue>>(text) {
        return invalid(DocumentInvalidReason::MalformedJson, diagnostic(error));
    }
    let document = match decode_at_path::<SdkDocument<Box<RawValue>>>(text) {
        Ok(document) => document,
        Err(error) => return invalid(DocumentInvalidReason::InvalidDocument, error),
    };
    if document.contract_revision != CONTRACT_REVISION {
        return DeserializationOutcome::Incompatible {
            expected_contract_revision: CONTRACT_REVISION,
            actual_contract_revision: document.contract_revision,
        };
    }
    let payload = document.payload.get();
    if let Err(mut error) = decode_at_path::<SupportedNumbers>(payload) {
        error.json_path.insert(0, "payload".into());
        return invalid(DocumentInvalidReason::UnsupportedNumber, error);
    }
    let value = match decode_at_path::<SdkDocumentValue>(payload) {
        Ok(value) => value,
        Err(mut error) => {
            error.json_path.insert(0, "payload".into());
            return invalid(DocumentInvalidReason::InvalidValue, error);
        }
    };
    if let Err(error) = validate_document_counts(&value) {
        return invalid(DocumentInvalidReason::InvalidValue, error);
    }
    // Typed normalization may coerce an exact integer token into f64. Check its
    // resulting numeric representation too, before returning any typed value.
    let normalized = match serde_json::to_string(&value) {
        Ok(normalized) => normalized,
        Err(error) => return invalid(DocumentInvalidReason::InvalidValue, diagnostic(error)),
    };
    if let Err(mut error) = decode_at_path::<SupportedNumbers>(&normalized) {
        error.json_path.insert(0, "payload".into());
        return invalid(DocumentInvalidReason::UnsupportedNumber, error);
    }
    DeserializationOutcome::Decoded { value }
}

#[cfg(test)]
#[path = "document_codec_test.rs"]
mod tests;

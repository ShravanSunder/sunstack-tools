use crate::{RequestOwner, SavedRequest, SavedRequestOutcome};
use serde::Deserialize;
use std::path::Path;
use sunstack_rules::RecordedSelection;

#[derive(Deserialize)]
struct RequestLedger {
    requests: Vec<SavedRequest>,
}
#[derive(Deserialize)]
struct RevisionLedger {
    revision: String,
    report: RequestLedger,
}
#[derive(Deserialize)]
struct CaseLedger {
    runs: Vec<RevisionLedger>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum SourceDocument {
    Lint(RequestLedger),
    Cases(CaseLedger),
}

/// Reads only the request ledger; checked text and other saved-report sections are not replay inputs.
pub(crate) struct SelectionSource {
    document: SourceDocument,
}
impl SelectionSource {
    pub(crate) fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
        let mut document: SourceDocument =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        let keep = |ledger: &mut RequestLedger| {
            ledger
                .requests
                .retain(|request| matches!(request.owner, RequestOwner::Selection { .. }))
        };
        match &mut document {
            SourceDocument::Lint(ledger) => keep(ledger),
            SourceDocument::Cases(cases) => {
                for run in &mut cases.runs {
                    keep(&mut run.report);
                }
            }
        }
        Ok(Self { document })
    }
    fn records(ledger: &RequestLedger, revision: Option<&str>) -> Vec<RecordedSelection> {
        ledger
            .requests
            .iter()
            .filter_map(|record| {
                let RequestOwner::Selection { selection_kind, .. } = &record.owner else {
                    return None;
                };
                let outcome = match &record.outcome {
                    SavedRequestOutcome::Run { outcome }
                    | SavedRequestOutcome::FromReport { outcome, .. } => outcome,
                    SavedRequestOutcome::NotSent { .. } | SavedRequestOutcome::Prepared { .. } => {
                        return None;
                    }
                };
                Some(RecordedSelection {
                    source_request_id: record.id,
                    source_revision: revision.map(str::to_owned),
                    selection_kind: selection_kind.clone(),
                    request: record.request.clone(),
                    input_sha256: record.input_sha256.clone()?,
                    outcome: outcome.clone(),
                })
            })
            .collect()
    }
    pub(crate) fn all_records(&self) -> Vec<RecordedSelection> {
        match &self.document {
            SourceDocument::Lint(ledger) => Self::records(ledger, None),
            SourceDocument::Cases(cases) => cases
                .runs
                .iter()
                .flat_map(|run| Self::records(&run.report, Some(&run.revision)))
                .collect(),
        }
    }
}

pub fn read_selection_records(path: &Path) -> Result<Vec<RecordedSelection>, String> {
    SelectionSource::read(path).map(|source| source.all_records())
}

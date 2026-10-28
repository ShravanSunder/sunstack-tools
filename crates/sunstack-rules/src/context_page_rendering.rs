//! Readable context text; full machine identities remain in the bundle.
use crate::DiagnosticList;
use crate::*;
use serde::Serialize;
use std::collections::BTreeMap;
use sunstack_evidence::*;

pub(super) struct PieceNames(BTreeMap<PieceId, String>);
impl PieceNames {
    pub fn new<'context>(pieces: impl Iterator<Item = &'context PieceId>) -> Self {
        let pieces = pieces.cloned().collect::<Vec<_>>();
        Self(
            pieces
                .iter()
                .map(|piece| {
                    let text = piece.as_str();
                    let chars = text.chars().collect::<Vec<_>>();
                    let abbreviated = (8..chars.len()).find_map(|length| {
                        let suffix = chars[chars.len() - length..].iter().collect::<String>();
                        pieces
                            .iter()
                            .filter(|other| *other != piece)
                            .all(|other| !other.as_str().ends_with(&suffix))
                            .then(|| format!("…{suffix}"))
                    });
                    (
                        piece.clone(),
                        abbreviated.unwrap_or_else(|| text.to_owned()),
                    )
                })
                .collect(),
        )
    }
    pub fn get<'piece>(&'piece self, piece: &'piece PieceId) -> &'piece str {
        self.0
            .get(piece)
            .map(String::as_str)
            .unwrap_or_else(|| piece.as_str())
    }
}
pub(super) fn side(revision: &SourceRevision) -> &'static str {
    let (SourceRevision::Commit { side, .. }
    | SourceRevision::Index { side, .. }
    | SourceRevision::Worktree { side, .. }) = revision;
    match side {
        SourceSide::Before => "before",
        SourceSide::After => "after",
    }
}
pub(super) fn source_anchor(span: &SourceSpan) -> String {
    let source = match &span.source {
        SourceRef::RepositoryFile { path, revision, .. } => {
            format!("{} [{}]", path.display(), side(revision))
        }
        SourceRef::Document { path, .. } | SourceRef::Transcript { path, .. } => {
            path.display().to_string()
        }
        SourceRef::Payload { input, .. } => format!("payload {}", input.as_str()),
    };
    format!(
        "{source}:{}:{}-{}:{}",
        span.start_line, span.start_column, span.end_line, span.end_column
    )
}
fn key_anchor(context: &CandidateContext, key: &SourceUnitKey) -> String {
    context
        .units
        .iter()
        .find(|unit| unit.unit.key == *key)
        .map_or_else(
            || {
                format!(
                    "{} [{}] bytes {}..{} (lines not captured)",
                    key.path.display(),
                    side(&key.revision),
                    key.byte_start,
                    key.byte_end
                )
            },
            |unit| source_anchor(&unit.unit.span),
        )
}
fn name(value: &impl Serialize) -> Result<String, DiagnosticList> {
    serde_json::to_string(value)
        .map(|text| text.trim_matches('"').to_owned())
        .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))
}
fn fact_kind(fact: &RelationshipFact) -> &'static str {
    match fact {
        RelationshipFact::Declaration { .. } => "declaration",
        RelationshipFact::ImportBinding { .. } => "import_binding",
        RelationshipFact::StaticCall { .. } => "static_call",
        RelationshipFact::PackageMapping { .. } => "package_mapping",
    }
}
pub(super) fn relation_anchor(
    context: &CandidateContext,
    relation: &RelatedPiece,
) -> Result<String, DiagnosticList> {
    Ok(match relation {
        RelatedPiece::Resolved {
            connection,
            seed,
            unit,
            facts,
            ..
        } => format!(
            "{}: {} -> {}; facts: {}",
            name(connection)?,
            key_anchor(context, seed),
            key_anchor(context, unit),
            facts.iter().map(fact_kind).collect::<Vec<_>>().join(", "),
        ),
        RelatedPiece::Unresolved {
            connection,
            name_as_written,
            call_site,
            reason,
            ..
        } => format!(
            "{}: {name_as_written}; call_site {}; {} (source not found)",
            name(connection)?,
            source_anchor(call_site),
            name(reason)?,
        ),
    })
}
pub(super) fn label(
    context: &CandidateContext,
    label: &RelatednessLabel,
    names: &PieceNames,
) -> Result<String, DiagnosticList> {
    Ok(match label {
        RelatednessLabel::Related { basis } => match basis {
            RelatedBasis::InputUnit { unit } => {
                format!("related — input unit {}", key_anchor(context, unit))
            }
            RelatedBasis::InputSpan { span } => {
                format!("related — input span {}", source_anchor(span))
            }
            RelatedBasis::Judged { answer } => format!("related — judged by {}", answer.0.as_str()),
        },
        RelatednessLabel::ProbablyRelated { fact } => format!(
            "probably_related — graph fact {}#{}",
            names.get(&fact.piece_id),
            fact.position
        ),
        RelatednessLabel::Uncertain { reasons } => format!(
            "uncertain — {}",
            reasons
                .iter()
                .map(|reason| match reason {
                    UncertaintyReason::Graph { reason } => name(reason),
                    UncertaintyReason::Contradicted { reason } => name(reason),
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        RelatednessLabel::Unrelated { judged_by } => {
            format!("unrelated — judged by {}", judged_by.0.as_str())
        }
    })
}
pub(super) fn revision(revision: &SourceRevision) -> String {
    match revision {
        SourceRevision::Commit { commit_id, .. } => {
            format!("commit {} {}", commit_id.as_str(), side(revision))
        }
        SourceRevision::Index {
            content_identity, ..
        } => format!("index {content_identity} {}", side(revision)),
        SourceRevision::Worktree { snapshot, .. } => {
            format!("worktree {} {}", snapshot.as_str(), side(revision))
        }
    }
}
pub(super) fn part(part: &CodeTextPart) -> &'static str {
    match part {
        CodeTextPart::Match => "match",
        CodeTextPart::Hunk { .. } => "diff",
        CodeTextPart::UnitText { .. } | CodeTextPart::UnitSignature { .. } => "code",
        CodeTextPart::Import { .. } => "import",
        CodeTextPart::Hint { .. } => "relationship",
    }
}

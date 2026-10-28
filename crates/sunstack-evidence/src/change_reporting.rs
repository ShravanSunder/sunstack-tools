//! The public change-selection account; frozen bytes and verification state stay native.
use crate::*;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

validated_name!(HunkId, |value: &str| !value.is_empty());
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodeExcerpt {
    pub span: SourceSpan,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeHunk {
    pub hunk_id: HunkId,
    pub path: std::path::PathBuf,
    pub before: Option<CodeExcerpt>,
    pub after: Option<CodeExcerpt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedChangeSelection {
    pub repository: RepositoryId,
    pub anchor_utc: DateTime<Utc>,
    pub members: Vec<ResolvedChangeMember>,
    pub coverage: DiscoveryCoverage,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedChangeMember {
    pub scope_position: usize,
    pub bounds: ResolvedChangeBounds,
    pub changed_paths: Vec<ResolvedChangedPath>,
    pub recency: ChangeRecencyBasis,
    pub diagnostics: DiagnosticList,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResolvedChangeBounds {
    Full,
    Uncommitted {
        layers: Vec<DirtyLayer>,
    },
    RevisionDiff {
        base_revision: GitObjectId,
        head: SourceRevision,
    },
    CommitSet {
        commit_ids: Vec<GitObjectId>,
        parent_policy: CommitParentPolicy,
    },
    RecentCommits {
        reference: String,
        commit_ids: Vec<GitObjectId>,
        parent_policy: CommitParentPolicy,
        since_utc: DateTime<Utc>,
        until_utc: DateTime<Utc>,
    },
    RecentUncommitted {
        layers: Vec<DirtyLayer>,
        since_utc: DateTime<Utc>,
        until_utc: DateTime<Utc>,
        unknown_recency: UnknownRecencyPolicy,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedChangedPath {
    pub path: PathBuf,
    pub provenance: ChangeMemberBasis,
    pub before: Option<SourceRef>,
    pub after: Option<SourceRef>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeRecencyBasis {
    NotTimeBased,
    CommitTime,
    ApproximateFileTime { unknown: UnknownFileRecency },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnknownFileRecency {
    None,
    Included { paths: Vec<PathBuf> },
    Excluded { count: usize },
}

pub fn report_change_selection(
    capture: &CapturedChangeSelection,
) -> EvidenceResult<ResolvedChangeSelection> {
    let scopes = match &capture.scope {
        ChangeScope::Union { members } => members.as_slice(),
        scope => std::slice::from_ref(scope),
    };
    let mut members = Vec::new();
    for (scope_position, scope) in scopes.iter().enumerate() {
        let observations = capture
            .members
            .iter()
            .filter(|member| member.scope_position == scope_position)
            .collect::<Vec<_>>();
        let bounds = resolved_bounds(scope, &observations, capture.anchor_utc)?;
        let changed_paths = observations
            .iter()
            .flat_map(|member| {
                member.changes.iter().map(|change| ResolvedChangedPath {
                    path: change.path.clone(),
                    provenance: member.basis.clone(),
                    before: side_source(&change.before),
                    after: side_source(&change.after),
                })
            })
            .collect();
        let mut diagnostics = DiagnosticList::default();
        let recency = match scope {
            ChangeScope::RecentCommits { .. } => ChangeRecencyBasis::CommitTime,
            ChangeScope::RecentUncommitted {
                unknown_recency, ..
            } => {
                let included = observations
                    .iter()
                    .flat_map(|member| match &member.recency {
                        CapturedRecencyBasis::ApproximateFileMtime {
                            included_unknown, ..
                        } => included_unknown.clone(),
                        _ => vec![],
                    })
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let excluded = observations
                    .iter()
                    .map(|member| match &member.recency {
                        CapturedRecencyBasis::ApproximateFileMtime {
                            excluded_unknown_paths,
                            ..
                        } => excluded_unknown_paths.len(),
                        _ => 0,
                    })
                    .sum();
                let unknown = match unknown_recency {
                    UnknownRecencyPolicy::IncludeWithDiagnostic if !included.is_empty() => {
                        diagnostics.push(Diagnostic::new(EvidenceCode::RecencyUnknown));
                        UnknownFileRecency::Included { paths: included }
                    }
                    UnknownRecencyPolicy::ExcludeWithOmission if excluded > 0 => {
                        UnknownFileRecency::Excluded { count: excluded }
                    }
                    _ => UnknownFileRecency::None,
                };
                ChangeRecencyBasis::ApproximateFileTime { unknown }
            }
            _ => ChangeRecencyBasis::NotTimeBased,
        };
        for diagnostic in capture
            .scope_diagnostics
            .get(&scope_position)
            .into_iter()
            .flat_map(|diagnostics| &diagnostics.items)
        {
            if diagnostic.code == EvidenceCode::HistoryIncomplete {
                diagnostics.push(diagnostic.clone());
            }
        }
        members.push(ResolvedChangeMember {
            scope_position,
            bounds,
            changed_paths,
            recency,
            diagnostics,
        });
    }
    Ok(ResolvedChangeSelection {
        repository: capture.repository.clone(),
        anchor_utc: capture.anchor_utc,
        members,
        coverage: capture.coverage.clone(),
    })
}
fn side_source(side: &CapturedFileSide) -> Option<SourceRef> {
    match side {
        CapturedFileSide::Present { file } => Some(file.source.clone()),
        CapturedFileSide::Absent => None,
    }
}
fn resolved_bounds(
    scope: &ChangeScope,
    observations: &[&CapturedChangeMember],
    anchor: DateTime<Utc>,
) -> EvidenceResult<ResolvedChangeBounds> {
    Ok(match scope {
        ChangeScope::Full {} => ResolvedChangeBounds::Full,
        ChangeScope::Uncommitted { layers } => ResolvedChangeBounds::Uncommitted {
            layers: layers.clone(),
        },
        ChangeScope::RevisionDiff { .. } => {
            let Some(ChangeMemberBasis::RevisionDiff { base, head }) =
                observations.first().map(|member| &member.basis)
            else {
                return Err(EvidenceCode::RevisionUnavailable.into());
            };
            ResolvedChangeBounds::RevisionDiff {
                base_revision: base.clone(),
                head: head.clone(),
            }
        }
        ChangeScope::CommitSet {
            commit_ids,
            parent_policy,
        } => ResolvedChangeBounds::CommitSet {
            commit_ids: commit_ids.clone(),
            parent_policy: *parent_policy,
        },
        ChangeScope::RecentCommits {
            reference,
            window,
            parent_policy,
        } => {
            let window = resolve_time_window(window, anchor)?;
            let mut commit_ids = Vec::new();
            for member in observations {
                if let ChangeMemberBasis::Commit { commit_id, .. } = &member.basis
                    && !commit_ids.contains(commit_id)
                {
                    commit_ids.push(commit_id.clone());
                }
            }
            ResolvedChangeBounds::RecentCommits {
                reference: reference.clone(),
                commit_ids,
                parent_policy: *parent_policy,
                since_utc: window.since_utc,
                until_utc: window.until_utc,
            }
        }
        ChangeScope::RecentUncommitted {
            layers,
            window,
            unknown_recency,
        } => {
            let window = resolve_time_window(window, anchor)?;
            ResolvedChangeBounds::RecentUncommitted {
                layers: layers.clone(),
                since_utc: window.since_utc,
                until_utc: window.until_utc,
                unknown_recency: *unknown_recency,
            }
        }
        ChangeScope::Union { .. } => return Err(EvidenceCode::InvalidRepositoryBinding.into()),
    })
}

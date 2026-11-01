use crate::evidence_planning::evidence_contracts::validated_name;
use crate::*;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroUsize, path::PathBuf};
validated_name!(SnapshotId, |value: &str| !value.is_empty());
validated_name!(GitObjectId, |value: &str| matches!(value.len(), 40 | 64)
    && value.bytes().all(|byte| byte.is_ascii_hexdigit()));
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DirtyLayer {
    Staged,
    Unstaged,
    Untracked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommitParentPolicy {
    FirstParent,
    EachParent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnknownRecencyPolicy {
    IncludeWithDiagnostic,
    ExcludeWithOmission,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TimeWindow {
    Lookback {
        seconds: NonZeroUsize,
    },
    Absolute {
        since_utc: DateTime<Utc>,
        until_utc: DateTime<Utc>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedTimeWindow {
    pub anchor_utc: DateTime<Utc>,
    pub since_utc: DateTime<Utc>,
    pub until_utc: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RevisionHead {
    Commit { revision: String },
    Worktree {},
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeScope {
    Full {},
    Uncommitted {
        layers: Vec<DirtyLayer>,
    },
    RevisionDiff {
        base_revision: String,
        head: RevisionHead,
    },
    CommitSet {
        commit_ids: Vec<GitObjectId>,
        parent_policy: CommitParentPolicy,
    },
    RecentCommits {
        reference: String,
        window: TimeWindow,
        parent_policy: CommitParentPolicy,
    },
    RecentUncommitted {
        layers: Vec<DirtyLayer>,
        window: TimeWindow,
        unknown_recency: UnknownRecencyPolicy,
    },
    Union {
        members: Vec<ChangeScope>,
    },
}
impl ChangeScope {
    pub fn has_change_selection(&self) -> bool {
        match self {
            Self::Full {} => false,
            Self::Union { members } => members.iter().any(Self::has_change_selection),
            _ => true,
        }
    }
    pub fn includes_full_scope(&self) -> bool {
        match self {
            Self::Full {} => true,
            Self::Union { members } => members.iter().any(Self::includes_full_scope),
            _ => false,
        }
    }
    pub fn validate(&self) -> EvidenceResult<()> {
        if let Self::RecentCommits { window, .. } | Self::RecentUncommitted { window, .. } = self
            && let TimeWindow::Absolute {
                since_utc,
                until_utc,
            } = window
            && since_utc > until_utc
        {
            return Err(EvidenceCode::InvalidRepositoryBinding.into());
        }
        match self {
            Self::Full {} => Ok(()),
            Self::Uncommitted { layers } | Self::RecentUncommitted { layers, .. } => {
                if layers.is_empty()
                    || layers
                        .iter()
                        .enumerate()
                        .any(|(index, layer)| layers[..index].contains(layer))
                {
                    Err(EvidenceCode::InvalidRepositoryBinding.into())
                } else {
                    Ok(())
                }
            }
            Self::CommitSet { commit_ids, .. } if commit_ids.is_empty() => {
                Err(EvidenceCode::InvalidRepositoryBinding.into())
            }
            Self::RevisionDiff {
                base_revision,
                head,
            } if base_revision.trim().is_empty()
                || matches!(head, RevisionHead::Commit { revision } if revision.trim().is_empty()) =>
            {
                Err(EvidenceCode::InvalidRepositoryBinding.into())
            }
            Self::RecentCommits { reference, .. } if reference.trim().is_empty() => {
                Err(EvidenceCode::InvalidRepositoryBinding.into())
            }
            Self::Union { members } => {
                if members.is_empty()
                    || members
                        .iter()
                        .any(|member| matches!(member, Self::Union { .. }))
                {
                    return Err(EvidenceCode::InvalidRepositoryBinding.into());
                }
                for member in members {
                    member.validate()?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceRevision {
    Commit {
        commit_id: GitObjectId,
        side: SourceSide,
    },
    Index {
        content_identity: String,
        side: SourceSide,
    },
    Worktree {
        snapshot: SnapshotId,
        side: SourceSide,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrozenRepositoryFile {
    pub source: SourceRef,
    pub text: String,
    pub side: SourceSide,
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SourceSide {
    Before,
    After,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapturedFileSide {
    Present { file: FrozenRepositoryFile },
    Absent,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapturedChange {
    pub path: PathBuf,
    pub before: CapturedFileSide,
    pub after: CapturedFileSide,
    pub before_spans: Vec<ChangeSpan>,
    pub after_spans: Vec<ChangeSpan>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeMemberBasis {
    Full,
    DirtyLayer {
        layer: DirtyLayer,
    },
    Commit {
        commit_id: GitObjectId,
        parent: ParentBasis,
    },
    RevisionDiff {
        base: GitObjectId,
        head: SourceRevision,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParentBasis {
    Root,
    Parent {
        commit_id: GitObjectId,
        position: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CapturedRecencyBasis {
    NotTimeScoped,
    CommitTime {
        window: ResolvedTimeWindow,
    },
    ApproximateFileMtime {
        window: ResolvedTimeWindow,
        excluded_unknown_paths: Vec<PathBuf>,
        included_unknown: Vec<PathBuf>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapturedChangeMember {
    pub scope_position: usize,
    pub basis: ChangeMemberBasis,
    pub changes: Vec<CapturedChange>,
    pub recency: CapturedRecencyBasis,
}
#[derive(Clone, Debug)]
pub struct CapturedChangeSelection {
    pub repository: RepositoryId,
    pub anchor_utc: DateTime<Utc>,
    pub scope: ChangeScope,
    pub snapshot: SnapshotId,
    pub members: Vec<CapturedChangeMember>,
    pub coverage: DiscoveryCoverage,
    pub diagnostics: DiagnosticList,
    pub sources: Vec<FrozenRepositoryFile>,
    pub(crate) mutable_files: Vec<MutableRepositoryFile>,
    pub(crate) acquisitions: Vec<CaptureCoverage>,
    pub(crate) scope_diagnostics: std::collections::BTreeMap<usize, DiagnosticList>,
}
#[derive(Clone, Debug)]
pub(crate) enum MutableRepositoryFile {
    Worktree {
        root: PathBuf,
        path: PathBuf,
        bytes: Option<Vec<u8>>,
    },
    Index {
        root: PathBuf,
        path: PathBuf,
        blob: Option<GitObjectId>,
    },
}

#[cfg(test)]
#[path = "change_contracts_test.rs"]
mod tests;

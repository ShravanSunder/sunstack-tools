use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

validated_name!(ProviderName, |value: &str| !value.is_empty());
validated_name!(SessionId, |value: &str| !value.is_empty());
validated_name!(SourceInputId, |value: &str| !value.is_empty());

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceInput {
    Diff {
        repository: RepositoryId,
        scope: ChangeScope,
    },
    Code {
        repository: RepositoryId,
        revision: RevisionHead,
        paths: Vec<QueryPath>,
    },
    SessionPair {
        provider: ProviderName,
        session: SessionId,
    },
    CustomEntry {
        location: PathBuf,
        schema: CustomEntrySchema,
    },
}

impl SourceInput {
    pub fn validate(&self) -> EvidenceResult<()> {
        match self {
            Self::Diff { scope, .. } => {
                scope.validate().map_err(|mut error| {
                    error.field_path.insert(0, "scope".into());
                    error
                })?;
                if !scope.has_change_selection() {
                    return Err(EvidenceError {
                        code: EvidenceCode::NoChangeSelection,
                        field_path: vec!["scope".into()],
                    });
                }
            }
            Self::Code {
                revision, paths, ..
            } => {
                if matches!(revision,RevisionHead::Commit {revision} if revision.trim().is_empty())
                {
                    return Err(EvidenceError {
                        code: EvidenceCode::RevisionUnavailable,
                        field_path: vec!["revision".into()],
                    });
                }
                for (position, path) in paths.iter().enumerate() {
                    let valid = match path {
                        QueryPath::Exact { path } => {
                            !path.is_absolute()
                                && !path
                                    .components()
                                    .any(|part| matches!(part, std::path::Component::ParentDir))
                        }
                        QueryPath::Glob { pattern } => globset::GlobBuilder::new(pattern)
                            .literal_separator(true)
                            .build()
                            .is_ok(),
                    };
                    if !valid {
                        return Err(EvidenceError {
                            code: EvidenceCode::InvalidSelector,
                            field_path: vec!["paths".into(), position.to_string()],
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn source_identity(&self) -> EvidenceResult<SourceInputId> {
        let bytes = serde_json::to_vec(self).map_err(|_| EvidenceCode::InvalidSelector)?;
        SourceInputId::new(content_digest(&bytes))
    }
}

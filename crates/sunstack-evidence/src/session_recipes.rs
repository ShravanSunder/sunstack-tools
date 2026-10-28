use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderRecipe {
    pub provider: ProviderName,
    pub locate: SessionLocation,
    pub read: TranscriptReading,
    #[serde(default)]
    pub map: RecordMapping,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionLocation {
    pub roots: Vec<PathBuf>,
    pub pattern: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TranscriptReading {
    pub format: TranscriptFormatName,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordMapping {
    pub fields: BTreeMap<String, String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnlocatableReason {
    Missing,
    Ambiguous,
    Unreadable,
}
#[derive(Clone, Debug)]
pub enum SessionLookup {
    Located {
        path: PathBuf,
        format: TranscriptFormatName,
    },
    Unlocatable {
        reason: UnlocatableReason,
    },
}

pub fn builtin_session_recipes(home: &std::path::Path) -> EvidenceResult<Vec<ProviderRecipe>> {
    [
        (
            "claude_code",
            ".claude/projects",
            "**/{session}.jsonl",
            "claude_code_jsonl",
        ),
        (
            "codex",
            ".codex/sessions",
            "**/rollout-*{session}.jsonl",
            "codex_rollout_jsonl",
        ),
    ]
    .into_iter()
    .map(|(provider, root, pattern, format)| {
        Ok(ProviderRecipe {
            provider: ProviderName::new(provider)?,
            locate: SessionLocation {
                roots: vec![home.join(root)],
                pattern: pattern.into(),
            },
            read: TranscriptReading {
                format: TranscriptFormatName::new(format)?,
            },
            map: RecordMapping::default(),
        })
    })
    .collect()
}

pub fn locate_session(
    recipe: &ProviderRecipe,
    session: &SessionId,
) -> EvidenceResult<SessionLookup> {
    // Quote the session identifier as a literal inside the caller's glob.
    let escaped = session
        .as_str()
        .chars()
        .map(|character| match character {
            '*' => "[*]".into(),
            '?' => "[?]".into(),
            '[' => "[[]".into(),
            ']' => "[]]".into(),
            '{' => "[{]".into(),
            '}' => "[}]".into(),
            '\\' => "\\\\".into(),
            other => other.to_string(),
        })
        .collect::<String>();
    let pattern = recipe.locate.pattern.replace("{session}", &escaped);
    let matcher = globset::GlobBuilder::new(&pattern)
        .literal_separator(true)
        .build()
        .map_err(|_| EvidenceCode::InvalidSelector)?
        .compile_matcher();
    let mut matches = BTreeSet::new();
    let mut unreadable = false;
    for root in &recipe.locate.roots {
        if !root.exists() {
            continue;
        }
        for entry in ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(false)
            .git_exclude(false)
            .git_global(false)
            .build()
        {
            match entry {
                Ok(entry) if entry.file_type().is_some_and(|kind| kind.is_file()) => {
                    if entry
                        .path()
                        .strip_prefix(root)
                        .is_ok_and(|relative| matcher.is_match(relative))
                    {
                        match entry.path().canonicalize() {
                            Ok(path) => {
                                matches.insert(path);
                            }
                            Err(_) => unreadable = true,
                        }
                    }
                }
                Err(_) => unreadable = true,
                _ => {}
            }
        }
    }
    Ok(match matches.len() {
        0 => SessionLookup::Unlocatable {
            reason: if unreadable {
                UnlocatableReason::Unreadable
            } else {
                UnlocatableReason::Missing
            },
        },
        1 => match matches.into_iter().next() {
            Some(path) => SessionLookup::Located {
                path,
                format: recipe.read.format.clone(),
            },
            None => return Err(EvidenceCode::TranscriptUnavailable.into()),
        },
        _ => SessionLookup::Unlocatable {
            reason: UnlocatableReason::Ambiguous,
        },
    })
}

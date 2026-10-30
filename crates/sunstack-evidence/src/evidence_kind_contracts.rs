use crate::{MatchRange, SourceSpan};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroU32, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid evidence identifier: {0}")]
pub struct InvalidEvidenceName(String);

macro_rules! evidence_name {
    ($name:ident, $validate:expr) => {
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidEvidenceName> {
                let value = value.into();
                if value.is_empty() || !($validate)(&value) {
                    return Err(InvalidEvidenceName(value));
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidEvidenceName;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
evidence_name!(EvidenceKindName, |_: &str| true);
evidence_name!(EvidenceRuleId, |_: &str| true);
impl std::borrow::Borrow<str> for EvidenceRuleId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
evidence_name!(FieldName, |value: &str| !["text", "where", "standard"]
    .contains(&value)
    && value.split('_').all(
        |word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase())
    ));

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSettings {
    pub ast_grep: PathBuf,
    pub kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceKindDefinition {
    pub rule: EvidenceRuleId,
    pub fields: BTreeMap<FieldName, FieldSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_chars: Option<NonZeroU32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "from", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldSource {
    FirstLine {},
    Capture {
        regex: String,
    },
    Inside {
        rule: EvidenceRuleId,
    },
    Enclosing {
        rule: EvidenceRuleId,
    },
    SameFile {
        rule: EvidenceRuleId,
    },
    Yaml {
        key: String,
    },
    Remainder {},
    Target {
        regex: String,
        resolve: EvidenceTargetResolution,
        #[serde(default = "default_opening_chars")]
        opening_chars: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        as_kind: Option<EvidenceKindName>,
    },
}
fn default_opening_chars() -> usize {
    3000
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTargetResolution {
    Path,
    Skill,
    PathOrSkill,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldShape {
    Text,
    Texts,
    Targets,
}
impl FieldSource {
    pub const fn shape(&self) -> FieldShape {
        match self {
            Self::FirstLine {} | Self::Capture { .. } | Self::Yaml { .. } | Self::Remainder {} => {
                FieldShape::Text
            }
            Self::Inside { .. } | Self::Enclosing { .. } | Self::SameFile { .. } => {
                FieldShape::Texts
            }
            Self::Target { .. } => FieldShape::Targets,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldValue {
    Text {
        value: String,
        spans: Vec<SourceSpan>,
    },
    Texts {
        values: Vec<String>,
        spans: Vec<SourceSpan>,
    },
    Targets {
        values: Vec<LoadTarget>,
    },
    Absent,
    Unavailable {
        reason: FieldUnavailable,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldUnavailable {
    FrontmatterMalformed,
    NotAString,
    Unreadable,
}
impl FieldValue {
    pub fn spans(&self) -> &[SourceSpan] {
        match self {
            Self::Text { spans, .. } | Self::Texts { spans, .. } => spans,
            Self::Targets { .. } | Self::Absent | Self::Unavailable { .. } => &[],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LoadTarget {
    pub name: String,
    pub path: PathBuf,
    pub exists: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<BTreeMap<FieldName, FieldValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PieceLocation {
    pub file: PathBuf,
    pub first_line: usize,
    pub last_line: usize,
    pub span: SourceSpan,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EvidencePiece {
    pub kind: EvidenceKindName,
    #[serde(rename = "where")]
    pub location: PieceLocation,
    pub text: String,
    pub fields: BTreeMap<FieldName, FieldValue>,
    pub model_view: ModelView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ModelFieldValue {
    Text(String),
    List(Vec<ModelFieldValue>),
    Object(BTreeMap<String, ModelFieldValue>),
    Boolean(bool),
    Absent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ModelView {
    Bounded {
        fields: BTreeMap<FieldName, ModelFieldValue>,
        cut: Vec<FieldName>,
    },
    TooLarge {
        chars: usize,
        limit: usize,
    },
}

pub fn match_within(inner: &MatchRange, outer: &MatchRange) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[cfg(test)]
#[path = "evidence_kind_contracts_test.rs"]
mod tests;

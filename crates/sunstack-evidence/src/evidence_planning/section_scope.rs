use crate::EvidenceKindName;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum SectionScope {
    Section { section: EvidenceKindName },
    Prefix { section_prefix: EvidenceKindName },
}
impl SectionScope {
    pub fn kind(&self) -> &EvidenceKindName {
        match self {
            Self::Section { section } => section,
            Self::Prefix { section_prefix } => section_prefix,
        }
    }
}

#[cfg(test)]
#[path = "section_scope_test.rs"]
mod tests;

use crate::{ContainingKinds, EvidenceKindName};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum SectionScope {
    Section { section: EvidenceKindName },
    Prefix { section_prefix: EvidenceKindName },
    Own { section_own: ContainingKinds },
}
impl SectionScope {
    pub fn kinds(&self) -> &[EvidenceKindName] {
        match self {
            Self::Section { section } => std::slice::from_ref(section),
            Self::Prefix { section_prefix } => std::slice::from_ref(section_prefix),
            Self::Own { section_own } => section_own.as_slice(),
        }
    }
}

#[cfg(test)]
#[path = "section_scope_test.rs"]
mod tests;

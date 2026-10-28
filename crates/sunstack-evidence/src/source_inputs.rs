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
    pub fn source_identity(&self) -> EvidenceResult<SourceInputId> {
        let bytes = serde_json::to_vec(self).map_err(|_| EvidenceCode::InvalidSelector)?;
        SourceInputId::new(content_digest(&bytes))
    }
}

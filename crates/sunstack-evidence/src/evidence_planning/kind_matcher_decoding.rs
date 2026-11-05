use super::evidence_kind_contracts::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroU32};

// The closed wire owns the exclusive matcher keys; the runtime has only valid variants.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("oneOf" = [{"required":["rule"]}, {"required":["inline_rule"]}]))]
pub(super) struct EvidenceKindWire {
    #[serde(
        default,
        deserialize_with = "present_matcher",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "EvidenceRuleId")]
    rule: Option<EvidenceRuleId>,
    #[serde(
        default,
        deserialize_with = "present_matcher",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String")]
    inline_rule: Option<String>,
    fields: BTreeMap<FieldName, FieldSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_chars: Option<NonZeroU32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    select: Option<SelectionWire>,
    #[serde(
        default,
        deserialize_with = "decode_containing_kinds",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "ContainingKinds")]
    within: Option<ContainingKinds>,
}

fn present_matcher<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn decode_containing_kinds<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ContainingKinds>, D::Error> {
    ContainingKinds::deserialize(deserializer)
        .map(Some)
        .map_err(|error| serde::de::Error::custom(format!("within: {error}")))
}

impl TryFrom<EvidenceKindWire> for EvidenceKindDefinition {
    type Error = &'static str;
    fn try_from(wire: EvidenceKindWire) -> Result<Self, Self::Error> {
        let matcher = match (wire.rule, wire.inline_rule) {
            (Some(rule), None) => KindMatcher::Named(rule),
            (None, Some(text)) => KindMatcher::Inline { text },
            _ => return Err("evidence kind needs exactly one of rule or inline_rule"),
        };
        Ok(Self {
            matcher,
            fields: wire.fields,
            max_chars: wire.max_chars,
            select: wire.select,
            within: wire.within,
        })
    }
}

impl From<EvidenceKindDefinition> for EvidenceKindWire {
    fn from(kind: EvidenceKindDefinition) -> Self {
        let (rule, inline_rule) = match kind.matcher {
            KindMatcher::Named(rule) => (Some(rule), None),
            KindMatcher::Inline { text } => (None, Some(text)),
        };
        Self {
            rule,
            inline_rule,
            fields: kind.fields,
            max_chars: kind.max_chars,
            select: kind.select,
            within: kind.within,
        }
    }
}

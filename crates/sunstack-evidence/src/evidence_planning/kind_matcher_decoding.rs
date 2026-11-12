use super::evidence_kind_contracts::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroU32};

// The closed wire owns the exclusive matcher keys; the runtime has only valid variants.
#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("oneOf" = [{"required":["rule"]}, {"required":["inline_rule"]}, {"required":["html_units"]}]))]
pub(super) struct EvidenceKindWire {
    #[serde(
        default,
        deserialize_with = "present_matcher",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "EvidenceRuleId")]
    #[cfg_attr(feature = "typescript", specta(type = EvidenceRuleId, optional))]
    rule: Option<EvidenceRuleId>,
    #[serde(
        default,
        deserialize_with = "present_matcher",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "String")]
    #[cfg_attr(feature = "typescript", specta(type = String, optional))]
    inline_rule: Option<String>,
    #[serde(
        default,
        deserialize_with = "present_matcher",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "crate::HtmlUnitSelection")]
    #[cfg_attr(feature = "typescript", specta(type = crate::HtmlUnitSelection, optional))]
    html_units: Option<crate::HtmlUnitSelection>,
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
    #[cfg_attr(feature = "typescript", specta(type = ContainingKinds, optional))]
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
        let matcher = match (wire.rule, wire.inline_rule, wire.html_units) {
            (Some(rule), None, None) => KindMatcher::Named(rule),
            (None, Some(text), None) => KindMatcher::Inline { text },
            (None, None, Some(selection)) => KindMatcher::HtmlUnits(selection),
            _ => return Err("evidence kind needs exactly one of rule, inline_rule or html_units"),
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
        let (rule, inline_rule, html_units) = match kind.matcher {
            KindMatcher::Named(rule) => (Some(rule), None, None),
            KindMatcher::Inline { text } => (None, Some(text), None),
            KindMatcher::HtmlUnits(selection) => (None, None, Some(selection)),
        };
        Self {
            rule,
            inline_rule,
            html_units,
            fields: kind.fields,
            max_chars: kind.max_chars,
            select: kind.select,
            within: kind.within,
        }
    }
}

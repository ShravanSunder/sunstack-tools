use super::evidence_kind_contracts::*;
use serde::Deserialize;

// Decode the closed field envelope before resolving its exclusive rule/kind source.
// Serde's deny_unknown_fields cannot recognize a flattened untagged enum here.
#[derive(Deserialize)]
#[serde(tag = "from", rename_all = "snake_case", deny_unknown_fields)]
enum FieldSourceWire {
    FirstLine {},
    Capture {
        regex: String,
    },
    Inside {
        #[serde(default, deserialize_with = "present_match_source")]
        rule: Option<EvidenceRuleId>,
        #[serde(default, deserialize_with = "present_match_source")]
        kind: Option<EvidenceKindName>,
    },
    Enclosing {
        #[serde(default, deserialize_with = "present_match_source")]
        rule: Option<EvidenceRuleId>,
        #[serde(default, deserialize_with = "present_match_source")]
        kind: Option<EvidenceKindName>,
        #[serde(default)]
        text: ContextText,
    },
    SameFile {
        #[serde(default, deserialize_with = "present_match_source")]
        rule: Option<EvidenceRuleId>,
        #[serde(default, deserialize_with = "present_match_source")]
        kind: Option<EvidenceKindName>,
        #[serde(default)]
        text: ContextText,
    },
    Next {
        rule: EvidenceRuleId,
        #[serde(default)]
        count: NeighbourCount,
    },
    Previous {
        rule: EvidenceRuleId,
        #[serde(default)]
        count: NeighbourCount,
    },
    Yaml {
        key: String,
    },
    Remainder {
        exclude: Option<MatchSource>,
    },
    Target {
        regex: String,
        resolve: EvidenceTargetResolution,
        #[serde(default = "default_opening_chars")]
        opening_chars: usize,
        as_kind: Option<EvidenceKindName>,
        #[serde(default, deserialize_with = "target_exclusion")]
        exclude: Option<TargetExclude>,
    },
}
fn target_exclusion<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<TargetExclude>, D::Error> {
    Option::<TargetExclude>::deserialize(deserializer)
        .map_err(|error| serde::de::Error::custom(format!("target exclude: {error}")))
}
// Missing source keys use the field default; a present key must decode as T,
// so an explicit null cannot disappear into Option's missing-key state.
fn present_match_source<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
impl<'de> Deserialize<'de> for FieldSource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let source = |rule, kind| match (rule, kind) {
            (Some(rule), None) => Ok(MatchSource::Rule { rule }),
            (None, Some(kind)) => Ok(MatchSource::Kind { kind }),
            _ => Err(serde::de::Error::custom(
                "field source needs exactly one of rule or kind",
            )),
        };
        Ok(match FieldSourceWire::deserialize(deserializer)? {
            FieldSourceWire::FirstLine {} => Self::FirstLine {},
            FieldSourceWire::Capture { regex } => Self::Capture { regex },
            FieldSourceWire::Inside { rule, kind } => Self::Inside {
                of: source(rule, kind)?,
            },
            FieldSourceWire::Enclosing { rule, kind, text } => Self::Enclosing {
                of: source(rule, kind)?,
                text,
            },
            FieldSourceWire::SameFile { rule, kind, text } => Self::SameFile {
                of: source(rule, kind)?,
                text,
            },
            FieldSourceWire::Next { rule, count } => Self::Next { rule, count },
            FieldSourceWire::Previous { rule, count } => Self::Previous { rule, count },
            FieldSourceWire::Yaml { key } => Self::Yaml { key },
            FieldSourceWire::Remainder { exclude } => Self::Remainder { exclude },
            FieldSourceWire::Target {
                regex,
                resolve,
                opening_chars,
                as_kind,
                exclude,
            } => Self::Target {
                regex,
                resolve,
                opening_chars,
                as_kind,
                exclude,
            },
        })
    }
}

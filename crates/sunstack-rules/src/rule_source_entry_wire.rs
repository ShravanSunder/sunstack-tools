use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sunstack_evidence::EvidenceDeclaration;

// Parse known fields strictly before selecting one verdict. This preserves
// precise unknown-field diagnostics while the domain enum excludes mixtures.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceEntryWire {
    name: RuleId,
    target: RuleSourceTarget,
    #[serde(default)]
    context: Vec<EvidenceDeclaration>,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    threshold: Option<f64>,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    error_at_or_below: Option<Option<f64>>,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    expect: Option<ExpectPredicate>,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    ast_grep: Option<Box<AstGrepRule>>,
    #[serde(default)]
    severity: Severity,
}
fn present_value<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}
impl TryFrom<SourceEntryWire> for RuleSourceEntry {
    type Error = &'static str;
    fn try_from(wire: SourceEntryWire) -> Result<Self, Self::Error> {
        let verdict = match (
            wire.threshold,
            wire.error_at_or_below,
            wire.expect,
            wire.ast_grep,
        ) {
            (Some(threshold), error_at_or_below, None, None) => RuleSourceVerdict::Judgment {
                threshold,
                error_at_or_below: error_at_or_below.flatten(),
            },
            (None, None, Some(expect), None) => RuleSourceVerdict::Expect { expect },
            (None, None, None, Some(ast_grep)) => RuleSourceVerdict::AstGrep { ast_grep },
            _ => return Err("use exactly one judgment, expect or ast_grep verdict"),
        };
        Ok(Self {
            name: wire.name,
            target: wire.target,
            context: wire.context,
            verdict,
            severity: wire.severity,
        })
    }
}
impl From<RuleSourceEntry> for SourceEntryWire {
    fn from(entry: RuleSourceEntry) -> Self {
        let mut wire = Self {
            name: entry.name,
            target: entry.target,
            context: entry.context,
            severity: entry.severity,
            threshold: None,
            error_at_or_below: None,
            expect: None,
            ast_grep: None,
        };
        match entry.verdict {
            RuleSourceVerdict::Judgment {
                threshold,
                error_at_or_below,
            } => {
                wire.threshold = Some(threshold);
                wire.error_at_or_below = error_at_or_below.map(Some);
            }
            RuleSourceVerdict::Expect { expect } => wire.expect = Some(expect),
            RuleSourceVerdict::AstGrep { ast_grep } => wire.ast_grep = Some(ast_grep),
        }
        wire
    }
}

// These alternatives describe the exclusive flat transport schema. The raw
// decoder above deliberately retains native field errors instead of Serde's
// generic untagged-enum failure; both shapes have permanent parity controls.
#[allow(dead_code)]
#[derive(JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
enum SourceEntrySchema {
    Judgment {
        name: RuleId,
        target: RuleSourceTarget,
        #[serde(default)]
        context: Vec<EvidenceDeclaration>,
        #[schemars(extend("exclusiveMinimum"=0,"maximum"=1))]
        threshold: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("minimum"=0,"exclusiveMaximum"=1))]
        error_at_or_below: Option<f64>,
        #[serde(default)]
        severity: Severity,
    },
    Expect {
        name: RuleId,
        target: RuleSourceTarget,
        #[serde(default)]
        context: Vec<EvidenceDeclaration>,
        expect: ExpectPredicate,
        #[serde(default)]
        severity: Severity,
    },
    AstGrep {
        name: RuleId,
        target: RuleSourceTarget,
        #[serde(default)]
        context: Vec<EvidenceDeclaration>,
        ast_grep: Box<AstGrepRule>,
        #[serde(default)]
        severity: Severity,
    },
}
impl JsonSchema for SourceEntryWire {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "RuleSourceEntry".into()
    }
    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        SourceEntrySchema::json_schema(generator)
    }
}

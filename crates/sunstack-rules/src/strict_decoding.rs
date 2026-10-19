use crate::rule_diagnostics::*;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use std::{collections::BTreeSet, fmt, path::Path};
// First boundary pass rejects duplicates in every object, including otherwise free author content.
// It retains no parallel JSON tree; the typed deserializer consumes the original bytes next.
struct UniqueJson;
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("one JSON value with unique object keys")
            }
            fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_f64<E>(self, _: f64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                while sequence.next_element::<UniqueJson>()?.is_some() {}
                Ok(UniqueJson)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut keys = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !keys.insert(key) {
                        return Err(serde::de::Error::custom("duplicate field"));
                    }
                    map.next_value::<UniqueJson>()?;
                }
                Ok(UniqueJson)
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}
pub(crate) fn decode_definition<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    file: &Path,
    line: usize,
) -> Result<T, DiagnosticList> {
    let diagnostic = |error: &serde_json::Error, path: Vec<String>| -> DiagnosticList {
        let message = error.to_string();
        let code = if message.starts_with("duplicate field") {
            RuleDiagnosticCode::DuplicateField
        } else if message.starts_with("unknown field") {
            RuleDiagnosticCode::UnknownField
        } else {
            RuleDiagnosticCode::MalformedJson
        };
        let mut diagnostic = RuleDiagnostic::new(code);
        // Only these fixed migration messages may cross the diagnostic privacy boundary.
        if message.starts_with("severity `block` was renamed to `error`") {
            diagnostic.guidance = "Use severity `error`; the old severity word is refused.".into();
        } else if message.starts_with("severity `nudge` was renamed to `warn`") {
            diagnostic.guidance = "Use severity `warn`; the old severity word is refused.".into();
        }
        diagnostic
            .definition(
                file,
                line.saturating_add(error.line()).saturating_sub(1),
                error.column(),
                path,
            )
            .into()
    };
    let mut uniqueness = serde_json::Deserializer::from_slice(bytes);
    let _: UniqueJson = serde_path_to_error::deserialize(&mut uniqueness)
        .map_err(|error| diagnostic(error.inner(), safe_path(error.path())))?;
    uniqueness
        .end()
        .map_err(|error| diagnostic(&error, vec![]))?;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let result = serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let safe = safe_path(error.path());
        diagnostic(error.inner(), safe)
    })?;
    deserializer
        .end()
        .map_err(|error| diagnostic(&error, vec![]))?;
    Ok(result)
}
fn safe_path(path: &serde_path_to_error::Path) -> Vec<String> {
    path.iter()
        .filter_map(|segment| match segment {
            serde_path_to_error::Segment::Map { key } if safe_field(key) => Some(key.clone()),
            serde_path_to_error::Segment::Seq { index } => Some(index.to_string()),
            _ => None,
        })
        .take(32)
        .collect()
}
fn safe_field(field: &str) -> bool {
    matches!(
        field,
        "instructions"
            | "criteria"
            | "schema_version"
            | "name"
            | "description"
            | "inputs"
            | "applicability"
            | "common_ruleset_instructions"
            | "rule_paths"
            | "rule_sources"
            | "markdown"
            | "rules"
            | "marker_prefix"
            | "context"
            | "output"
            | "max_question_words"
            | "rule_id"
            | "evidence"
            | "target"
            | "questions"
            | "verdict"
            | "severity"
            | "kind"
            | "type"
            | "threshold"
            | "block_at_or_below"
            | "dimensions"
            | "weight"
            | "question"
            | "routes"
            | "confidence_floor"
            | "bindings"
            | "binding_groups"
            | "select"
            | "input"
            | "id"
            | "requirement"
            | "test"
            | "pointer"
            | "pattern"
            | "max_scan_bytes"
            | "definition_root"
            | "definitions"
            | "repositories"
            | "payload_schemas"
            | "origin"
            | "cursors"
            | "execution"
            | "limits"
            | "backend"
            | "attempts"
            | "request_budget"
    )
}
pub fn decode_rule_invocation(bytes: &[u8]) -> Result<crate::RuleInvocationInput, DiagnosticList> {
    decode_definition(bytes, Path::new("<invocation>"), 1)
}

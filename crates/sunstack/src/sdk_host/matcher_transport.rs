//! Pinned ast-grep matcher and utility transport.
//!
//! Upstream `SerializableRuleConfig<SupportLang>` has no schema because `SupportLang`
//! has none. These small wrappers compose the upstream schema-bearing core with the
//! accepted wrapper properties and a canonical language-name union. They exist for
//! declaration generation and decoding; the upstream decoder still decides validity.
use ast_grep_config::{
    SerializableGlobalRule, SerializableRewriter, SerializableRuleConfig, SerializableRuleCore,
    Severity,
};
use ast_grep_language::SupportLang;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap};

/// Canonical upstream language spellings accepted by the pinned decoder.
///
/// Distinct from the snake_case `EvidenceLanguage` in results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AuthoredLanguage {
    Bash,
    C,
    Cpp,
    CSharp,
    Css,
    Dart,
    Elixir,
    Go,
    Haskell,
    Hcl,
    Html,
    Java,
    JavaScript,
    Json,
    Kotlin,
    Lua,
    Markdown,
    Nix,
    Php,
    Python,
    Ruby,
    Rust,
    Scala,
    Solidity,
    Swift,
    Tsx,
    TypeScript,
    Yaml,
    Zig,
}

impl AuthoredLanguage {
    pub const ALL: [Self; 29] = [
        Self::Bash,
        Self::C,
        Self::Cpp,
        Self::CSharp,
        Self::Css,
        Self::Dart,
        Self::Elixir,
        Self::Go,
        Self::Haskell,
        Self::Hcl,
        Self::Html,
        Self::Java,
        Self::JavaScript,
        Self::Json,
        Self::Kotlin,
        Self::Lua,
        Self::Markdown,
        Self::Nix,
        Self::Php,
        Self::Python,
        Self::Ruby,
        Self::Rust,
        Self::Scala,
        Self::Solidity,
        Self::Swift,
        Self::Tsx,
        Self::TypeScript,
        Self::Yaml,
        Self::Zig,
    ];
}

/// Recursive JSON data: the representable subset of upstream YAML metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonData(pub serde_json::Value);

impl JsonSchema for JsonData {
    fn schema_name() -> Cow<'static, str> {
        "JsonData".into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let item = generator.subschema_for::<Self>();
        json_schema!({
            "anyOf": [
                {"type": "null"},
                {"type": "boolean"},
                {"type": "number"},
                {"type": "string"},
                {"type": "array", "items": item},
                {"type": "object", "additionalProperties": item}
            ]
        })
    }
}

/// Upstream `RuleFileGlob`, which is not exported by the pinned crate.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RuleFileGlob {
    Glob(String),
    #[serde(rename_all = "camelCase")]
    Config {
        glob: String,
        #[serde(default)]
        case_insensitive: bool,
    },
}

/// Upstream `LabelConfig`, which is not exported by the pinned crate.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LabelConfig {
    pub style: ast_grep_config::LabelStyle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn fixer_schema(generator: &mut SchemaGenerator) -> Schema {
    // `SerializableFixer` is reachable only through the exported rewriter schema.
    generator.subschema_for::<SerializableRewriter>();
    let path = generator.settings().definitions_path.clone();
    json_schema!({"$ref": format!("#{path}/SerializableFixer")})
}

/// The declared transport shape of a matcher: the pinned upstream core composed with
/// every accepted wrapper property. Used only to generate declarations; values are
/// decoded by the upstream decoder into [`AstMatcher`].
#[derive(Serialize, Deserialize, JsonSchema)]
#[schemars(title = "AstMatcher", rename = "AstMatcher")]
pub struct AstMatcherSchema {
    #[serde(flatten)]
    pub core: SerializableRuleCore,
    pub language: AuthoredLanguage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "fixer_schema")]
    pub fix: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rewriters: Option<Vec<SerializableRewriter>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<BTreeMap<String, LabelConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<RuleFileGlob>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignores: Option<Vec<RuleFileGlob>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, JsonData>>,
}

/// The declared transport shape of a global utility; see [`AstMatcherSchema`].
#[derive(Serialize, Deserialize, JsonSchema)]
#[schemars(title = "AstUtility", rename = "AstUtility")]
pub struct AstUtilitySchema {
    #[serde(flatten)]
    pub core: SerializableRuleCore,
    pub language: AuthoredLanguage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<String>>,
}

/// Wrapper properties the transport exposes; compared with upstream in tests.
pub const MATCHER_WRAPPER_FIELDS: [&str; 15] = [
    "rule",
    "constraints",
    "utils",
    "transform",
    "language",
    "fix",
    "rewriters",
    "message",
    "note",
    "severity",
    "labels",
    "files",
    "ignores",
    "url",
    "metadata",
];

/// Why a transported definition was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportRefusal {
    pub kind: TransportRefusalKind,
    /// Path inside the definition value.
    pub json_path: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportRefusalKind {
    /// The pinned decoder refused the value.
    Decoder,
    /// An authored value declared an `id` the map key owns.
    DeclaredId,
    /// Representable only in YAML; the SDK cannot carry it without loss.
    Unrepresentable,
}

fn decode_with_id<T: serde::de::DeserializeOwned>(
    name: &str,
    value: &serde_json::Value,
) -> Result<T, TransportRefusal> {
    let serde_json::Value::Object(object) = value else {
        return Err(TransportRefusal {
            kind: TransportRefusalKind::Decoder,
            json_path: vec![],
            message: "definition must be an object".into(),
        });
    };
    if object.contains_key("id") {
        return Err(TransportRefusal {
            kind: TransportRefusalKind::DeclaredId,
            json_path: vec!["id".into()],
            message: "an inline definition must not declare id; its map key supplies it".into(),
        });
    }
    let mut object = object.clone();
    object.insert("id".into(), serde_json::Value::String(name.into()));
    serde_path_to_error::deserialize(serde_json::Value::Object(object)).map_err(|error| {
        TransportRefusal {
            kind: TransportRefusalKind::Decoder,
            json_path: error
                .path()
                .iter()
                .filter_map(|segment| match segment {
                    serde_path_to_error::Segment::Map { key } => Some(key.clone()),
                    serde_path_to_error::Segment::Seq { index } => Some(index.to_string()),
                    _ => None,
                })
                .collect(),
            message: error.inner().to_string(),
        }
    })
}

/// Decode a transported matcher with the pinned upstream decoder.
pub fn decode_matcher(
    name: &str,
    value: &serde_json::Value,
) -> Result<SerializableRuleConfig<SupportLang>, TransportRefusal> {
    decode_with_id(name, value)
}

/// Decode a transported utility with the pinned upstream decoder.
pub fn decode_utility(
    name: &str,
    value: &serde_json::Value,
) -> Result<SerializableGlobalRule<SupportLang>, TransportRefusal> {
    decode_with_id(name, value)
}

fn refuse_unrepresentable(
    value: &serde_value::Value,
    path: &mut Vec<String>,
) -> Result<(), TransportRefusal> {
    let refusal = |path: &[String], message: &str| TransportRefusal {
        kind: TransportRefusalKind::Unrepresentable,
        json_path: path.to_vec(),
        message: message.into(),
    };
    match value {
        serde_value::Value::F32(number) if !number.is_finite() => {
            Err(refusal(path, "metadata holds a non-finite number"))
        }
        serde_value::Value::F64(number) if !number.is_finite() => {
            Err(refusal(path, "metadata holds a non-finite number"))
        }
        serde_value::Value::Bytes(_) => Err(refusal(path, "metadata holds bytes")),
        serde_value::Value::Map(map) => {
            for (key, item) in map {
                let serde_value::Value::String(key) = key else {
                    return Err(refusal(path, "metadata holds a non-string key"));
                };
                path.push(key.clone());
                refuse_unrepresentable(item, path)?;
                path.pop();
            }
            Ok(())
        }
        serde_value::Value::Seq(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(index.to_string());
                refuse_unrepresentable(item, path)?;
                path.pop();
            }
            Ok(())
        }
        serde_value::Value::Option(Some(item)) | serde_value::Value::Newtype(item) => {
            refuse_unrepresentable(item, path)
        }
        _ => Ok(()),
    }
}

fn transport_value<T: Serialize>(
    definition: &T,
    metadata: Option<&ast_grep_config::Metadata>,
) -> Result<serde_json::Value, TransportRefusal> {
    if let Some(metadata) = metadata {
        let value = serde_value::to_value(metadata).map_err(|error| TransportRefusal {
            kind: TransportRefusalKind::Unrepresentable,
            json_path: vec!["metadata".into()],
            message: error.to_string(),
        })?;
        refuse_unrepresentable(&value, &mut vec!["metadata".into()])?;
    }
    let mut value = serde_json::to_value(definition).map_err(|error| TransportRefusal {
        kind: TransportRefusalKind::Unrepresentable,
        json_path: vec![],
        message: error.to_string(),
    })?;
    if let serde_json::Value::Object(object) = &mut value {
        object.remove("id");
    }
    Ok(super::normalized_values::canonical_value(value))
}

/// The canonical transport value of a decoded matcher: canonical language spelling,
/// absent optional keys omitted, no `id`.
pub fn matcher_value(
    matcher: &SerializableRuleConfig<SupportLang>,
) -> Result<serde_json::Value, TransportRefusal> {
    transport_value(matcher, matcher.metadata.as_ref())
}

/// The canonical transport value of a decoded utility.
pub fn utility_value(
    utility: &SerializableGlobalRule<SupportLang>,
) -> Result<serde_json::Value, TransportRefusal> {
    transport_value(utility, None)
}

impl TransportRefusal {
    /// One-line description: the decoder's message, then where it applies.
    pub fn describe(&self) -> String {
        if self.json_path.is_empty() {
            self.message.clone()
        } else {
            format!("{} (at {})", self.message, self.json_path.join("."))
        }
    }
}

/// One named ast-grep matcher, decoded and validated by the pinned upstream decoder.
///
/// Its map key supplies `id`; an authored `id` is refused. Unknown wrapper keys are
/// ignored and unknown rule or contextual-pattern keys are refused, as upstream does.
#[derive(Clone)]
pub struct AstMatcher(SerializableRuleConfig<SupportLang>);

/// One named global utility, decoded and validated by the pinned upstream decoder.
#[derive(Clone)]
pub struct AstUtility(SerializableGlobalRule<SupportLang>);

impl AstMatcher {
    /// Accept an upstream-decoded matcher whose transport value is representable.
    pub fn from_upstream(
        matcher: SerializableRuleConfig<SupportLang>,
    ) -> Result<Self, TransportRefusal> {
        matcher_value(&matcher)?;
        Ok(Self(matcher))
    }
    /// The upstream matcher carrying `name` as its id.
    pub fn upstream(&self, name: &str) -> SerializableRuleConfig<SupportLang> {
        let mut matcher = self.0.clone();
        matcher.id = name.to_owned();
        matcher
    }
    /// Canonical transport value: canonical language spelling, no absent keys, no id.
    pub fn transport_value(&self) -> Result<serde_json::Value, TransportRefusal> {
        matcher_value(&self.0)
    }
}

impl AstUtility {
    pub fn from_upstream(
        utility: SerializableGlobalRule<SupportLang>,
    ) -> Result<Self, TransportRefusal> {
        utility_value(&utility)?;
        Ok(Self(utility))
    }
    pub fn upstream(&self, name: &str) -> SerializableGlobalRule<SupportLang> {
        let mut utility = self.0.clone();
        utility.id = name.to_owned();
        utility
    }
    pub fn transport_value(&self) -> Result<serde_json::Value, TransportRefusal> {
        utility_value(&self.0)
    }
}

macro_rules! upstream_transport {
    ($name:ident, $decode:ident) => {
        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.transport_value()
                    .map_err(|refusal| serde::ser::Error::custom(refusal.describe()))?
                    .serialize(serializer)
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                // The raw value exists only here and is classified at once.
                let value = serde_json::Value::deserialize(deserializer)?;
                $decode("", &value)
                    .map(Self)
                    .map_err(|refusal| serde::de::Error::custom(refusal.describe()))
            }
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.transport_value() {
                    Ok(value) => write!(formatter, "{}({value})", stringify!($name)),
                    Err(refusal) => {
                        write!(formatter, "{}(<{}>)", stringify!($name), refusal.describe())
                    }
                }
            }
        }
    };
}
upstream_transport!(AstMatcher, decode_matcher);
upstream_transport!(AstUtility, decode_utility);

/// The JSON schema the TypeScript matcher declarations are generated from.
pub fn matcher_schema() -> serde_json::Value {
    let mut generator = schemars::SchemaGenerator::default();
    let matcher = generator.subschema_for::<AstMatcherSchema>();
    let utility = generator.subschema_for::<AstUtilitySchema>();
    let languages = generator.subschema_for::<AuthoredLanguage>();
    let definitions = generator.take_definitions(true);
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "AstGrepTransport",
        "type": "object",
        "properties": {"matcher": matcher, "utility": utility, "language": languages},
        "$defs": definitions,
    })
}

/// These two transports are declared by the generated JSON-schema module, so the
/// native declaration set only references their names.
#[cfg(feature = "typescript")]
macro_rules! externally_declared_type {
    ($name:ident) => {
        impl specta::Type for $name {
            fn definition(_: &mut specta::Types) -> specta::datatype::DataType {
                specta::datatype::DataType::Reference(specta_typescript::define(stringify!($name)))
            }
        }
    };
}
#[cfg(feature = "typescript")]
externally_declared_type!(AstMatcher);
#[cfg(feature = "typescript")]
externally_declared_type!(AstUtility);

#[cfg(test)]
#[path = "matcher_transport_test.rs"]
mod tests;

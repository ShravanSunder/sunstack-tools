#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use std::collections::BTreeSet;

fn refusal_kind<T>(result: Result<T, TransportRefusal>) -> TransportRefusalKind {
    match result {
        Ok(_) => panic!("expected a refusal"),
        Err(refusal) => refusal.kind,
    }
}

const FULL_MATCHER: &str = r#"
id: full
language: ts
rule: {pattern: console.log($A)}
constraints: {A: {kind: identifier}}
utils: {local: {kind: identifier}}
transform: {B: {substring: {source: $A, startChar: 1}}}
fix: console.info($A)
rewriters:
  - id: rewrite-one
    rule: {kind: identifier}
    fix: x
message: Use info.
note: Prefer info.
severity: warning
labels: {A: {style: primary, message: here}}
files: ["src/**", {glob: "lib/**", caseInsensitive: true}]
ignores: ["test/**"]
url: https://example.invalid
metadata: {owner: docs, tags: [a, b], nested: {level: 1}}
"#;

#[test]
fn transport_exposes_exactly_the_upstream_wrapper_properties() {
    let upstream: SerializableRuleConfig<SupportLang> =
        ast_grep_config::from_str(FULL_MATCHER).unwrap();
    let value = matcher_value(&upstream).unwrap();
    let serialized = value
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let declared = MATCHER_WRAPPER_FIELDS
        .iter()
        .map(|field| (*field).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(serialized, declared);

    let schema = matcher_schema();
    let matcher = &schema["$defs"]["AstMatcher"]["properties"];
    let schema_fields = matcher
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(schema_fields, declared);

    // The transport value decodes back to the same canonical value.
    let decoded = decode_matcher("full", &value).unwrap();
    assert_eq!(matcher_value(&decoded).unwrap(), value);
    assert_eq!(value["language"], "TypeScript");
}

#[test]
fn every_authored_language_spelling_decodes_through_upstream() {
    for language in AuthoredLanguage::ALL {
        let name = serde_json::to_value(language).unwrap();
        let value = serde_json::json!({"language": name, "rule": {"kind": "x"}});
        let decoded = decode_utility("probe", &value).unwrap();
        assert_eq!(utility_value(&decoded).unwrap()["language"], name);
    }
    let unsupported = serde_json::json!({"language": "Cobol", "rule": {"kind": "x"}});
    assert_eq!(
        refusal_kind(decode_utility("probe", &unsupported)),
        TransportRefusalKind::Decoder
    );
}

#[test]
fn yaml_only_metadata_is_refused_instead_of_coerced() {
    let upstream: SerializableRuleConfig<SupportLang> = ast_grep_config::from_str(
        "id: nan\nlanguage: md\nrule: {kind: paragraph}\nmetadata: {score: .nan}\n",
    )
    .unwrap();
    let refused = matcher_value(&upstream).unwrap_err();
    assert_eq!(refused.kind, TransportRefusalKind::Unrepresentable);
    assert_eq!(refused.json_path, ["metadata", "score"]);
}

#[test]
fn declared_ids_and_nested_unknown_keys_are_refused() {
    let declared = serde_json::json!({"id": "x", "language": "Markdown", "rule": {"kind": "x"}});
    assert_eq!(
        refusal_kind(decode_matcher("x", &declared)),
        TransportRefusalKind::DeclaredId
    );
    let nested = serde_json::json!({"language": "Markdown", "rule": {"kindz": "x"}});
    assert_eq!(
        refusal_kind(decode_matcher("x", &nested)),
        TransportRefusalKind::Decoder
    );
    let wrapper = serde_json::json!({"language": "Markdown", "rule": {"kind": "x"}, "extra": 1});
    assert!(decode_matcher("x", &wrapper).is_ok());
}

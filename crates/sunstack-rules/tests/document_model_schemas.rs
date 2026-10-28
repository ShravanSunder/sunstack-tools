#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/contract_schemas.rs"]
mod schema_support;
use serde_json::json;
use sunstack_rules::*;
#[test]
fn marked_model_fields_agree_across_native_and_published_shapes_and_cannot_override_meaning() {
    let evidence = json!([{"id":"unit","input":"skill","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}}]);
    let entry = json!({"name":"document.standard","target":{"input":"skill","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}},"mechanical_pass":{"predicate":{"kind":"literal","field":"/text","text":"Use"}},"narrowing":{"question":{"type":"noul","instructions":"Relevant unit?"},"evidence":evidence,"threshold":0.8},"gates":[{"kind":"decision","question":{"type":"noul","instructions":"Applicable unit?"},"evidence":evidence,"threshold":0.8}],"questions":{"extra":{"type":"noul","instructions":"One detail?"}},"threshold":0.8});
    let generated = schema_support::generated::<RuleSourceEntry>();
    let published = schema_support::reviewed("rule-source.schema.json");
    for value in [entry.clone(), {
        let mut value = entry.clone();
        value["pass_means"] = json!("finding");
        value
    }] {
        let valid = value.get("pass_means").is_none();
        assert_eq!(generated.is_valid(&value), valid);
        assert_eq!(published.is_valid(&json!({"K014":value})), valid);
        assert_eq!(
            serde_json::from_value::<RuleSourceEntry>(value).is_ok(),
            valid
        );
    }
    let native: RuleSourceEntry = serde_json::from_value(entry).unwrap();
    let roundtrip = serde_json::to_value(native).unwrap();
    assert_eq!(roundtrip["gates"].as_array().unwrap().len(), 1);
    assert_eq!(roundtrip["narrowing"]["threshold"], 0.8);
    assert_eq!(roundtrip["mechanical_pass"]["predicate"]["text"], "Use");
    assert!(roundtrip["questions"].get("extra").is_some());
}
#[test]
fn judged_full_definitions_default_to_finding_and_explicit_conformance_survives_roundtrip() {
    let base = json!({"rule_id":"document.one","evidence":[{"id":"target","input":"skill","select":{"kind":"frontmatter_field","field":"description"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"Test the field"},"threshold":0.8}});
    let inputs = serde_json::from_value(json!({"skill":{"kind":"document"}})).unwrap();
    for (meaning, expected) in [
        (None, PassMeans::Finding),
        (Some("finding"), PassMeans::Finding),
        (Some("conforms"), PassMeans::Conforms),
    ] {
        let mut value = base.clone();
        if let Some(meaning) = meaning {
            value["verdict"]["pass_means"] = json!(meaning);
        }
        assert!(schema_support::generated::<RuleDefinition>().is_valid(&value));
        assert!(schema_support::reviewed("rule-file.schema.json").is_valid(&value));
        let definition: RuleDefinition = serde_json::from_value(value).unwrap();
        validate_rule_definition(&definition, &inputs).unwrap();
        assert_eq!(definition.verdict.pass_means(), expected);
        let wire = serde_json::to_value(definition).unwrap();
        if expected == PassMeans::Finding {
            assert!(wire["verdict"].get("pass_means").is_none());
        } else {
            assert_eq!(wire["verdict"]["pass_means"], "conforms");
        }
        let roundtrip: RuleDefinition = serde_json::from_value(wire).unwrap();
        assert_eq!(roundtrip.verdict.pass_means(), expected);
    }
}

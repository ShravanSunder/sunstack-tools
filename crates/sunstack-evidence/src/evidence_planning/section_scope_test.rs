#![allow(clippy::unwrap_used)]
use crate::*;
use serde_json::json;

#[test]
fn scoped_inside_schema_and_decoder_agree_on_exclusive_kind_scope() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for scope in [
        json!({"section":"outline"}),
        json!({"section_prefix":"outline"}),
        json!({"section_own":["outline","markdown_outline"]}),
    ] {
        let wire = json!({"from":"inside","kind":"picture","scope":scope});
        let source = serde_json::from_value::<FieldSource>(wire.clone()).unwrap();
        assert_eq!(source.shape(), FieldShape::Texts);
        assert_eq!(serde_json::to_value(source).unwrap(), wire);
        assert!(validator.is_valid(&wire), "{wire}");
    }
    for wire in [
        json!({"from":"inside","rule":"image","scope":{"section":"outline"}}),
        json!({"from":"inside","kind":"picture","scope":null}),
        json!({"from":"inside","kind":"picture","scope":{}}),
        json!({"from":"inside","kind":"picture","scope":{"section":"outline","section_prefix":"outline"}}),
        json!({"from":"inside","kind":"picture","scope":{"section":"outline","typo":true}}),
        json!({"from":"inside","kind":"picture","scope":{"section_own":[]}}),
        json!({"from":"inside","kind":"picture","scope":{"section_own":["outline","outline"]}}),
        json!({"from":"inside","kind":"picture","scope":{"section_own":["outline"],"section":"outline"}}),
    ] {
        assert!(
            serde_json::from_value::<FieldSource>(wire.clone()).is_err(),
            "{wire}"
        );
        assert!(!validator.is_valid(&wire), "{wire}");
    }
}

#[test]
fn scoped_inside_plans_both_dependencies_and_refuses_undefined_or_cyclic_scopes() {
    let wire = json!({
        "outline":{"html_units":{"mode":"unit","units":["section"]},"fields":{}},
        "picture":{"rule":"image","fields":{}},
        "choice":{"rule":"choice","fields":{"pictures":{"from":"inside","kind":"picture","scope":{"section_prefix":"outline"}}}}
    });
    let kinds = serde_json::from_value(wire.clone()).unwrap();
    let plan = EvidenceKindPlan::derive(&kinds).unwrap();
    assert_eq!(
        plan.depends_on[&EvidenceKindName::new("choice").unwrap()].len(),
        2
    );
    for name in ["missing", "choice"] {
        let mut invalid = wire.clone();
        invalid["choice"]["fields"]["pictures"]["scope"]["section_prefix"] = json!(name);
        let kinds = serde_json::from_value(invalid).unwrap();
        assert!(EvidenceKindPlan::derive(&kinds).is_err());
    }
}

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::*;

#[test]
fn verdict_admission_refuses_every_invalid_band_and_composite_dimension_shape() {
    let inputs = std::collections::BTreeMap::from([(
        sunstack_evidence::SourceName::new("subject").unwrap(),
        InputDeclaration::Document {},
    )]);
    let baseline = serde_json::json!({"rule_id":"check","evidence":[{"id":"target","input":"subject","select":{"kind":"paragraph","target_id":"U001"}}],"target":"target","applicability":[],"questions":{"yes":{"type":"noul","instructions":"yes"},"quality":{"type":"score","instructions":"quality","criteria":["low","middle","high"]}},"verdict":{"kind":"composite","dimensions":[{"question":"yes","weight":0.5},{"question":"quality","weight":0.5}],"threshold":0.75},"severity":"error"});
    for case in [
        "valid",
        "missing_dimension",
        "wrong_type",
        "single_level",
        "zero_weight",
        "negative_weight",
        "weight_sum",
    ] {
        let mut raw = baseline.clone();
        match case {
            "missing_dimension" => {
                raw["verdict"]["dimensions"][0]["question"] = serde_json::json!("absent")
            }
            "wrong_type" => {
                raw["questions"]["yes"] = serde_json::json!({"type":"choice","instructions":"choose","criteria":{"a":"a","b":"b"}})
            }
            "single_level" => raw["questions"]["quality"]["criteria"] = serde_json::json!(["only"]),
            "zero_weight" => raw["verdict"]["dimensions"][0]["weight"] = serde_json::json!(0),
            "negative_weight" => {
                raw["verdict"]["dimensions"][0]["weight"] = serde_json::json!(-0.5)
            }
            "weight_sum" => raw["verdict"]["dimensions"][0]["weight"] = serde_json::json!(0.6),
            _ => {}
        }
        let rule = decode_rule_definition(
            &serde_json::to_vec(&raw).unwrap(),
            "rules.jsonl".as_ref(),
            1,
        )
        .unwrap();
        assert_eq!(
            validate_rule_definition(&rule, &inputs).is_ok(),
            case == "valid",
            "{case}"
        );
    }
    for (severity, band, accepted) in [
        ("error", 0.3, true),
        ("error", 0.8, false),
        ("error", 0.9, false),
        ("warn", 0.3, false),
    ] {
        let mut raw = baseline.clone();
        raw["verdict"] = serde_json::json!({"kind":"noul","question":{"type":"noul","instructions":"judge"},"threshold":0.8});
        raw["severity"] = serde_json::json!(severity);
        raw["verdict"]["error_at_or_below"] = serde_json::json!(band);
        let rule = decode_rule_definition(
            &serde_json::to_vec(&raw).unwrap(),
            "rules.jsonl".as_ref(),
            1,
        )
        .unwrap();
        assert_eq!(
            validate_rule_definition(&rule, &inputs).is_ok(),
            accepted,
            "{severity}/{band}"
        );
    }
}

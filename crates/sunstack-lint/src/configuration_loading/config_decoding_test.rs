#![allow(clippy::unwrap_used)]
#[path = "config_decoding_test/existing.rs"]
mod existing;

#[test]
fn admits_explicit_document_conventions_in_project_evidence_config() {
    let wire = serde_json::json!({
        "ast_grep":"evidence/sgconfig.yml", "kinds":{},
        "html":{"skip_tags":["example-source"],
            "text_attributes":{"example-diagram":["caption"]}},
        "markdown":{"heading_anchors":"github"}
    });
    let settings: Result<sunstack_evidence::EvidenceSettings, _> =
        serde_json::from_value(wire.clone());
    assert!(
        settings.is_ok(),
        "explicit project conventions refused: {settings:?}"
    );
    let schema =
        serde_json::to_value(schemars::schema_for!(sunstack_evidence::EvidenceSettings)).unwrap();
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&wire));
}

#[test]
fn rejects_invalid_document_conventions_at_the_config_field() {
    use super::{ConfigLocation, decode_config};
    use serde_json::json;
    let location = ConfigLocation {
        file: "sunstack.lint.json".into(),
        directory: ".".into(),
    };
    let schema = serde_json::to_value(crate::config_schema()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (html, markdown, expected_path) in [
        (json!({"skip_tags":[4]}), json!({}), "html"),
        (json!({"skip_tags":[""]}), json!({}), "html"),
        (json!({"skip_tags":"source"}), json!({}), "html"),
        (json!({"text_attributes":[]}), json!({}), "html"),
        (
            json!({"text_attributes":{"source":"label"}}),
            json!({}),
            "html",
        ),
        (
            json!({"text_attributes":{"source":[false]}}),
            json!({}),
            "html",
        ),
        (
            json!({"text_attributes":{"source":[""]}}),
            json!({}),
            "html",
        ),
        (json!({"text_attributes":{"":["label"]}}), json!({}), "html"),
        (json!({}), json!({"heading_anchors":"unknown"}), "markdown"),
        (json!({}), json!({"heading_anchors":9}), "markdown"),
    ] {
        let config = json!({
            "rulebook":[{"markdown":"rulebook.md","rules":"rules.json"}],
            "marker_prefix":"K","check":{"files":["**/*.html"]},
            "evidence":{"ast_grep":"evidence/sgconfig.yml", "html":html,
                "markdown":markdown,"kinds":{"piece":{"rule":"whole","fields":{}}}},
            "decision_model":{"model":"jev-latest"}
        });
        let text = serde_json::to_string(&config).unwrap();
        assert!(
            !validator.is_valid(&config),
            "schema accepted invalid conventions: {config}"
        );
        let error = decode_config(&text, &location).unwrap_err();
        assert!(
            error.message.contains(&format!("evidence.{expected_path}")),
            "human refusal lost its owning field: {error:?}"
        );
        assert!(
            error
                .json_path
                .iter()
                .any(|part| part.contains(expected_path)),
            "missing owning config field: {error:?}"
        );
    }
}

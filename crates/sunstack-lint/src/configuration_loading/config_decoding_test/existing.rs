#![allow(clippy::unwrap_used)]
use super::super::*;

#[test]
fn within_entry_location_skips_an_earlier_matching_list_and_keeps_fallback() {
    for pretty in [false, true] {
        let value = serde_json::json!({
            "rulebook":[{"markdown":"rulebook.md","rules":"rules.json"}],
            "marker_prefix":"K","check":{"files":["step"]},
            "evidence":{"ast_grep":"evidence/sgconfig.yml","kinds":{
                "step":{"rule":"r","fields":{},
                    "select":{"question":{},"yes_at_or_above":0.7,"no_at_or_below":0.3,"shared":["within","missing"]},
                    "within":["parent","missing"]}
            }},"decision_model":{"model":"jev-latest"}
        });
        let text = if pretty {
            serde_json::to_string_pretty(&value).unwrap()
        } else {
            serde_json::to_string(&value).unwrap()
        };
        let location = ConfigLocation {
            file: "sunstack.lint.json".into(),
            directory: ".".into(),
        };
        let config = decode_config(&text, &location).unwrap();
        let position = text.rfind("\"missing\"").unwrap();
        let prefix = &text[..position];
        let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
        let column =
            u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
        let diagnostic = config_error(
            &config,
            &ConfigPath::keys(["evidence", "kinds", "step", "within"]).with_index(1),
            LintInvalidCode::Syntax,
            "undefined kind missing",
        );
        assert_eq!(
            (diagnostic.line, diagnostic.column),
            (Some(line), Some(column)),
            "pretty={pretty}"
        );
        let absent = config_error(
            &config,
            &ConfigPath::keys(["evidence", "kinds", "absent"]),
            LintInvalidCode::Syntax,
            "undefined kind absent",
        );
        assert_eq!((absent.line, absent.column), (Some(1), Some(1)));
    }
}
#[test]
fn structured_array_indexes_skip_nested_values_and_keep_literal_keys() {
    for pretty in [false, true] {
        let value = serde_json::json!({"rows":[{"ignore":[1,2]},"escaped, \"value\"",{"a.b":{"body":[false,{"literal.key":"wanted"}]}}],"0":"object-key"});
        let text = if pretty {
            serde_json::to_string_pretty(&value).unwrap()
        } else {
            serde_json::to_string(&value).unwrap()
        };
        let indexed = ConfigPath::keys(["rows"])
            .with_index(2)
            .with_key("a.b")
            .with_key("body")
            .with_index(1)
            .with_key("literal.key");
        assert_eq!(
            config_field_position(&text, &indexed),
            text.find("\"literal.key\"")
        );
        assert_eq!(
            config_field_position(&text, &ConfigPath::keys(["rows"]).with_index(1)),
            text.find("\"escaped,")
        );
        assert_eq!(
            config_field_position(&text, &ConfigPath::keys(["rows"]).with_index(3)),
            None
        );
        assert_eq!(
            config_field_position(&text, &ConfigPath::keys(["rows", "2"])),
            None
        );
        assert_eq!(
            config_field_position(&text, &ConfigPath::keys(["0"])),
            text.find("\"0\"")
        );
    }
}
#[test]
fn serde_paths_keep_map_names_and_sequence_indexes_distinct() {
    let text = r#"{"a.b":["valid",false]}"#;
    let mut decoder = serde_json::Deserializer::from_str(text);
    let result: Result<std::collections::BTreeMap<String, Vec<String>>, _> =
        serde_path_to_error::deserialize(&mut decoder);
    let error = result.unwrap_err();
    let path = path_fields(error.path());
    assert_eq!(
        path.segments(),
        &[
            ConfigPathSegment::Key("a.b".into()),
            ConfigPathSegment::Index(1)
        ]
    );
    assert_eq!(path.to_string(), "a.b.1");
    assert_eq!(config_field_position(text, &path), text.find("false"));
}

#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/lint_fixture.rs"]
mod fixture;
use serde_json::json;
use sunstack_decisions::CancellationToken;
use sunstack_lint::*;

async fn assert_kind_refusal_location(scenario: &str) {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["check"]["files"] = json!(["child", "docs/*.md"]);
    config["evidence"]["kinds"]["child"] = json!({"rule":"skill","fields":{}});
    match scenario {
        "undefined_kind" => {
            config["evidence"]["kinds"]["child"]["fields"]["body"] =
                json!({"from":"inside","kind":"missing"})
        }
        "cycle" => {
            config["evidence"]["kinds"]["child"]["within"] = json!(["skill", "z"]);
            config["evidence"]["kinds"]["z"] =
                json!({"rule":"skill","fields":{},"within":["child"]});
        }
        "undefined_within_entry" => {
            config["evidence"]["kinds"]["child"]["within"] = json!(["skill", "missing"])
        }
        _ => panic!(),
    }
    fixture::config_file(root.path(), &config);
    let path = root.path().join("sunstack.lint.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let position = text.find("\"child\": {").unwrap();
    let prefix = &text[..position];
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
    let column = u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
    let loaded = load_config(&ConfigLocation {
        file: path.clone(),
        directory: root.path().to_owned(),
    })
    .unwrap();
    let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
    else {
        panic!()
    };

    let diagnostic = diagnostics
        .iter()
        .find(|row| {
            row.message.starts_with("undefined kind ")
                || row.message.starts_with("kind dependency cycle")
        })
        .unwrap();
    assert_eq!(diagnostic.file, Some(path));
    assert_eq!(
        (diagnostic.line, diagnostic.column),
        (Some(line), Some(column)),
        "{scenario}: {diagnostics:?}"
    );
    assert!(line > 1);
}

#[tokio::test]
async fn kind_plan_refusals_locate_undefined_kind() {
    assert_kind_refusal_location("undefined_kind").await;
}

#[tokio::test]
async fn kind_plan_refusals_locate_cycle() {
    assert_kind_refusal_location("cycle").await;
}

#[tokio::test]
async fn kind_plan_refusals_locate_undefined_within_entry() {
    assert_kind_refusal_location("undefined_within_entry").await;
}

#[tokio::test]
async fn cycle_location_and_path_are_stable_across_declared_kind_orders() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["evidence"]["kinds"] = json!("KINDS_PLACEHOLDER");
    let envelope = serde_json::to_string(&config).unwrap();
    let child = r#""child":{"rule":"skill","fields":{},"within":["skill","z"]}"#;
    let skill =
        r#""skill":{"rule":"skill","fields":{"description":{"from":"yaml","key":"description"}}}"#;
    let z = r#""z":{"rule":"skill","fields":{},"within":["child"]}"#;
    let mut observed = Vec::new();
    for kinds in [
        format!("{{{child},{skill},{z}}}"),
        format!("{{{child},{z},{skill}}}"),
    ] {
        let text = envelope.replace("\"KINDS_PLACEHOLDER\"", &kinds);
        let path = root.path().join("sunstack.lint.json");
        std::fs::write(&path, &text).unwrap();
        let loaded = load_config(&ConfigLocation {
            file: path,
            directory: root.path().to_owned(),
        })
        .unwrap();
        let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
        else {
            panic!()
        };
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("child -> z -> child"));
        let row = serde_json::to_value(&diagnostics[0]).unwrap();
        assert_eq!(
            diagnostics[0].column,
            Some(u32::try_from(text.find("\"child\":").unwrap() + 1).unwrap())
        );
        observed.push(row);
    }
    assert_eq!(observed[0], observed[1]);
}

async fn assert_kind_key_skips_decoys(scenario: &str) {
    for pretty in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::write(root.path());
        let decoy = match scenario {
            "value" => json!({"rule":"skill","fields":{"body":{"from":"inside","kind":"step"}}}),
            "nested_key" => json!({"rule":"skill","fields":{"step":{"from":"first_line"}}}),
            "escaped_string" => {
                json!({"rule":"skill","fields":{"description":{"from":"yaml","key":"see \\\"step\\\" and \\\\ { ["}}})
            }
            "array_value" => json!({"rule":"skill","within":["step"],"fields":{}}),
            _ => panic!(),
        };
        config["evidence"]["kinds"]["a_decoy"] = decoy;
        config["evidence"]["kinds"]["step"] =
            json!({"rule":"skill","fields":{},"within":["skill","missing"]});
        let text = if pretty {
            serde_json::to_string_pretty(&config).unwrap()
        } else {
            serde_json::to_string(&config).unwrap()
        };
        let position = text.rfind("\"step\":").unwrap();
        let prefix = &text[..position];
        let expected_line =
            u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
        let expected_column =
            u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
        let path = root.path().join("sunstack.lint.json");
        std::fs::write(&path, &text).unwrap();
        let config = load_config(&ConfigLocation {
            file: path,
            directory: root.path().to_owned(),
        })
        .unwrap();
        let CheckResult::Invalid { diagnostics } = check(&config, CancellationToken::new()).await
        else {
            panic!()
        };
        let diagnostic = diagnostics
            .iter()
            .find(|row| row.message == "undefined kind missing")
            .unwrap();
        assert_eq!(
            (diagnostic.line, diagnostic.column),
            (Some(expected_line), Some(expected_column)),
            "{scenario}, pretty={pretty}: {diagnostics:?}"
        );
    }
}
#[tokio::test]
async fn kind_key_location_skips_an_earlier_matching_value() {
    assert_kind_key_skips_decoys("value").await;
}
#[tokio::test]
async fn kind_key_location_skips_an_earlier_nested_key() {
    assert_kind_key_skips_decoys("nested_key").await;
}
#[tokio::test]
async fn kind_key_location_skips_escaped_quotes_backslashes_and_braces() {
    assert_kind_key_skips_decoys("escaped_string").await;
}
#[tokio::test]
async fn kind_key_location_skips_an_earlier_within_array_value() {
    assert_kind_key_skips_decoys("array_value").await;
}

#[tokio::test]
async fn kind_locations_preserve_literal_dotted_and_escaped_identities() {
    for kind_name in ["ordinary", "a.b", "a.fields.body", "odd\"\\kind"] {
        for pretty in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let mut value = fixture::write(root.path());
            value["evidence"]["kinds"]["a"] =
                json!({"rule":"skill","fields":{"body":{"from":"first_line"}}});
            value["evidence"]["kinds"][kind_name] =
                json!({"rule":"skill","fields":{},"within":["skill","missing"]});
            let text = if pretty {
                serde_json::to_string_pretty(&value).unwrap()
            } else {
                serde_json::to_string(&value).unwrap()
            };
            let literal_key = format!("{}:", serde_json::to_string(kind_name).unwrap());
            let position = text.find(&literal_key).unwrap();
            let prefix = &text[..position];
            let expected_line =
                u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
            let expected_column =
                u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
            let path = root.path().join("sunstack.lint.json");
            std::fs::write(&path, &text).unwrap();
            let config = load_config(&ConfigLocation {
                file: path,
                directory: root.path().to_owned(),
            })
            .unwrap();
            let CheckResult::Invalid { diagnostics } =
                check(&config, CancellationToken::new()).await
            else {
                panic!()
            };
            let diagnostic = diagnostics
                .iter()
                .find(|row| row.message == "undefined kind missing")
                .unwrap();
            assert_eq!(
                (diagnostic.line, diagnostic.column),
                (Some(expected_line), Some(expected_column)),
                "{kind_name}, pretty={pretty}: {diagnostics:?}"
            );
        }
    }
}

#[tokio::test]
async fn field_locations_preserve_the_owner_across_kind_prefix_collisions() {
    for (pretty, reverse_order) in [(false, false), (false, true), (true, false), (true, true)] {
        let root = tempfile::tempdir().unwrap();
        let mut value = fixture::write(root.path());
        value["evidence"]["kinds"]["a"] =
            json!({"rule":"skill","fields":{},"within":["foo.fields.body"]});
        value["evidence"]["kinds"]["foo.fields.body"] = json!({"rule":"skill","fields":{}});
        value["evidence"]["kinds"]["a.within.foo"] =
            json!({"rule":"skill","fields":{"body":{"from":"inside","kind":"missing"}}});
        let mut entries = value["evidence"]["kinds"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, definition)| {
                let definition = if pretty {
                    serde_json::to_string_pretty(definition).unwrap()
                } else {
                    serde_json::to_string(definition).unwrap()
                };
                format!("{}: {}", serde_json::to_string(name).unwrap(), definition)
            })
            .collect::<Vec<_>>();
        if reverse_order {
            entries.reverse();
        }
        let kinds_text = format!("{{{}}}", entries.join(if pretty { ",\n" } else { "," }));
        value["evidence"]["kinds"] = json!("KINDS_PLACEHOLDER");
        let text = if pretty {
            serde_json::to_string_pretty(&value).unwrap()
        } else {
            serde_json::to_string(&value).unwrap()
        };
        let text = text.replace("\"KINDS_PLACEHOLDER\"", &kinds_text);
        let owner = text.find("\"a.within.foo\":").unwrap();
        let position = owner + text[owner..].find("\"body\":").unwrap();
        let prefix = &text[..position];
        let expected_line =
            u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
        let expected_column =
            u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
        let path = root.path().join("sunstack.lint.json");
        std::fs::write(&path, &text).unwrap();
        let config = load_config(&ConfigLocation {
            file: path,
            directory: root.path().to_owned(),
        })
        .unwrap();
        let CheckResult::Invalid { diagnostics } = check(&config, CancellationToken::new()).await
        else {
            panic!()
        };
        let diagnostic = diagnostics
            .iter()
            .find(|row| row.message == "undefined evidence kind")
            .unwrap();
        assert_eq!(
            (diagnostic.line, diagnostic.column),
            (Some(expected_line), Some(expected_column)),
            "pretty={pretty}, reverse={reverse_order}: {diagnostics:?}"
        );
    }
}

#[test]
fn local_evidence_limit_refusal_names_its_full_path_and_actual_key_location() {
    for pretty in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut value = fixture::write(root.path());
        value["limits"] = json!({"evidence_max_chars":10});
        let text = if pretty {
            serde_json::to_string_pretty(&value).unwrap()
        } else {
            serde_json::to_string(&value).unwrap()
        };
        let position = text.find("\"evidence_max_chars\"").unwrap();
        let prefix = &text[..position];
        let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
        let column =
            u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
        let path = root.path().join("sunstack.lint.json");
        std::fs::write(&path, &text).unwrap();
        let error = load_config(&ConfigLocation {
            file: path,
            directory: root.path().to_owned(),
        })
        .unwrap_err();
        assert_eq!(error.json_path, ["limits.evidence_max_chars"]);
        assert_eq!(error.message, "evidence_max_chars must be at least 1000");
        assert_eq!(
            (error.line, error.column),
            (Some(line), Some(column)),
            "pretty={pretty}"
        );
    }
}

fn assert_local_shape_path(setting: &str, public_path: &str, key: &str) {
    for pretty in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut value = fixture::write(root.path());
        match setting {
            "check.files" => value["check"]["files"] = json!([]),
            "evidence.kinds" => value["evidence"]["kinds"] = json!({}),
            "evidence.ast_grep" => value["evidence"]["ast_grep"] = json!(""),
            "pair.markdown" => {
                value["rulebook"] =
                    json!([{"markdown":"m","rules":"r"},{"markdown":"","rules":"r"}])
            }
            "pair.rules" => {
                value["rulebook"] =
                    json!([{"markdown":"m","rules":"r"},{"markdown":"m","rules":""}])
            }
            _ => panic!(),
        }
        let text = if pretty {
            serde_json::to_string_pretty(&value).unwrap()
        } else {
            serde_json::to_string(&value).unwrap()
        };
        let position = text.rfind(&format!("\"{key}\":")).unwrap();
        let prefix = &text[..position];
        let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1).unwrap();
        let column =
            u32::try_from(prefix.rsplit('\n').next().unwrap().chars().count() + 1).unwrap();
        let path = root.path().join("sunstack.lint.json");
        std::fs::write(&path, &text).unwrap();
        let error = load_config(&ConfigLocation {
            file: path,
            directory: root.path().to_owned(),
        })
        .unwrap_err();
        assert_eq!(error.json_path, [public_path], "{setting}, pretty={pretty}");
        assert_eq!(
            (error.line, error.column),
            (Some(line), Some(column)),
            "{setting}, pretty={pretty}"
        );
    }
}
#[test]
fn empty_checked_files_names_its_full_path() {
    assert_local_shape_path("check.files", "check.files", "files");
}
#[test]
fn empty_kind_registry_names_its_full_path() {
    assert_local_shape_path("evidence.kinds", "evidence.kinds", "kinds");
}
#[test]
fn empty_evidence_project_names_its_full_path() {
    assert_local_shape_path("evidence.ast_grep", "evidence.ast_grep", "ast_grep");
}
#[test]
fn empty_markdown_path_names_its_indexed_pair() {
    assert_local_shape_path("pair.markdown", "rulebook.1.markdown", "markdown");
}
#[test]
fn empty_rules_path_names_its_indexed_pair() {
    assert_local_shape_path("pair.rules", "rulebook.1.rules", "rules");
}

#[test]
fn local_shape_refusals_choose_the_first_failing_setting_deterministically() {
    let root = tempfile::tempdir().unwrap();
    let mut value = fixture::write(root.path());
    value["rulebook"] = json!([]);
    value["check"]["files"] = json!([]);
    value["evidence"]["kinds"] = json!({});
    for (expected, message) in [
        (
            "rulebook",
            "rulebook, check.files and evidence.kinds must be nonempty",
        ),
        (
            "check.files",
            "rulebook, check.files and evidence.kinds must be nonempty",
        ),
        (
            "evidence.kinds",
            "rulebook, check.files and evidence.kinds must be nonempty",
        ),
        ("rulebook.0.markdown", "configured paths must be nonempty"),
        ("rulebook.0.rules", "configured paths must be nonempty"),
        ("evidence.ast_grep", "configured paths must be nonempty"),
    ] {
        fixture::config_file(root.path(), &value);
        let error = load_config(&ConfigLocation {
            file: root.path().join("sunstack.lint.json"),
            directory: root.path().to_owned(),
        })
        .unwrap_err();
        assert_eq!(error.json_path, [expected]);
        assert_eq!(error.message, message);
        match expected {
            "rulebook" => value["rulebook"] = json!([{"markdown":"","rules":""}]),
            "check.files" => value["check"]["files"] = json!(["docs/*.md"]),
            "evidence.kinds" => {
                value["evidence"]["kinds"] = json!({"skill":{"rule":"skill","fields":{}}});
                value["evidence"]["ast_grep"] = json!("");
            }
            "rulebook.0.markdown" => value["rulebook"][0]["markdown"] = json!("rulebook.md"),
            "rulebook.0.rules" => value["rulebook"][0]["rules"] = json!("rules.json"),
            "evidence.ast_grep" => {}
            _ => panic!(),
        }
    }
}

#![allow(clippy::unwrap_used, clippy::panic)]
use crate::test_support::lint_fixture as fixture;
use crate::*;
use serde_json::json;

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

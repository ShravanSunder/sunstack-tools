#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/change_fixture.rs"]
mod changes;
#[path = "support/rule_source_fixture.rs"]
mod rule_source_fixture;
#[path = "support/rule_transport.rs"]
mod rule_transport;
mod support;
use rule_source_fixture::{runtime_input, source_fixture};
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use serde_json::{Value, json};
use support::*;

#[tokio::test]
async fn connected_checks_admit_discovery_settings_without_acquiring_reviewed_content() {
    let (directory, mut input) = source_fixture();
    let root = directory.path();
    let mut header: Value =
        serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
    header["inputs"]["changes"] = json!({"kind":"change_set"});
    std::fs::write(root.join("set.json"), serde_json::to_vec(&header).unwrap()).unwrap();
    std::fs::write(root.join("teaching.rules.json"), serde_json::to_vec_pretty(&json!({"K014":{"name":"connected.valid","target":{"input":"changes","select":{"kind":"changed_units","languages":["typescript"],"unit_kinds":["function"],"source_sides":"after_only"}},"threshold":0.8}})).unwrap()).unwrap();
    input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["changes"] =
        json!({"kind":"change_set","repository":"repo"});
    input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
    assert!(!root.join("missing-reviewed.md").exists());
    assert!(!root.join(".git").exists());
    let checked = response(
        &run_without_credential(root, "check_rules", input.clone()).await,
        0,
    );
    assert_eq!(checked["result"]["outcome"]["checked_rule_count"], 1);
    assert_eq!(
        checked["result"]["outcome"]["printed_rules"][0]["definition"]["evidence"][0]["select"]["kind"],
        "changed_units"
    );
    input.as_object_mut().unwrap().remove("discovery");
    let refused = response(&run_without_credential(root, "check_rules", input).await, 2);
    let diagnostic = &refused["result"]["outcome"]["diagnostics"]["items"][0];
    assert_eq!(diagnostic["code"], "discovery_not_enabled");
    assert_eq!(
        diagnostic["location"]["file"],
        json!(root.canonicalize().unwrap().join("teaching.rules.json"))
    );
    assert_eq!(diagnostic["location"]["line"], 2);
    assert_eq!(diagnostic["location"]["column"], 11);
}

#[tokio::test]
async fn invalid_connected_entries_skip_locally_but_equivalent_jsonl_remains_strict() {
    for context in [false, true] {
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.9,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let (directory, input) = source_fixture();
        let root = directory.path();
        git2::Repository::init(root).unwrap();
        std::fs::write(root.join("missing-reviewed.md"), "U001: surviving target\n").unwrap();
        let mut header: Value =
            serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
        header["inputs"]["changes"] = json!({"kind":"change_set"});
        std::fs::write(root.join("set.json"), serde_json::to_vec(&header).unwrap()).unwrap();
        std::fs::write(root.join("teaching.md"), "Good standard<!-- K014 -->\n").unwrap();
        let mut input = runtime_input(input, &server.endpoint);
        input["definitions"]["rule_sets"][0]["binding_groups"][0]["bindings"]["changes"] =
            json!({"kind":"change_set","repository":"repo"});
        input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
        let baseline = response(&run(root, "evaluate_rules", input.clone()).await, 0);
        let baseline_report = &baseline["result"]["outcome"]["report"];
        let baseline_judgment = baseline_report["occurrences"][0]["judgment"].clone();
        let mut entries: Value =
            serde_json::from_slice(&std::fs::read(root.join("teaching.rules.json")).unwrap())
                .unwrap();
        let selector = json!({"kind":"changed_units","languages":[],"unit_kinds":["function"],"source_sides":"after_only"});
        let mut bad = json!({"name":"bad.connected","target":{"input":"changes","select":selector},"threshold":0.8});
        if context {
            bad["target"] = entries["K014"]["target"].clone();
            bad["context"] = json!([{"id":"extra","input":"changes","select":selector}]);
        }
        entries["K015"] = bad;
        std::fs::write(
            root.join("teaching.md"),
            "Good standard<!-- K014 -->\n\nBad connected standard<!-- K015 -->\n",
        )
        .unwrap();
        std::fs::write(
            root.join("teaching.rules.json"),
            serde_json::to_vec_pretty(&entries).unwrap(),
        )
        .unwrap();
        let before_prepare = server.count();
        let prepared = response(&run(root, "prepare_rules", input.clone()).await, 0);
        assert_eq!(server.count(), before_prepare);
        assert_eq!(
            prepared["result"]["outcome"]["preparation"]["rule_coverage"],
            "partial"
        );
        let partial = response(&run(root, "evaluate_rules", input.clone()).await, 0);
        let report = &partial["result"]["outcome"]["report"];
        assert_eq!(report["rule_coverage"], "partial");
        assert_eq!(report["occurrences"].as_array().unwrap().len(), 1);
        assert_eq!(report["occurrences"][0]["judgment"], baseline_judgment);
        assert_eq!(report["skipped_rules"].as_array().unwrap().len(), 1);
        let skipped = &report["skipped_rules"][0];
        assert_eq!(skipped["kind"], "entry");
        assert_eq!(skipped["key"], "K015");
        let diagnostic = &skipped["diagnostics"]["items"][0];
        assert_eq!(diagnostic["code"], "invalid_selector");
        assert_eq!(
            diagnostic["location"]["file"],
            json!(root.canonicalize().unwrap().join("teaching.rules.json"))
        );
        assert!(diagnostic["location"]["line"].as_u64().unwrap() > 1);
        assert!(diagnostic["location"]["column"].as_u64().unwrap() > 1);
        let strict = json!({"rule_id":"bad.jsonl","evidence":[{"id":"target","input":"changes","select":selector}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"Bad selector"},"threshold":0.8}});
        std::fs::write(
            root.join("strict.jsonl"),
            serde_json::to_vec(&strict).unwrap(),
        )
        .unwrap();
        header["rule_paths"] = json!(["strict.jsonl"]);
        std::fs::write(root.join("set.json"), serde_json::to_vec(&header).unwrap()).unwrap();
        for operation in ["prepare_rules", "evaluate_rules"] {
            let before = server.count();
            let refusal = response(&run(root, operation, input.clone()).await, 2);
            assert_eq!(refusal["result"]["outcome"]["kind"], "invalid");
            assert_eq!(server.count(), before);
        }
        server.finish().await;
    }
}

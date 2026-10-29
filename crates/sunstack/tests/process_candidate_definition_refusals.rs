#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/document_candidate_fixture.rs"]
mod document_fixture;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use serde_json::{Value, json};
use transport::{AnswerServer, ReplyMode, RouterReply};

fn write_json(path: &std::path::Path, value: &Value) {
    std::fs::write(path, value.to_string()).unwrap();
}
fn read_json(path: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn marked_entry() -> Value {
    json!({"name":"standard.valid","target":{"input":"subject","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}},"threshold":0.8})
}
fn configure(root: &std::path::Path, valid: bool, orphan: bool) {
    let mut set = read_json(&root.join("set.json"));
    set.as_object_mut().unwrap().remove("rule_paths");
    set["marker_prefix"] = json!("K");
    set["rule_sources"] = json!([{"markdown":"models.md","rules":"models.rules.json"}]);
    write_json(&root.join("set.json"), &set);
    std::fs::write(
        root.join("models.md"),
        if valid {
            "## Keep words clear<!-- K001 -->\n\nA standard is supplied.\n"
        } else {
            "No marked standards.\n"
        },
    )
    .unwrap();
    let mut entries = json!({});
    if valid {
        entries["K001"] = marked_entry();
    }
    if orphan {
        let mut entry = marked_entry();
        entry["name"] = json!("standard.orphan");
        entries["K999"] = entry;
    }
    write_json(&root.join("models.rules.json"), &entries);
}

// All skips, a mixed tree, source failures and late/unselected sets must each refuse before work.
#[tokio::test]
async fn every_configured_skipped_definition_is_refused_with_its_own_located_diagnostic() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    for variant in [
        "all_entries",
        "mixed_entries",
        "source_failure",
        "both_causes",
        "later_set",
        "other_route",
    ] {
        let (repository, mut input, _, _) = document_fixture::fixture(
            &server.endpoint,
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            None,
        );
        let root = repository.root.path();
        configure(
            root,
            variant != "all_entries",
            matches!(variant, "all_entries" | "mixed_entries" | "both_causes"),
        );
        let mut expected = Vec::new();
        if matches!(variant, "all_entries" | "mixed_entries" | "both_causes") {
            expected.push(("models.rules.json", "orphaned_rule_entry", json!(["K999"])));
        }
        if matches!(variant, "source_failure" | "both_causes") {
            std::fs::write(root.join("broken.md"), "## Broken<!-- K002 -->\n").unwrap();
            std::fs::write(root.join("broken.rules.json"), "{broken json").unwrap();
            let mut set = read_json(&root.join("set.json"));
            set["rule_sources"]
                .as_array_mut()
                .unwrap()
                .push(json!({"markdown":"broken.md","rules":"broken.rules.json"}));
            write_json(&root.join("set.json"), &set);
            expected.push(("broken.rules.json", "malformed_json", json!([])));
        }
        if matches!(variant, "later_set" | "other_route") {
            let mut bad = read_json(&root.join("set.json"));
            bad["name"] = json!("bad standards");
            bad["rule_sources"] =
                json!([{"markdown":"badmodels.md","rules":"badmodels.rules.json"}]);
            write_json(&root.join("badset.json"), &bad);
            std::fs::write(root.join("badmodels.md"), "No marked standards.\n").unwrap();
            write_json(
                &root.join("badmodels.rules.json"),
                &json!({"K999":marked_entry()}),
            );
            expected.push((
                "badmodels.rules.json",
                "orphaned_rule_entry",
                json!(["K999"]),
            ));
            input["definitions"] = if variant == "later_set" {
                json!({"kind":"rule_sets","rule_sets":["set.json","badset.json"]})
            } else {
                let router = json!({"schema_version":1,"name":"choose valid","description":"The invalid set is reachable but not selected","inputs":{"subject":{"kind":"document"}},"evidence":[{"id":"route_context","input":"subject","select":{"kind":"whole_document"}}],"question":{"type":"choice","instructions":"Route the candidate","criteria":{"review_handback":"good","other":"bad"}},"routes":{"review_handback":[{"rule_set":"set.json","inputs":{"subject":"subject"}}],"other":[{"rule_set":"badset.json","inputs":{"subject":"subject"}}]},"confidence_floor":0.6});
                write_json(&root.join("router.json"), &router);
                json!({"kind":"router","path":"router.json"})
            };
        }
        let output = support::run(root, "evaluate_candidates", input).await;
        let wire = document_fixture::candidate_response(&output, 2);
        let outcome = &wire["result"]["outcome"];
        assert_eq!(outcome["kind"], "invalid", "{variant}: {wire}");
        assert!(outcome.get("report").is_none());
        let diagnostics = outcome["diagnostics"]["items"].as_array().unwrap();
        assert_eq!(diagnostics.len(), expected.len(), "{variant}: {wire}");
        for (file, code, field) in expected {
            let path = root.join(file).canonicalize().unwrap();
            let diagnostic = diagnostics
                .iter()
                .find(|item| {
                    item["code"] == code
                        && item["location"]["file"] == json!(path)
                        && item["location"]["field_path"] == field
                })
                .unwrap();
            assert!(diagnostic["location"]["line"].as_u64().unwrap() >= 1);
            assert!(diagnostic["location"]["column"].as_u64().unwrap() >= 1);
        }
        assert_eq!(
            server.count(),
            0,
            "definition refusal must precede dispatch: {variant}"
        );
    }
    assert_eq!(server.finish().await.0, 0);
}

// A refused definition is different from valid work or a genuinely nonmatching pass.
#[tokio::test]
async fn valid_marked_models_dispatch_and_a_real_nonmatching_pass_can_find_nothing() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let (repository, input, _, _) = document_fixture::fixture(
        &server.endpoint,
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
        None,
    );
    let root = repository.root.path();
    configure(root, true, false);
    let output = support::run(root, "evaluate_candidates", input.clone()).await;
    let wire = document_fixture::candidate_response(&output, 0);
    let entries = wire["result"]["outcome"]["report"]["candidates"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 3);
    assert!(
        entries
            .iter()
            .all(|entry| entry["rule"]["rule_id"] == "standard.valid"
                && entry["outcome"]["kind"] == "dropped"
                && entry["answers"][0]["answer"]["yes_probability"] == 0.9)
    );
    assert_eq!(server.count(), 3);
    let mut models = read_json(&root.join("models.rules.json"));
    models["K001"]["mechanical_pass"] = json!({"predicate":{"kind":"literal","field":"/text","text":"__ABSENT_FROM_ALL_PARAGRAPHS__"}});
    write_json(&root.join("models.rules.json"), &models);
    let output = support::run(root, "evaluate_candidates", input).await;
    let wire = document_fixture::candidate_response(&output, 0);
    assert_eq!(
        wire["result"]["outcome"]["report"]["completeness"],
        json!({"kind":"nothing_found"})
    );
    assert!(
        wire["result"]["outcome"]["report"]["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(server.finish().await.0, 3);
}

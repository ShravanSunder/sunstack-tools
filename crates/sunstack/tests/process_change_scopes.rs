#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/rule_transport.rs"]
mod rule_transport;
#[path = "../../sunstack-rules/tests/support/scope_fixture.rs"]
mod scope_fixture;
mod support;
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use scope_fixture::{GATE_TEXT, ScopeFixture};
use serde_json::json;
use support::*;

#[tokio::test]
async fn real_process_selects_nonempty_union_after_bytes_from_each_relative_git_cwd() {
    for marker in ["SELECTED_PROCESS_SENTINEL_A", "SELECTED_PROCESS_SENTINEL_B"] {
        let fixture = ScopeFixture::new();
        let root = fixture.repository.root.path();
        fixture
            .repository
            .write("source.md", "U001: staged intermediate\n");
        fixture.repository.stage("source.md");
        fixture
            .repository
            .write("source.md", &format!("U001: {marker}\n"));
        let mut input = evidence_input(root);
        input["repositories"][0]["root"] = json!(".");
        input["repositories"][0]["change_scope"] = json!({"kind":"union","members":[{"kind":"uncommitted","layers":["staged"]},{"kind":"uncommitted","layers":["unstaged"]}]});
        input["sources"] = json!({"changes":{"kind":"change_set","repository":"repo"}});
        input["discovery"] = serde_json::to_value(scope_fixture::changes::discovery()).unwrap();
        input["declarations"] = json!([{"id":"changed","input":"changes","requirement":"required","select":{"kind":"change_set_literal_text","paths":[{"kind":"exact","path":"source.md"}],"text":marker,"sides":["before_file","after_file"]}}]);
        let output = run(root, "prepare_evidence", input.clone()).await;
        let wire = response(&output, 0);
        assert_eq!(wire["protocol_version"], 1);
        assert_eq!(wire["result"]["operation"], "prepare_evidence");
        let outcome = &wire["result"]["outcome"];
        assert_eq!(outcome["kind"], "prepared");
        let matches = outcome["selections"][0]["readiness"]["content"]["matches"]
            .as_array()
            .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["text"], marker);
        let source = &matches[0]["span"]["source"];
        assert_eq!(source["repository"], "repo");
        assert_eq!(source["path"], "source.md");
        assert_eq!(source["revision"]["kind"], "worktree");
        assert_eq!(source["revision"]["side"], "after");
        let selections = outcome["change_selections"].as_array().unwrap();
        assert_eq!(selections.len(), 1);
        let members = selections[0]["members"].as_array().unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(
            members[0]["bounds"],
            json!({"kind":"uncommitted","layers":["staged"]})
        );
        assert_eq!(
            members[1]["bounds"],
            json!({"kind":"uncommitted","layers":["unstaged"]})
        );
        assert_eq!(&members[1]["changed_paths"][0]["after"], source);
        assert_eq!(
            members[1]["changed_paths"][0]["provenance"],
            json!({"kind":"dirty_layer","layer":"unstaged"})
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains(marker));
        input["discovery"] = json!({"kind":"none"});
        let invalid = response(&run(root, "prepare_evidence", input).await, 2);
        assert_eq!(invalid["result"]["outcome"]["kind"], "refused");
        assert_eq!(
            invalid["result"]["outcome"]["diagnostics"]["items"][0]["code"],
            "discovery_not_enabled"
        );
    }
}
#[tokio::test]
async fn full_document_remains_valid_while_full_changed_units_has_no_change_selection() {
    let fixture = ScopeFixture::new();
    let root = fixture.repository.root.path();
    std::fs::write(root.join("skill.md"), "U001: full control\n").unwrap();
    let mut input = evidence_input(root);
    let wire = response(&run(root, "prepare_evidence", input.clone()).await, 0);
    assert_eq!(wire["result"]["outcome"]["change_selections"], json!([]));
    input["sources"] = json!({"skill":{"kind":"change_set","repository":"repo"}});
    input["discovery"] = serde_json::to_value(scope_fixture::changes::discovery()).unwrap();
    input["declarations"][0]["select"] = json!({"kind":"changed_units","languages":["markdown"],"unit_kinds":["paragraph"],"source_sides":"after_only"});
    let wire = response(&run(root, "prepare_evidence", input).await, 3);
    assert_eq!(wire["result"]["outcome"]["kind"], "incomplete");
    assert_eq!(
        wire["result"]["outcome"]["selections"][0]["readiness"]["diagnostics"]["items"][0]["code"],
        "no_change_selection"
    );
}
#[tokio::test]
async fn mixed_process_scope_preserves_exact_selection_reasons_and_http_owner() {
    let fixture = ScopeFixture::new();
    let root = fixture.repository.root.path();
    fixture.repository.write(
        "source.md",
        "U001: changed process target\n\nU002: neighbor\n",
    );
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let mut input = serde_json::to_value(fixture.input(&server.endpoint)).unwrap();
    input["definition_root"] = json!(".");
    input["repositories"][0]["root"] = json!(".");
    let prepared = response(&run(root, "prepare_rules", input.clone()).await, 0);
    let preparation = &prepared["result"]["outcome"]["preparation"];
    assert_eq!(
        preparation["occurrences"][0]["status"],
        json!({"kind":"planned","selection":{"kind":"selected","selected_by":["target_changed"]}})
    );
    assert_eq!(
        preparation["occurrences"][1]["status"],
        json!({"kind":"outside_change_scope","reason":"no_declared_change"})
    );
    assert_eq!(
        preparation["planned_requests"][0]["questions"],
        json!([{"request_name":"q000001","owner":{"kind":"verdict","key":{"rule_set":root.canonicalize().unwrap().join("set.json"),"group":0,"rule_id":"changed","target":{"kind":"legacy"}}}}])
    );
    assert_eq!(server.count(), 0);
    let wire = response(&run(root, "evaluate_rules", input).await, 0);
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(
        report["occurrences"][0]["selected_by"],
        json!(["target_changed"])
    );
    assert_eq!(
        report["occurrences"][1],
        json!({"kind":"outside_change_scope","key":{"rule_set":root.canonicalize().unwrap().join("set.json"),"group":0,"rule_id":"neighbor","target":{"kind":"legacy"}},"reason":"no_declared_change"})
    );
    let source = &report["occurrences"][0]["judgment"]["target"]["span"]["source"];
    let after = &report["change_selections"][0]["members"][0]["changed_paths"][0]["after"];
    assert_eq!(source, after);
    assert_eq!(
        preparation["change_selections"][0]["members"][0]["changed_paths"][0]["after"]["content_identity"],
        after["content_identity"]
    );
    let (count, bodies) = server.finish().await;
    assert_eq!(count, 1);
    assert_eq!(bodies[0]["questions"].as_object().unwrap().len(), 1);
    assert_eq!(
        bodies[0]["questions"]["q000001"]["instructions"]["question"],
        "judge changed"
    );
    assert_eq!(
        bodies[0]["state"]["evidence"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["rule:changed/target"]
    );
    record_observation(
        "m2-mixed-scope",
        json!({"preparation":prepared,"evaluation":wire,"http":bodies,"connections":count}),
    );
}
#[tokio::test]
async fn moved_process_paragraphs_are_pruned_with_outside_not_sent_and_no_http() {
    let fixture = ScopeFixture::new();
    fixture.repository.write(
        "source.md",
        "# Prefix\n\nU001: original\n\nU002: neighbor\n",
    );
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let input = serde_json::to_value(fixture.input(&server.endpoint)).unwrap();
    let prepared = response(
        &run(
            fixture.repository.root.path(),
            "prepare_rules",
            input.clone(),
        )
        .await,
        0,
    );
    let preparation = &prepared["result"]["outcome"]["preparation"];
    assert_eq!(preparation["occurrences"].as_array().unwrap().len(), 2);
    assert_eq!(preparation["planned_requests"], json!([]));
    assert!(
        preparation["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|occurrence| occurrence["status"]
                == json!({"kind":"outside_change_scope","reason":"no_declared_change"}))
    );
    let evaluated = response(
        &run(fixture.repository.root.path(), "evaluate_rules", input).await,
        0,
    );
    let report = &evaluated["result"]["outcome"]["report"];
    assert_eq!(report["occurrences"].as_array().unwrap().len(), 2);
    assert!(
        report["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .all(|occurrence| occurrence["kind"] == "outside_change_scope")
    );
    assert_eq!(
        report["requests"][0]["status"],
        json!({"kind":"not_sent","reason":{"kind":"outside_change_scope"}})
    );
    assert_eq!(server.finish().await.0, 0);
}
#[tokio::test]
async fn process_gate_frontier_stays_pending_until_nonpass_equality_change_or_refusal() {
    for (probability, text, kind, exit, count) in [
        (
            0.1,
            "# Prefix\n\nU001: original\n\nU002: neighbor\n",
            "not_applicable",
            0,
            1,
        ),
        (
            0.9,
            "# Prefix\n\nU001: original\n\nU002: neighbor\n",
            "outside_change_scope",
            0,
            1,
        ),
        (0.9, "U001: changed\n\nU002: neighbor\n", "judged", 0, 2),
        (
            0.9,
            "U001: duplicate A\n\nU001: duplicate B\n\nU002: neighbor\n",
            "selection_failed",
            3,
            1,
        ),
    ] {
        let fixture = ScopeFixture::new();
        let server = AnswerServer::start(ReplyMode::Answers {
            probability,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let input = serde_json::to_value(fixture.add_gate(&server.endpoint)).unwrap();
        fixture.repository.write("source.md", text);
        let prepared = response(
            &run(
                fixture.repository.root.path(),
                "prepare_rules",
                input.clone(),
            )
            .await,
            0,
        );
        let preparation = &prepared["result"]["outcome"]["preparation"];
        assert_eq!(preparation["occurrences"].as_array().unwrap().len(), 2);
        assert!(
            preparation["occurrences"]
                .as_array()
                .unwrap()
                .iter()
                .all(|occurrence| occurrence["status"]
                    == json!({"kind":"planned","selection":{"kind":"comparison_pending"}}))
        );
        assert_eq!(
            preparation["measurements"]["bytes_scanned"],
            GATE_TEXT.len()
        );
        assert!(
            preparation["planned_requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|request| request["readiness"]["kind"] == "waiting"
                    && !request["readiness"]["on"].as_array().unwrap().is_empty())
        );
        assert_eq!(server.count(), 0);
        let evaluated = response(
            &run(fixture.repository.root.path(), "evaluate_rules", input).await,
            exit,
        );
        let report = &evaluated["result"]["outcome"]["report"];
        assert_eq!(report["occurrences"][0]["kind"], kind);
        if probability < 0.6 {
            assert_eq!(report["measurements"]["bytes_scanned"], GATE_TEXT.len());
        }
        if kind == "outside_change_scope" {
            assert!(
                report["requests"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|request| request["status"]
                        == json!({"kind":"not_sent","reason":{"kind":"outside_change_scope"}}))
            );
        }
        if kind == "judged" {
            assert_eq!(
                report["occurrences"][0]["selected_by"],
                json!(["target_changed"])
            );
        }
        assert_eq!(server.finish().await.0, count);
    }
}
#[tokio::test]
async fn ready_duplicate_target_is_native_execution_refusal_not_protocol_failure() {
    let fixture = ScopeFixture::new();
    fixture.repository.write(
        "source.md",
        "U001: duplicate A\n\nU001: duplicate B\n\nU002: neighbor\n",
    );
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let input = serde_json::to_value(fixture.input(&server.endpoint)).unwrap();
    for (operation, outcome) in [
        ("prepare_rules", "prepared_with_failures"),
        ("evaluate_rules", "incomplete_report"),
    ] {
        let wire = response(
            &run(fixture.repository.root.path(), operation, input.clone()).await,
            3,
        );
        assert_eq!(wire["kind"], "operation_result");
        assert_eq!(wire["result"]["operation"], operation);
        assert_eq!(wire["result"]["outcome"]["kind"], outcome);
    }
    assert_eq!(server.finish().await.0, 0);
}

#[tokio::test]
async fn closed_empty_milestone_two_input_variants_refuse_through_the_real_process() {
    let fixture = ScopeFixture::new();
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    for variant in ["full", "worktree", "none"] {
        let mut input = serde_json::to_value(fixture.input(&server.endpoint)).unwrap();
        match variant {
            "full" => {
                input["repositories"][0]["change_scope"] =
                    json!({"kind":"full","AUTHOR_SECRET_KEY":true})
            }
            "worktree" => {
                input["repositories"][0]["change_scope"] = json!({"kind":"revision_diff","base_revision":"HEAD","head":{"kind":"worktree","AUTHOR_SECRET_KEY":true}})
            }
            "none" => input["discovery"] = json!({"kind":"none","AUTHOR_SECRET_KEY":true}),
            _ => panic!("variant"),
        }
        for operation in ["prepare_rules", "evaluate_rules"] {
            let output = run(fixture.repository.root.path(), operation, input.clone()).await;
            let wire = response(&output, 2);
            assert_eq!(wire["kind"], "operation_result");
            assert_eq!(wire["result"]["operation"], operation);
            assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
            assert_eq!(
                wire["result"]["outcome"]["diagnostics"]["items"][0]["code"],
                "unknown_field"
            );
            assert!(!String::from_utf8_lossy(&output.stdout).contains("AUTHOR_SECRET_KEY"));
        }
    }
    assert_eq!(server.finish().await.0, 0);
}

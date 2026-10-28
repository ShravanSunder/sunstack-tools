#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/change_fixture.rs"]
mod change_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
#[path = "support/candidate_evidence_fixture.rs"]
mod evidence_fixture;
mod support;
use controlled_http::ControlledServer;
use evidence_fixture::{fixture, report};
use serde_json::{Value, json};

fn revision_fixture(endpoint: &str) -> (change_fixture::ChangeFixture, Value) {
    let repository = change_fixture::ChangeFixture::new();
    repository.write(
        "code.rs",
        "fn example() -> &'static str { \"BEFORE_FACT\" }\n",
    );
    repository.stage("code.rs");
    let before = repository.commit();
    repository.write(
        "code.rs",
        "fn example() -> &'static str { \"COMMITTED_AFTER_FACT\" }\n",
    );
    repository.stage("code.rs");
    let after = repository.commit();
    repository.write(
        "code.rs",
        "fn example() -> &'static str { \"DIRTY_WORKTREE_FACT\" }\n",
    );
    let root = repository.root.path();
    let mut input = fixture(root, endpoint, false);
    let path = root.join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["inputs"]["fact"] = json!({"kind":"change_set"});
    std::fs::write(path, set.to_string()).unwrap();
    let path = root.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["evidence"][1]["select"] = json!({"kind":"changed_units","languages":["rust"],"unit_kinds":["function"],"source_sides":"after_only"});
    std::fs::write(path, rule.to_string()).unwrap();
    input["payload_schemas"] = json!([]);
    input["repositories"] = json!([{"key":"facts","root":root,"input_paths":[{"kind":"exact","path":"code.rs"}],"change_scope":{"kind":"revision_diff","base_revision":before,"head":{"kind":"commit","revision":after}}}]);
    input["evidence"] =
        json!({"fact":{"kind":"bound","binding":{"kind":"change_set","repository":"facts"}}});
    (repository, input)
}

#[tokio::test]
async fn a_nonexistent_repository_revision_is_unknown_for_its_question() {
    let server = ControlledServer::start().await;
    let (repository, mut input) = revision_fixture(&server.endpoint);
    input["repositories"][0]["change_scope"]["base_revision"] = json!("missing-revision-for-proof");
    input["discovery"] = serde_json::to_value(change_fixture::discovery()).unwrap();
    let outcome = report(
        &support::run_without_credential(repository.root.path(), "evaluate_candidates", input)
            .await,
        0,
    );
    let entry = &outcome["report"]["candidates"][0];
    assert_eq!(entry["outcome"]["kind"], "unresolved");
    assert_eq!(entry["outcome"]["missing"]["kind"], "evidence_not_captured");
    assert_eq!(entry["outcome"]["missing"]["evidence"], "fact");
    assert_eq!(
        entry["outcome"]["missing"]["diagnostic"],
        "revision_unavailable"
    );
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn repository_facts_require_an_explicit_discovery_capability() {
    let server = ControlledServer::start().await;
    let (repository, input) = revision_fixture(&server.endpoint);
    let outcome = report(
        &support::run_without_credential(repository.root.path(), "evaluate_candidates", input)
            .await,
        2,
    );
    assert_eq!(outcome["kind"], "invalid");
    assert!(
        outcome["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "discovery_not_enabled"
                && item["location"]["field_path"] == json!(["discovery"]))
    );
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn repository_facts_read_the_declared_revision_instead_of_the_dirty_worktree() {
    let mut server = ControlledServer::start().await;
    let (repository, mut input) = revision_fixture(&server.endpoint);
    input["discovery"] = serde_json::to_value(change_fixture::discovery()).unwrap();
    let directory = repository.root.path().to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    let request = tokio::select! {
        output = &mut run => panic!("valid revision was refused: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
        request = server.next() => request,
    };
    let body = request.body.to_string();
    assert!(body.contains("COMMITTED_AFTER_FACT"));
    assert!(!body.contains("DIRTY_WORKTREE_FACT"));
    request.noul(0.9);
    let outcome = report(&run.await.unwrap(), 0);
    assert_eq!(
        outcome["report"]["candidates"][0]["outcome"]["kind"],
        "finding"
    );
    assert!(outcome["report"]["candidates"][0]["evidence"][0]["content_identity"].is_string());
    assert_eq!(server.finish().await, 1);
}

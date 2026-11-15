use super::*;
use serde_json::Value;
use std::{path::Path, time::Duration};

fn paragraph_project(root: &Path, endpoint: &str) {
    let mut config = fixture::judged(root, endpoint);
    config["evidence"]["kinds"] = json!({
        "paragraph": {"rule":"paragraph","fields":{"body":{"from":"remainder"}}}
    });
    fixture::config_file(root, &config);
    std::fs::write(
        root.join("evidence/rules/skill.yml"),
        "id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    std::fs::write(root.join("docs/one.md"), "SELECTED café\n\nEXCLUDED β\n").unwrap();
    fixture::entry_file(
        root,
        &json!({"K001":{
            "name":"body","evidence":"paragraph","severity":"error",
            "questions":{"complies":{"type":"noul","instructions":{
                "check":"body","task":"Does this exact paragraph comply?"
            }}},
            "decision":{"start":"complies","steps":{"complies":{
                "ask":"complies","yes_at_or_above":0.8,"no_at_or_below":0.3,
                "yes":"pass","no":{"finding":"Paragraph does not comply."},
                "uncertain":"uncertain"
            }}}
        }}),
    );
}

async fn observe_requests(root: &Path, selection: Option<Value>) -> (LintReport, Vec<Vec<u8>>) {
    let mut server = controlled::ControlledServer::start().await;
    paragraph_project(root, &server.endpoint);
    let mut input = serde_json::to_value(fixture::request(root)).unwrap();
    if let Some(selection) = selection {
        input["case_selection"] = selection;
    }
    let request: LintRequest = serde_json::from_value(input).unwrap();
    let mut operation = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC_CASE_DISPATCH".into()),
    ));
    let (report, captured) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                report = &mut operation => break (report.unwrap(), captured),
                request = server.receive() => {
                    captured.push(request.raw_body.clone());
                    request.noul(0.9);
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(server.finish().await, captured.len());
    (report, captured)
}

#[tokio::test]
async fn case_selection_excludes_a_same_file_same_rule_owner_before_dispatch_with_exact_payload_parity()
 {
    let root = tempfile::tempdir().unwrap();
    let (ordinary, unfiltered) = observe_requests(root.path(), None).await;
    assert_eq!(ordinary.status, LintStatus::Clean, "{ordinary:?}");
    assert_eq!(
        unfiltered.len(),
        2,
        "the ordinary observer must see both eligible owners"
    );
    assert_eq!(
        ordinary
            .results
            .iter()
            .map(|result| (result.first_line, result.last_line))
            .collect::<Vec<_>>(),
        vec![(1, 1), (3, 3)],
        "the independently authored source ranges must reach both pieces"
    );
    let selection = json!([{
        "id":"selected-paragraph","kind":"rule","rule":"body","file":"docs/one.md",
        "lines":[1,1],"text_sha256":sunstack_evidence::content_digest("SELECTED café\n".as_bytes())
    }]);
    let (selected, filtered) = observe_requests(root.path(), Some(selection)).await;
    assert_eq!(selected.status, LintStatus::Clean, "{selected:?}");
    assert_eq!(
        filtered.len(),
        1,
        "an eligible excluded owner must make zero HTTP requests"
    );
    let expected = unfiltered
        .iter()
        .find(|bytes| String::from_utf8_lossy(bytes).contains("SELECTED café"))
        .unwrap();
    assert_eq!(
        &filtered[0], expected,
        "selection must preserve the complete request bytes"
    );
    assert!(!String::from_utf8_lossy(&filtered[0]).contains("EXCLUDED β"));
}

async fn observe_filter_requests(
    root: &Path,
    selection: Option<Value>,
    gated: bool,
) -> (LintReport, Vec<Vec<u8>>, LintReport) {
    let mut server = controlled::ControlledServer::start().await;
    super::filters::setup(root, &server.endpoint);
    if !gated {
        let mut entries: Value =
            serde_json::from_slice(&std::fs::read(root.join("rules.json")).unwrap()).unwrap();
        entries["K001"].as_object_mut().unwrap().remove("only_when");
        fixture::entry_file(root, &entries);
    }
    let mut input = serde_json::to_value(fixture::request(root)).unwrap();
    if let Some(selection) = selection {
        input["case_selection"] = selection;
    }
    let lint_request: LintRequest = serde_json::from_value(input).unwrap();
    let mut operation = tokio::spawn(run(
        lint_request.clone(),
        CancellationToken::new(),
        Some("SYNTHETIC_CASE_FILTER".into()),
    ));
    let (report, bodies) = tokio::time::timeout(Duration::from_secs(10), async {
        let mut bodies = Vec::new();
        loop {
            tokio::select! {
                report = &mut operation => break (report.unwrap(), bodies),
                request = server.receive() => {
                    let answers = request.body["questions"].as_object().unwrap().keys().map(|name| (
                        name.clone(), json!({"type":"choice","choice":"a","confidence":0.99,"probabilities":{"a":0.9,"b":0.1}})
                    )).collect::<serde_json::Map<_, _>>();
                    bodies.push(request.raw_body.clone());
                    request.respond(200, json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5}}));
                }
            }
        }
    }).await.unwrap();
    assert_eq!(server.finish().await, bodies.len());
    let prepared = prepare(lint_request, CancellationToken::new(), vec![]).await;
    (report, bodies, prepared)
}

#[tokio::test]
async fn case_selection_deduplicates_rule_dependencies_and_dispatches_a_standalone_filter_only() {
    let root = tempfile::tempdir().unwrap();
    let (ordinary, requests, ordinary_prepared) =
        observe_filter_requests(root.path(), None, true).await;
    assert_eq!(
        requests.len(),
        2,
        "ordinary filter traffic must expose both folders"
    );
    let expected = requests
        .iter()
        .find(|request| String::from_utf8_lossy(request).contains("Purpose one"))
        .unwrap();
    let expected_digest = ordinary_prepared.requests.iter().find(|request| matches!(&request.owner, RequestOwner::Filter { folder, .. } if folder == Path::new("packages/one"))).unwrap().input_sha256.clone();
    assert!(expected_digest.is_some());
    let actual_request = ordinary.requests.iter().find(|request| matches!(&request.owner, RequestOwner::Filter { folder, .. } if folder == Path::new("packages/one"))).unwrap();
    assert_eq!(
        Some(sunstack_evidence::content_digest(
            &serde_json::to_vec(&actual_request.request).unwrap()
        )),
        expected_digest
    );

    let rule = |id| json!({"id":id,"kind":"rule","rule":"description","file":"packages/one/references/member.md","lines":[1,1],"text_sha256":sunstack_evidence::content_digest(b"First.\n")});
    let (selected, bodies, selected_prepared) = observe_filter_requests(
        root.path(),
        Some(json!([rule("first"), rule("same-owner")])),
        true,
    )
    .await;
    assert_eq!(
        bodies,
        vec![expected.clone()],
        "a selected rule's filter dependency is one unchanged request, with no unrelated-folder request"
    );
    assert_eq!(selected.results.len(), 1);
    assert_eq!(selected.filter_answers.len(), 1);
    assert_eq!(selected_prepared.requests[0].input_sha256, expected_digest);
    let accounting = selected.case_selection.as_ref().unwrap();
    assert_eq!(accounting.bindings.len(), 2);
    assert!(
        accounting
            .bindings
            .iter()
            .all(|binding| matches!(binding.outcome, CaseBindingStatus::Selected { .. }))
    );
    assert!(accounting.excluded.iter().any(|owner| matches!(owner, CaseExecutionOwner::Filter { folder, .. } if folder == Path::new("packages/two"))));

    // This literal is the raw-read JSON contract, not the model input digest above.
    let reads_digest = sunstack_evidence::content_digest(br#"{"description":"Purpose one"}"#);
    let filter = |digest| json!([{"id":"filter-only","kind":"filter","filter":"purpose","folder":"packages/one","reads_sha256":digest}]);
    let (standalone, bodies, standalone_prepared) =
        observe_filter_requests(root.path(), Some(filter(reads_digest)), false).await;
    assert_eq!(
        bodies,
        vec![expected.clone()],
        "explicit filter cases remain reachable without any gated rule"
    );
    assert!(standalone.results.is_empty());
    assert_eq!(standalone.filter_answers.len(), 1);
    assert_eq!(
        standalone_prepared.requests[0].input_sha256,
        expected_digest
    );
    assert!(matches!(
        standalone.case_selection.unwrap().bindings[0].outcome,
        CaseBindingStatus::Selected { .. }
    ));

    let (stale, bodies, _) =
        observe_filter_requests(root.path(), Some(filter("0".repeat(64))), false).await;
    assert!(bodies.is_empty(), "stale filter reads must not reach HTTP");
    assert!(matches!(
        stale.case_selection.unwrap().bindings[0].outcome,
        CaseBindingStatus::Stale
    ));
}

#[tokio::test]
async fn case_selection_keeps_pending_selection_and_cancelled_or_incomplete_execution_in_accounting()
 {
    let root = tempfile::tempdir().unwrap();
    let server = controlled::ControlledServer::start().await;
    paragraph_project(root.path(), &server.endpoint);
    let mut config: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("sunstack.lint.json")).unwrap())
            .unwrap();
    config["evidence"]["kinds"]["paragraph"]["select"] = json!({
        "question":{"type":"noul","instructions":{"check":"body","task":"Select this paragraph?"}},
        "yes_at_or_above":0.8,"no_at_or_below":0.2
    });
    fixture::config_file(root.path(), &config);
    let mut request = fixture::request(root.path());
    request.case_selection = Some(
        serde_json::from_value(json!([{
            "id":"pending","kind":"rule","rule":"body","file":"docs/one.md","lines":[1,1],
            "text_sha256":sunstack_evidence::content_digest("SELECTED café\n".as_bytes())
        }]))
        .unwrap(),
    );
    let pending = prepare(request.clone(), CancellationToken::new(), vec![]).await;
    assert_eq!(pending.status, LintStatus::Prepared, "{pending:?}");
    assert!(
        matches!(
            pending.case_selection.as_ref().unwrap().bindings[0].outcome,
            CaseBindingStatus::SelectionPending { .. }
        ),
        "{pending:?}"
    );
    assert!(!pending.requests.is_empty());
    assert!(
        pending
            .requests
            .iter()
            .all(|request| matches!(request.owner, RequestOwner::Selection { .. })),
        "only shared selection prerequisites may be prepared"
    );
    assert!(
        pending
            .requests
            .iter()
            .all(|request| request.input_sha256.is_some())
    );
    assert_eq!(server.finish().await, 0);

    let mut server = controlled::ControlledServer::start().await;
    paragraph_project(root.path(), &server.endpoint);
    let cancellation = CancellationToken::new();
    let mut operation = tokio::spawn(run(
        request.clone(),
        cancellation.clone(),
        Some("SYNTHETIC_CANCEL".into()),
    ));
    let held = tokio::select! { held = server.next() => held, result = &mut operation => panic!("selected request never reached HTTP: {result:?}") };
    cancellation.cancel();
    let cancelled = tokio::time::timeout(Duration::from_secs(3), operation)
        .await
        .unwrap()
        .unwrap();
    drop(held);
    assert_eq!(server.finish().await, 1);
    assert_eq!(cancelled.status, LintStatus::Cancelled);
    assert!(matches!(
        cancelled.case_selection.unwrap().bindings[0].outcome,
        CaseBindingStatus::Cancelled
    ));

    let mut server = controlled::ControlledServer::start().await;
    paragraph_project(root.path(), &server.endpoint);
    let mut operation = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC_INCOMPLETE".into()),
    ));
    for _ in 0..3 {
        let held = tokio::select! { held = server.next() => held, result = &mut operation => panic!("selected request never reached HTTP: {result:?}") };
        held.respond(503, json!({"error":"controlled unavailable"}));
    }
    let incomplete = tokio::time::timeout(Duration::from_secs(3), operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 3);
    assert_eq!(incomplete.status, LintStatus::Incomplete);
    assert!(matches!(
        incomplete.case_selection.unwrap().bindings[0].outcome,
        CaseBindingStatus::Incomplete { .. }
    ));
}

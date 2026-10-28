#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/candidate_code_fixture.rs"]
mod code_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use code_fixture::CodeFixture;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[tokio::test]
async fn diff_pass_keeps_changed_and_unchanged_matches_with_frozen_related_code() {
    for changed_only in [false, true] {
        let mut server = ControlledServer::start().await;
        let fixture = CodeFixture::new(&server.endpoint, false);
        if changed_only {
            fixture.edit_rule(|rule| rule["mechanical_pass"]["context_parts"] = json!(["change"]));
        }
        let input = fixture.input.clone();
        let directory = fixture.repository.root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        for _ in 0..if changed_only { 1 } else { 2 } {
            let request = tokio::select! {output=&mut run=>panic!("code input refused: {}",String::from_utf8_lossy(&output.unwrap().stdout)),request=server.next()=>request};
            let body = request.body.to_string();
            assert!(body.contains("COMMITTED_CALLER"), "{body}");
            assert!(body.contains("COMMITTED_CALLEE"));
            assert!(!body.contains("DIRTY_CALLER_MUST_NOT_APPEAR"));
            assert!(!body.contains("sk-proj-publicfixturecredential123456789"));
            assert!(body.contains("UNCHANGED_MATCH") && body.contains("CHANGED_MATCH"));
            request.noul(0.9);
        }
        let output = run.await.unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let report = &wire["result"]["outcome"]["report"];
        let entries = report["candidates"].as_array().unwrap();
        assert_eq!(entries.len(), if changed_only { 1 } else { 2 });
        let mut parts = std::collections::BTreeSet::new();
        let expected = std::collections::BTreeMap::from([
            (
                fixture
                    .after_text
                    .find("flagged(\"CHANGED_MATCH\")")
                    .unwrap(),
                ("flagged(\"CHANGED_MATCH\")", "change"),
            ),
            (
                fixture
                    .after_text
                    .find("flagged(\"UNCHANGED_MATCH\")")
                    .unwrap(),
                ("flagged(\"UNCHANGED_MATCH\")", "touched_unit"),
            ),
        ]);
        let mut seen = std::collections::BTreeSet::new();
        for entry in entries {
            assert_eq!(entry["outcome"]["kind"], "finding");
            assert_eq!(entry["occurrence"]["kind"], "context_match");
            let span = &entry["occurrence"]["span"];
            assert_eq!(span["source"]["revision"]["commit_id"], fixture.after);
            assert_eq!(span["source"]["revision"]["side"], "after");
            let start = span["byte_start"].as_u64().unwrap() as usize;
            let end = span["byte_end"].as_u64().unwrap() as usize;
            let (snippet, part) = expected.get(&start).unwrap();
            assert!(seen.insert(start));
            assert_eq!(&fixture.after_text[start..end], *snippet);
            code_fixture::assert_span(
                span,
                &fixture.after_text,
                snippet,
                "src/lib.rs",
                "code",
                &fixture.after,
                "after",
            );
            assert_eq!(entry["occurrence"]["part"], *part);
            parts.insert(*part);
            let context = &entry["context"]["context"];
            assert!(!context["redactions"].as_array().unwrap().is_empty());
            assert!(!context.to_string().contains("DIRTY_CALLER_MUST_NOT_APPEAR"));
            assert!(
                !context
                    .to_string()
                    .contains("sk-proj-publicfixturecredential123456789")
            );
            let changes = context["changes"].as_array().unwrap();
            assert_eq!(changes.len(), 1);
            for (side, text, line, revision) in [
                (
                    "before",
                    &fixture.before_text,
                    "    unflagged(\"old\");\n",
                    &fixture.before,
                ),
                (
                    "after",
                    &fixture.after_text,
                    "    flagged(\"CHANGED_MATCH\");\n",
                    &fixture.after,
                ),
            ] {
                assert_eq!(changes[0][side]["text"], line);
                code_fixture::assert_span(
                    &changes[0][side]["span"],
                    text,
                    line,
                    "src/lib.rs",
                    "code",
                    revision,
                    side,
                );
            }
            assert!(
                context["units"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|unit| unit["text"].as_str().unwrap().contains("CHANGED_MATCH"))
            );
        }
        assert_eq!(
            seen,
            if changed_only {
                std::collections::BTreeSet::from([fixture
                    .after_text
                    .find("flagged(\"CHANGED_MATCH\")")
                    .unwrap()])
            } else {
                expected.keys().copied().collect()
            }
        );
        assert_eq!(
            parts,
            if changed_only {
                std::collections::BTreeSet::from(["change"])
            } else {
                std::collections::BTreeSet::from(["change", "touched_unit"])
            }
        );
        assert_eq!(
            report["inputs"][0]["location"],
            json!({"kind":"repository","repository":"code"})
        );
        assert_eq!(server.finish().await, entries.len());
    }
}

#[tokio::test]
async fn unsupported_ast_language_refuses_before_any_model_work() {
    let server = ControlledServer::start().await;
    let fixture = CodeFixture::new(&server.endpoint, false);
    fixture.edit_rule(|rule| {
        rule["mechanical_pass"]["predicate"]["language"] = json!("unsupported-language")
    });
    let output = support::run_without_credential(
        fixture.repository.root.path(),
        "evaluate_candidates",
        fixture.input.clone(),
    )
    .await;
    assert_eq!(output.status.code(), Some(2));
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
    assert!(
        wire["result"]["outcome"]["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |diagnostic| diagnostic["location"]["field_path"]
                    .as_array()
                    .is_some_and(
                        |path| path.starts_with(&[json!("mechanical_pass"), json!("predicate")])
                    )
            ),
        "unsupported language must be refused at its mechanical predicate: {wire}"
    );
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn code_inputs_need_no_session_and_overlapping_scopes_keep_their_own_facts() {
    let mut server = ControlledServer::start().await;
    let fixture = CodeFixture::new(&server.endpoint, false);
    let root = fixture.repository.root.path();
    let mut input = fixture.code_input();
    let first = input["sources"][0]["source"].clone();
    let mut second = first.clone();
    second["paths"] = json!([{"kind":"glob","pattern":"src/**"}]);
    let identities = [&first, &second].map(|source| {
        serde_json::from_value::<sunstack_evidence::SourceInput>(source.clone())
            .unwrap()
            .source_identity()
            .unwrap()
    });
    input["sources"] = json!([
        {"source":first,"evidence":{"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"ALLOW_CODE_SCOPE"}}}}},
        {"source":second,"evidence":{"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"DENY_CODE_SCOPE"}}}}}
    ]);
    let path = root.join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    set["inputs"]["fact"] = json!({"kind":"payload","schema":"urn:code-fact"});
    std::fs::write(path, set.to_string()).unwrap();
    fixture.edit_rule(|rule| {
        rule["evidence"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"outside","input":"fact","select":{"kind":"whole_payload"}}))
    });
    std::fs::write(root.join("code-fact.json"),json!({"type":"object","properties":{"decision":{"type":"string"}},"required":["decision"],"additionalProperties":false}).to_string()).unwrap();
    input["payload_schemas"] = json!([{"name":"urn:code-fact","path":"code-fact.json"}]);
    let directory = root.to_owned();
    let mut run =
        tokio::spawn(async move { support::run(&directory, "evaluate_candidates", input).await });
    for _ in 0..4 {
        let request = tokio::select! {output=&mut run=>panic!("overlapping code inputs stopped {}",String::from_utf8_lossy(&output.unwrap().stdout)),request=server.next()=>request};
        let text = request.body.to_string();
        let allowed = text.contains("ALLOW_CODE_SCOPE");
        assert_ne!(allowed, text.contains("DENY_CODE_SCOPE"));
        request.noul(if allowed { 0.9 } else { 0.1 });
    }
    let output = run.await.unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    let entries = wire["result"]["outcome"]["report"]["candidates"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 4);
    let mut distinct = std::collections::BTreeSet::new();
    let mut counts = [0, 0];
    let mut spans = [
        std::collections::BTreeSet::new(),
        std::collections::BTreeSet::new(),
    ];
    for entry in entries {
        assert_eq!(entry["occurrence"]["kind"], "code_match");
        let id = entry["occurrence"]["input_id"].as_str().unwrap();
        let position = identities
            .iter()
            .position(|identity| identity.as_str() == id)
            .unwrap();
        counts[position] += 1;
        let span = &entry["occurrence"]["span"];
        let start = span["byte_start"].as_u64().unwrap() as usize;
        let snippet = ["flagged(\"CHANGED_MATCH\")", "flagged(\"UNCHANGED_MATCH\")"]
            .into_iter()
            .find(|snippet| fixture.after_text.find(snippet) == Some(start))
            .unwrap();
        code_fixture::assert_span(
            span,
            &fixture.after_text,
            snippet,
            "src/lib.rs",
            "code",
            &fixture.after,
            "after",
        );
        assert!(spans[position].insert(snippet));
        let occurrence = entry["candidate_id"]["occurrence"].as_str().unwrap();
        assert!(occurrence.starts_with(&format!("{id}:")));
        assert!(distinct.insert(occurrence));
        assert_eq!(
            entry["outcome"]["kind"],
            if position == 0 { "finding" } else { "dropped" }
        );
        assert_eq!(
            entry["answers"][0]["answer"]["yes_probability"],
            if position == 0 { 0.9 } else { 0.1 }
        );
        assert_eq!(
            entry["evidence"][0]["binding"]["binding"]["value"]["decision"],
            if position == 0 {
                "ALLOW_CODE_SCOPE"
            } else {
                "DENY_CODE_SCOPE"
            }
        );
    }
    assert_eq!(counts, [2, 2]);
    for found in spans {
        assert_eq!(
            found,
            std::collections::BTreeSet::from([
                "flagged(\"CHANGED_MATCH\")",
                "flagged(\"UNCHANGED_MATCH\")"
            ])
        );
    }
    assert_eq!(server.finish().await, 4);
}

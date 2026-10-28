#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/candidate_code_fixture.rs"]
mod code_fixture;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use code_fixture::CodeFixture;
use serde_json::{Value, json};
use transport::{AnswerServer, ReplyMode, RouterReply};

// An impossible signature must leave exactly its own candidate over budget and its peer judged.
#[tokio::test]
async fn an_unfittable_signature_keeps_its_over_budget_candidate_and_the_ordinary_peer() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let fixture = CodeFixture::new(&server.endpoint, false);
    let parameters = (0..1500)
        .map(|position| format!("argument_{position:04}: i64"))
        .collect::<Vec<_>>()
        .join(", ");
    let signature = format!("pub fn impossible({parameters})");
    assert!(signature.chars().count() > 20_000);
    let source = format!(
        "{signature} {{\n    flagged(\"TOO_BIG\");\n    placeholder();\n}}\npub fn ordinary() {{ flagged(\"ORDINARY\"); }}\n"
    );
    fixture.repository.write("src/lib.rs", &source);
    fixture.repository.stage("src/lib.rs");
    let revision = fixture.repository.commit();
    let mut input = fixture.code_input();
    input["sources"][0]["source"]["revision"]["revision"] = json!(revision);
    let source_input: sunstack_evidence::SourceInput =
        serde_json::from_value(input["sources"][0]["source"].clone()).unwrap();
    let input_id = source_input.source_identity().unwrap();
    let expected_rule = json!({"rule_set":fixture.repository.root.path().join("set.json").canonicalize().unwrap(),"rule_id":"candidate.one"});
    let output = support::run(fixture.repository.root.path(), "evaluate_candidates", input).await;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(wire["result"]["outcome"]["kind"], "candidate_report");
    let report = &wire["result"]["outcome"]["report"];
    let (count, bodies) = server.finish().await;
    for body in &bodies {
        let contents = body["state"]["contents"].as_object().unwrap();
        let page = contents
            .values()
            .find_map(|content| content.get("page").and_then(Value::as_str))
            .unwrap();
        let rendered_chars = serde_json::to_string(page).unwrap().chars().count() - 2;
        assert!(
            rendered_chars <= 20_000,
            "recorded page exceeds the fixed cap: {rendered_chars}"
        );
    }
    let entries = report["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    let locate = |text: &str| {
        entries
            .iter()
            .find(|entry| entry["occurrence"]["span"]["byte_start"] == source.find(text).unwrap())
            .unwrap()
    };
    let over = locate("flagged(\"TOO_BIG\")");
    let peer = locate("flagged(\"ORDINARY\")");
    for (entry, snippet) in [
        (over, "flagged(\"TOO_BIG\")"),
        (peer, "flagged(\"ORDINARY\")"),
    ] {
        code_fixture::assert_span(
            &entry["occurrence"]["span"],
            &source,
            snippet,
            "src/lib.rs",
            "code",
            &revision,
            "after",
        );
        assert_eq!(entry["rule"], expected_rule);
        assert_eq!(entry["candidate_id"]["rule"], expected_rule);
        assert_eq!(entry["occurrence"]["input_id"], input_id.as_str());
        let occurrence: sunstack_evidence::SourceOccurrence =
            serde_json::from_value(entry["occurrence"].clone()).unwrap();
        assert_eq!(
            entry["candidate_id"]["occurrence"],
            occurrence.occurrence_identity().unwrap().as_str()
        );
        assert_eq!(
            entry["context"]["context"]["matched"]["span"],
            entry["occurrence"]["span"]
        );
    }
    assert_ne!(over["candidate_id"], peer["candidate_id"]);
    assert_eq!(
        over["outcome"],
        json!({"kind":"not_processed","reason":"over_budget"})
    );
    assert_eq!(peer["outcome"]["kind"], "finding");
    assert!(over["answers"].as_array().unwrap().is_empty());
    let view = &over["context"];
    assert!(view["pages"].as_array().unwrap().is_empty());
    let units = view["context"]["units"].as_array().unwrap();
    let blocked = units
        .iter()
        .find(|unit| unit["unit"]["key"] == over["occurrence"]["unit"])
        .unwrap();
    assert_eq!(blocked["unit"]["kind"]["signature"], signature);
    let coverage = view["coverage"].as_array().unwrap();
    assert!(view["context"]["imports"].as_array().unwrap().is_empty());
    assert!(view["context"]["changes"].as_array().unwrap().is_empty());
    let mut expected_pieces = units
        .iter()
        .map(|unit| unit["piece_id"].as_str().unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    expected_pieces.extend(
        view["context"]["related"]
            .as_array()
            .unwrap()
            .iter()
            .map(|piece| piece["piece_id"].as_str().unwrap().to_owned()),
    );
    expected_pieces.insert(format!(
        "match:{}",
        view["context"]["context_id"].as_str().unwrap()
    ));
    assert_eq!(
        coverage
            .iter()
            .map(|piece| piece["piece_id"].as_str().unwrap().to_owned())
            .collect::<std::collections::BTreeSet<_>>(),
        expected_pieces
    );
    assert_eq!(
        view["labels"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        expected_pieces
    );
    assert_eq!(
        coverage.len(),
        units.len()
            + view["context"]["related"].as_array().unwrap().len()
            + view["context"]["imports"].as_array().unwrap().len()
            + 1
    );
    assert_eq!(
        coverage
            .iter()
            .map(|piece| piece["piece_id"].as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        coverage.len()
    );
    assert_eq!(coverage.len(), view["labels"].as_object().unwrap().len());
    assert!(!coverage.is_empty());
    for piece in coverage {
        assert_eq!(piece["disposition"]["kind"], "excluded");
        assert_eq!(piece["disposition"]["reason"]["kind"], "over_page_budget");
        assert_eq!(piece["disposition"]["reason"]["unit"], blocked["piece_id"]);
        assert!(
            piece["disposition"]["reason"]["required_chars"]
                .as_u64()
                .unwrap()
                > 20_000
        );
    }
    assert_eq!(
        report["completeness"],
        json!({"kind":"findings","count":1,"inconclusive":1})
    );
    assert_eq!(report["usage"]["requests"], 1);
    assert_eq!(count, 1);
    assert_eq!(bodies.len(), 1);
    assert_eq!(peer["answers"].as_array().unwrap().len(), 1);
    assert_eq!(peer["answers"][0]["answer"]["yes_probability"], 0.9);
    assert_eq!(peer["answers"][0]["page"], 1);
    assert_eq!(
        bodies[0]["state"]["contents"]["code_context"]["page"],
        peer["context"]["pages"][0]["text"]
    );
    assert!(!bodies[0].to_string().contains("fn impossible"));
    assert!(bodies[0].to_string().contains("fn ordinary"));
}

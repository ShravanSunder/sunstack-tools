#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
#[path = "support/narrowing_sample_runtime.rs"]
mod narrowing_sample_runtime;
mod support;
use controlled_http::ControlledServer;
#[path = "support/candidate_code_fixture.rs"]
mod candidate_code_fixture;
use narrowing_sample_runtime::{
    add_narrowing_question, answer_until_process_finishes, noul_response,
    report_from_process_output, response_for_clear_code_or_document_drop, start_candidate_process,
};
use serde_json::{Value, json};

#[tokio::test]
async fn code_drops_keep_projected_excerpts_and_source_spans() {
    let code_fixture = candidate_code_fixture::CodeFixture::new("http://127.0.0.1:1", false);
    let code_source = (0..10)
        .map(|index| format!("pub fn sample_{index:02}() {{ flagged(\"CODE_{index:02}\"); }}\n"))
        .collect::<String>();
    let code_root = code_fixture.repository.root.path().to_owned();
    code_fixture.repository.write("src/lib.rs", &code_source);
    code_fixture.repository.stage("src/lib.rs");
    let code_revision = code_fixture.repository.commit();
    let mut code_input = code_fixture.code_input();
    code_input["sources"][0]["source"]["revision"]["revision"] = json!(code_revision.clone());
    code_input["samples_per_pass"] = json!(4);
    code_input["request_budget"]["max_requests"] = json!(100);
    let mut code_server = ControlledServer::start().await;
    code_input["backend"]["endpoint"] = json!(code_server.endpoint.clone());
    add_narrowing_question(&code_root, "NARROW_CODE");
    let mut code_run = start_candidate_process(&code_root, code_input).await;
    let code_output = answer_until_process_finishes(
        &mut code_server,
        &mut code_run,
        response_for_clear_code_or_document_drop,
    )
    .await;
    let code_report = report_from_process_output(&code_output);
    assert_eq!(code_report["candidates"].as_array().unwrap().len(), 10);
    for candidate in code_report["candidates"].as_array().unwrap() {
        assert_eq!(candidate["outcome"]["kind"], "dropped");
        assert_eq!(candidate["outcome"]["at"]["kind"], "narrowing");
    }
    let code_samples = code_report["dropped_samples"].as_array().unwrap();
    assert_eq!(code_samples.len(), 4);
    let expected_indices = [0, 2, 5, 7];
    let expected_starts = expected_indices.map(|index| {
        code_source
            .find(&format!("flagged(\"CODE_{index:02}\")"))
            .unwrap()
    });
    let actual_starts = code_samples
        .iter()
        .map(|sample| sample["occurrence"]["span"]["byte_start"].as_u64().unwrap() as usize)
        .collect::<Vec<_>>();
    assert_eq!(actual_starts, expected_starts);
    for (sample, index) in code_samples.iter().zip(expected_indices) {
        let expected_excerpt = format!("flagged(\"CODE_{index:02}\")");
        assert!(
            code_report["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate["rule"] == sample["rule"]
                    && candidate["occurrence"] == sample["occurrence"])
        );
        assert_eq!(sample["pass"], "narrowing");
        assert_eq!(sample["occurrence"]["kind"], "code_match");
        candidate_code_fixture::assert_span(
            &sample["occurrence"]["span"],
            &code_source,
            &expected_excerpt,
            "src/lib.rs",
            "code",
            &code_revision,
            "after",
        );
        assert_eq!(sample["excerpt"], expected_excerpt);
    }
    assert!(code_server.finish().await >= 10);
}

#[tokio::test]
async fn diff_tied_coordinates_choose_before_side_then_report_the_change_part() {
    let mut server = ControlledServer::start().await;
    let fixture = candidate_code_fixture::CodeFixture::new(&server.endpoint, false);
    let before_text = "pub fn touched() { flagged(\"OLD__MATCH\"); }\n";
    let after_text = "pub fn touched() { flagged(\"NEW__MATCH\"); }\n";
    assert_eq!(before_text.len(), after_text.len());
    fixture.repository.write("src/lib.rs", before_text);
    fixture.repository.stage("src/lib.rs");
    let before_revision = fixture.repository.commit();
    fixture.repository.write("src/lib.rs", after_text);
    fixture.repository.stage("src/lib.rs");
    let after_revision = fixture.repository.commit();
    let mut input = fixture.input.clone();
    input["sources"][0]["source"]["scope"]["base_revision"] = json!(before_revision);
    input["sources"][0]["source"]["scope"]["head"]["revision"] = json!(after_revision);
    input["samples_per_pass"] = json!(1);
    let root = fixture.repository.root.path().to_owned();
    let rule_path = root.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&rule_path).unwrap()).unwrap();
    rule["mechanical_pass"]["source_sides"] = json!("both_available");
    rule["narrowing"] = json!({
        "question":{"type":"noul","instructions":"NARROW_DIFF"},
        "evidence":rule["evidence"],
        "threshold":0.8
    });
    std::fs::write(rule_path, rule.to_string()).unwrap();
    input["backend"]["endpoint"] = json!(server.endpoint.clone());

    let mut run = start_candidate_process(&root, input).await;
    let output = answer_until_process_finishes(&mut server, &mut run, |body| {
        noul_response(body, |question| {
            if question.to_string().contains("NARROW_DIFF") {
                0.1
            } else {
                0.9
            }
        })
    })
    .await;
    let report = report_from_process_output(&output);
    assert_eq!(report["candidates"].as_array().unwrap().len(), 2);
    let candidates = report["candidates"].as_array().unwrap();
    for candidate in candidates {
        assert_eq!(candidate["outcome"]["kind"], "dropped");
        assert_eq!(candidate["outcome"]["at"]["kind"], "narrowing");
    }
    let before_candidate = candidates
        .iter()
        .find(|candidate| candidate["occurrence"]["span"]["source"]["revision"]["side"] == "before")
        .unwrap();
    let after_candidate = candidates
        .iter()
        .find(|candidate| candidate["occurrence"]["span"]["source"]["revision"]["side"] == "after")
        .unwrap();
    let before_span = &before_candidate["occurrence"]["span"];
    let after_span = &after_candidate["occurrence"]["span"];
    assert_eq!(before_span["byte_start"], after_span["byte_start"]);
    assert_eq!(before_span["byte_end"], after_span["byte_end"]);
    candidate_code_fixture::assert_span(
        before_span,
        before_text,
        "flagged(\"OLD__MATCH\")",
        "src/lib.rs",
        "code",
        &before_revision,
        "before",
    );
    candidate_code_fixture::assert_span(
        after_span,
        after_text,
        "flagged(\"NEW__MATCH\")",
        "src/lib.rs",
        "code",
        &after_revision,
        "after",
    );
    let samples = report["dropped_samples"].as_array().unwrap();
    assert_eq!(samples.len(), 1);
    let sample = &samples[0];
    let before_start = before_text.find("flagged(\"OLD__MATCH\")").unwrap();
    let after_start = after_text.find("flagged(\"NEW__MATCH\")").unwrap();
    assert_eq!(before_start, after_start);
    assert_eq!(sample["occurrence"]["kind"], "context_match");
    assert!(
        report["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate["rule"] == sample["rule"]
                && candidate["occurrence"] == sample["occurrence"])
    );
    assert_eq!(sample["occurrence"]["part"], "change");
    assert_eq!(&sample["occurrence"]["span"], before_span);
    assert_eq!(sample["excerpt"], "flagged(\"OLD__MATCH\")");
    assert!(server.finish().await >= 2);
}

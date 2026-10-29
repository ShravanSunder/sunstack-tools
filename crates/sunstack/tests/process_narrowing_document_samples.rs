#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
#[path = "support/narrowing_sample_runtime.rs"]
mod narrowing_sample_runtime;
mod support;
use controlled_http::ControlledServer;
#[path = "support/document_candidate_fixture.rs"]
mod document_candidate_fixture;
use narrowing_sample_runtime::{
    add_narrowing_question, answer_until_process_finishes, narrowing_answer,
    report_from_process_output, response_for_clear_code_or_document_drop, start_candidate_process,
};
use serde_json::json;

#[tokio::test]
async fn document_drops_keep_projected_excerpts_and_source_spans() {
    let mut document_server = ControlledServer::start().await;
    let selection = json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]});
    let (repository, input, source_text, revision) =
        document_candidate_fixture::fixture(&document_server.endpoint, selection, None);
    let document_root_path = repository.root.path().to_owned();
    let mut input = input;
    input["samples_per_pass"] = json!(2);
    input["execution"]["max_concurrent_requests"] = json!(2);
    add_narrowing_question(&document_root_path, "NARROW_DOCUMENT");
    let mut document_run = start_candidate_process(&document_root_path, input).await;
    let document_output = answer_until_process_finishes(
        &mut document_server,
        &mut document_run,
        response_for_clear_code_or_document_drop,
    )
    .await;
    let document_report = report_from_process_output(&document_output);
    assert_eq!(document_report["candidates"].as_array().unwrap().len(), 3);
    for candidate in document_report["candidates"].as_array().unwrap() {
        assert_eq!(candidate["outcome"]["kind"], "dropped");
        assert_eq!(candidate["outcome"]["at"]["kind"], "narrowing");
        assert_eq!(
            narrowing_answer(candidate)["answer"]["yes_probability"],
            0.1
        );
    }
    let document_samples = document_report["dropped_samples"].as_array().unwrap();
    assert_eq!(document_samples.len(), 2);
    let expected_paragraphs = [
        (
            "If café is ready, load alpha.\r\n",
            "If café is ready, load alpha.",
        ),
        ("Ordinary words remain.\r\n", "Ordinary words remain."),
    ];
    for (sample, (raw_paragraph, visible_paragraph)) in
        document_samples.iter().zip(expected_paragraphs)
    {
        assert_eq!(sample["pass"], "narrowing");
        assert_eq!(sample["excerpt"], visible_paragraph);
        assert!(
            document_report["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate["rule"] == sample["rule"]
                    && candidate["occurrence"] == sample["occurrence"])
        );
        let span = &sample["occurrence"]["occurrence"]["span"];
        document_candidate_fixture::assert_document_excerpt(
            span,
            &source_text,
            raw_paragraph,
            &revision,
        );
    }
    assert!(document_server.finish().await >= 3);
}

#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/eval_process_fixture.rs"]
mod eval_fixture;
#[path = "support/signal_process.rs"]
mod signal_process;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use eval_fixture::{EvalFixture, LABEL_SENTINEL, response};
use signal_process::{collect_output, expect_line, ready_process_without_credential, send_signal};
use tokio::io::AsyncWriteExt;
use transport::{AnswerServer, ReplyMode, RouterReply};
#[tokio::test]
async fn observed_cancellation_returns_an_eval_report_without_dispatch_or_invented_answers() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let fixture = EvalFixture::new(&server.endpoint);
    let (mut process, mut stderr, mut transcript) =
        ready_process_without_credential(fixture.root.path()).await;
    send_signal(&process, "-TERM").await;
    expect_line(
        &mut stderr,
        "sunstack: cancellation observed\n",
        &mut transcript,
    )
    .await;
    let bytes = serde_json::to_vec(
        &serde_json::json!({"protocol_version":1,"operation":"score_eval","input":fixture.input}),
    )
    .unwrap();
    let mut stdin = process.stdin.take().unwrap();
    stdin.write_all(&bytes).await.unwrap();
    drop(stdin);
    let output = collect_output(process, stderr, transcript).await;
    let wire = response(&output, 4);
    assert_eq!(wire["result"]["outcome"]["kind"], "cancelled");
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(report["cost"]["requests"], 0);
    assert_eq!(
        report["filter"]["unknown"],
        serde_json::json!([LABEL_SENTINEL])
    );
    assert_eq!(
        report["end_result"]["inconclusive"],
        serde_json::json!([LABEL_SENTINEL])
    );
    assert_eq!(server.finish().await.0, 0);
}

#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_decisions::test_support as support;
use sunstack_decisions::*;
use support::*;

#[tokio::test]
async fn default_stdout_stderr_and_diagnostic_display_exclude_sentinels() {
    if std::env::var_os("SUNSTACK_PRIVACY_CHILD").is_some() {
        let server = TestServer::start(vec![HttpReply {
            status: 400,
            body: b"SENTINEL_PROVIDER_BODY".to_vec(),
            chunked: false,
        }])
        .await;
        let mut input = mixed_input(&server.endpoint);
        input.request.state = SharedEvidenceState::Text("SENTINEL_EVIDENCE".into());
        let outcome = run_judgments(
            &input,
            &backend(&server.endpoint),
            &CancellationToken::new(),
        )
        .await;
        if let DecisionRunOutcome::ProviderFailure { diagnostics, .. } = &outcome {
            println!("{}", diagnostics.items[0]);
        } else {
            panic!("not provider failure");
        }
        println!("{}", serde_json::to_string(&outcome).unwrap());
        assert_eq!(server.finish().await, 1);
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "default_stdout_stderr_and_diagnostic_display_exclude_sentinels",
            "--nocapture",
        ])
        .env("SUNSTACK_PRIVACY_CHILD", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    for sentinel in [
        "SENTINEL_CREDENTIAL_987",
        "SENTINEL_PROVIDER_BODY",
        "SENTINEL_HEADER",
        "SENTINEL_EVIDENCE",
    ] {
        assert!(!stdout.contains(sentinel));
        assert!(!stderr.contains(sentinel));
    }
    assert!(stdout.contains("provider_rejected_request"));
    assert!(stdout.contains("repair_request"));
}

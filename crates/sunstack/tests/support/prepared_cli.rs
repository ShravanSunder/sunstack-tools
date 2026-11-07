use super::raw_request_fields::{RawRequestFields, compact_json_value};
use serde_json::Value;
use std::{path::Path, process::Output, time::Duration};
use sunstack_rules::test_support::controlled_http::ControlledServer;

pub async fn answered(
    root: &Path,
    server: &mut ControlledServer,
    args: &[&str],
    response: impl Fn(&Value) -> Value,
) -> (Output, Vec<Vec<u8>>) {
    let mut command = super::isolated_command(root);
    command
        .current_dir(root)
        .args(["lint", "--config", "skill-rules.config.json"])
        .args(args)
        .env("OPENROUTER_API_KEY", "SYNTHETIC_LOOPBACK_ONLY")
        .kill_on_drop(true);
    let mut operation = tokio::spawn(command.output());
    let mut bodies = Vec::new();
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                output = &mut operation => break output.unwrap().unwrap(),
                request = server.receive() => {
                    let reply = response(&request.body);
                    bodies.push(request.raw_body.clone()); request.respond(200, reply);
                }
            }
        }
    })
    .await
    .unwrap();
    (output, bodies)
}

pub async fn keyless(root: &Path, server: &mut ControlledServer, args: &[&str]) -> Output {
    let mut command = super::isolated_command(root);
    command
        .current_dir(root)
        .args(["lint", "--config", "skill-rules.config.json"])
        .args(args)
        .env_remove("OPENROUTER_API_KEY")
        .kill_on_drop(true);
    let mut operation = tokio::spawn(command.output());
    let mut unexpected = 0;
    // Settle forbidden calls before failing, so an ignored flag cannot strand a case child.
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                output = &mut operation => break output.unwrap().unwrap(),
                request = server.receive() => { unexpected += 1; request.noul(0.9); }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(unexpected, 0, "keyless prepare sent an HTTP request");
    output
}

pub fn assert_wire_parity(prepared: &[u8], ordinary: &[u8], captured: &[Vec<u8>]) {
    #[derive(serde::Deserialize)]
    struct Report<'a> {
        #[serde(borrow)]
        requests: Vec<Entry<'a>>,
    }
    #[derive(serde::Deserialize)]
    struct Entry<'a> {
        #[serde(borrow)]
        request: RawRequestFields<'a>,
    }
    let prepared: Report<'_> = serde_json::from_slice(prepared).unwrap();
    let ordinary: Report<'_> = serde_json::from_slice(ordinary).unwrap();
    assert_eq!(prepared.requests.len(), captured.len());
    assert_eq!(ordinary.requests.len(), captured.len());
    for entry in &prepared.requests {
        let state = compact_json_value(entry.request.state.get());
        let questions = compact_json_value(entry.request.questions.get());
        assert_eq!(
            captured
                .iter()
                .map(|raw| serde_json::from_slice::<RawRequestFields<'_>>(raw).unwrap())
                .filter(|request| compact_json_value(request.state.get()) == state
                    && compact_json_value(request.questions.get()) == questions)
                .count(),
            1
        );
        assert_eq!(
            ordinary
                .requests
                .iter()
                .filter(
                    |request| compact_json_value(request.request.state.get()) == state
                        && compact_json_value(request.request.questions.get()) == questions
                )
                .count(),
            1
        );
    }
}

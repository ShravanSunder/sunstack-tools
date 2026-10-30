#![allow(dead_code, clippy::unwrap_used, clippy::panic)]
use crate::support::{child, child_without_credential};
use std::{path::Path, process::Output, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{Child, ChildStderr},
};

const CHILD_READY_STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
pub async fn expect_line(
    stderr: &mut BufReader<ChildStderr>,
    expected: &str,
    transcript: &mut Vec<u8>,
) {
    let mut received = String::new();
    tokio::time::timeout(Duration::from_secs(5), stderr.read_line(&mut received))
        .await
        .unwrap()
        .unwrap();
    transcript.extend_from_slice(received.as_bytes());
    assert_eq!(received, expected);
}
pub async fn ready_process(root: &Path) -> (Child, BufReader<ChildStderr>, Vec<u8>) {
    observe_ready(child(root)).await
}
pub async fn ready_process_without_credential(
    root: &Path,
) -> (Child, BufReader<ChildStderr>, Vec<u8>) {
    observe_ready(child_without_credential(root)).await
}
async fn observe_ready(mut process: Child) -> (Child, BufReader<ChildStderr>, Vec<u8>) {
    let mut stderr = BufReader::new(process.stderr.take().unwrap());
    let mut transcript = vec![];
    tokio::time::timeout(CHILD_READY_STARTUP_TIMEOUT, async {
        loop {
            let mut received = String::new();
            stderr.read_line(&mut received).await.unwrap();
            transcript.extend_from_slice(received.as_bytes());
            assert!(transcript.len() <= 256);
            if received == "sunstack: ready\n" {
                break;
            }
            assert!(received.starts_with("cargo debug runner: sunstack runs unsigned ("));
        }
    })
    .await
    .expect("child startup did not emit sunstack: ready within 20 s");
    (process, stderr, transcript)
}
pub async fn send_signal(process: &Child, signal_name: &str) {
    assert!(
        tokio::process::Command::new("/bin/kill")
            .args([signal_name, &process.id().unwrap().to_string()])
            .status()
            .await
            .unwrap()
            .success()
    );
}
pub async fn collect_output(
    process: Child,
    mut stderr: BufReader<ChildStderr>,
    mut transcript: Vec<u8>,
) -> Output {
    let mut output = tokio::time::timeout(Duration::from_secs(5), process.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    stderr.read_to_end(&mut transcript).await.unwrap();
    output.stderr = transcript;
    output
}

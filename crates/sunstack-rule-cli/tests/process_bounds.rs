#![allow(clippy::unwrap_used, clippy::panic)]
mod support;
use serde_json::json;
use std::time::Duration;
use support::*;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn stdin_raw_byte_bound_accepts_exact_limit_and_refuses_excess_without_eof_or_parse() {
    let fixture = tempfile::tempdir().unwrap();
    std::fs::write(fixture.path().join("skill.md"), "U001: bounded ☃\n").unwrap();
    let mut input = serde_json::to_vec(&json!({"protocol_version":1,"operation":"prepare_evidence","input":evidence_input(fixture.path())})).unwrap();
    input.resize(8_388_608, b' ');
    let wire = response(&run_raw(fixture.path(), &input).await, 0);
    assert_eq!(wire["result"]["outcome"]["kind"], "prepared");
    for malformed in [false, true] {
        let mut excess = input.clone();
        excess.push(b' ');
        if malformed {
            excess[0] = b'!';
        }
        let mut process = child(fixture.path());
        let mut stdin = process.stdin.take().unwrap();
        stdin.write_all(&excess).await.unwrap();
        // Keep the writer open: a read-until-EOF implementation cannot finish.
        let output = tokio::time::timeout(Duration::from_secs(5), process.wait_with_output())
            .await
            .unwrap()
            .unwrap();
        drop(stdin);
        record_output(fixture.path(), &excess, &output);
        let wire = response(&output, 2);
        assert_eq!(wire["kind"], "protocol_error");
        assert_eq!(wire["diagnostics"]["items"][0]["code"], "input_limit");
    }
}

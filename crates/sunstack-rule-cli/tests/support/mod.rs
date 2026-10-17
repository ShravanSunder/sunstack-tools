#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Output, Stdio},
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, Command},
};

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .into()
}
fn process_command(root: &Path) -> Command {
    let mut command = Command::new(workspace_root().join("scripts/cargo-debug-signing-runner.sh"));
    command
        .arg(env!("CARGO_BIN_EXE_sunstack-rule"))
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}
pub fn child(root: &Path) -> Child {
    process_command(root)
        .env("OPENROUTER_API_KEY", "SYNTHETIC_PROCESS_CREDENTIAL")
        .spawn()
        .unwrap()
}
struct ProcessRequest<'request> {
    root: &'request Path,
    bytes: &'request [u8],
    command: Command,
    deadline: Duration,
}
async fn invoke_process(mut request: ProcessRequest<'_>) -> (Output, Duration) {
    let started = Instant::now();
    let mut process = request.command.spawn().unwrap();
    let mut stdin = process.stdin.take().unwrap();
    if let Err(error) = stdin.write_all(request.bytes).await {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    drop(stdin);
    let output = tokio::time::timeout(request.deadline, process.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    let elapsed = started.elapsed();
    // Timings end at owned child/output completion, before artifact I/O.
    record_output(request.root, request.bytes, &output);
    (output, elapsed)
}
pub async fn run_hosted_timed(root: &Path, operation: &str, input: Value) -> (Output, Duration) {
    assert!(
        std::env::var_os("OPENROUTER_API_KEY").is_some(),
        "hosted credential not supplied"
    );
    // Inherit the one-command credential. The controlled helper deliberately
    // replaces it and must never be used for a genuine hosted observation.
    let bytes =
        serde_json::to_vec(&json!({"protocol_version":1,"operation":operation,"input":input}))
            .unwrap();
    invoke_process(ProcessRequest {
        root,
        bytes: &bytes,
        command: process_command(root),
        deadline: Duration::from_secs(420),
    })
    .await
}
pub async fn run_raw(root: &Path, bytes: &[u8]) -> Output {
    let mut command = process_command(root);
    command.env("OPENROUTER_API_KEY", "SYNTHETIC_PROCESS_CREDENTIAL");
    invoke_process(ProcessRequest {
        root,
        bytes,
        command,
        deadline: Duration::from_secs(10),
    })
    .await
    .0
}
pub async fn run_timed(root: &Path, operation: &str, input: Value) -> (Output, Duration) {
    let bytes =
        serde_json::to_vec(&json!({"protocol_version":1,"operation":operation,"input":input}))
            .unwrap();
    let mut command = process_command(root);
    command.env("OPENROUTER_API_KEY", "SYNTHETIC_PROCESS_CREDENTIAL");
    invoke_process(ProcessRequest {
        root,
        bytes: &bytes,
        command,
        deadline: Duration::from_secs(10),
    })
    .await
}
pub fn record_output(root: &Path, input: &[u8], output: &Output) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
    let directory = workspace_root()
        .join("target/cli-process-evidence")
        .join(format!(
            "{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("stdin.json"), input).unwrap();
    std::fs::write(directory.join("stdout.json"), &output.stdout).unwrap();
    std::fs::write(directory.join("stderr.log"), &output.stderr).unwrap();
    std::fs::write(directory.join("result.json"), serde_json::to_vec_pretty(&json!({"cwd":root,"binary":env!("CARGO_BIN_EXE_sunstack-rule"),"exit":output.status.code(),"stdin_bytes":input.len()})).unwrap()).unwrap();
}
pub fn record_observation(name: &str, value: Value) {
    let directory = workspace_root().join("target/cli-process-observations");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join(format!("{}-{name}.json", std::process::id())),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
}
pub fn record_http(name: &str, body: &[u8], connections: usize) {
    let directory = workspace_root().join("target/cli-process-observations");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join(format!("{}-{name}-request.json", std::process::id())),
        body,
    )
    .unwrap();
    record_observation(
        name,
        json!({"connections":connections,"request_bytes":body.len()}),
    );
}
pub async fn run(root: &Path, operation: &str, input: Value) -> Output {
    run_raw(
        root,
        &serde_json::to_vec(&json!({"protocol_version":1,"operation":operation,"input":input}))
            .unwrap(),
    )
    .await
}
pub fn response(output: &Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.len() <= 256, "unbounded stderr");
    let wire =
        serde_json::from_slice(&output.stdout).expect("one complete JSON response is required");
    check_diagnostics(&wire);
    if wire["kind"] == "operation_result" {
        let outcome = wire["result"]["outcome"].clone();
        match wire["result"]["operation"].as_str().unwrap() {
            "prepare_evidence" => {
                serde_json::from_value::<sunstack_evidence::EvidencePlanOutcome>(outcome).unwrap();
            }
            "run_judgments" => {
                serde_json::from_value::<sunstack_decisions::DecisionRunOutcome>(outcome).unwrap();
            }
            "prepare_rules" => {
                serde_json::from_value::<sunstack_rules::RulePreparationOutcome>(outcome).unwrap();
            }
            "evaluate_rules" => {
                serde_json::from_value::<sunstack_rules::RuleInvocationOutcome>(outcome).unwrap();
            }
            other => panic!("unexpected operation {other}"),
        }
    }
    wire
}
fn check_diagnostics(value: &Value) {
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                if name == "diagnostics" {
                    let items = value["items"].as_array().unwrap();
                    assert!(items.len() <= 100);
                    assert!(value["omitted_count"].as_u64().is_some());
                    for item in items {
                        assert!(!item["code"].as_str().unwrap().is_empty());
                        assert!(item["guidance"].as_str().unwrap().chars().count() <= 512);
                        if let Some(segments) = item["location"]["field_path"].as_array() {
                            assert!(segments.len() <= 32);
                        }
                        let diagnostic = item.to_string();
                        for sentinel in [
                            "SELECTED_PROCESS_SENTINEL",
                            "PRIVATE_UNSELECTED_SENTINEL",
                            "SYNTHETIC_PROCESS_CREDENTIAL",
                            "AUTHOR_SECRET_KEY",
                            "PAYLOAD_SECRET_SENTINEL",
                            "PROVIDER_ERROR_SENTINEL",
                            "PROVIDER_BAD_JSON_SENTINEL",
                        ] {
                            assert!(!diagnostic.contains(sentinel));
                        }
                    }
                }
                check_diagnostics(value);
            }
        }
        Value::Array(items) => {
            for item in items {
                check_diagnostics(item);
            }
        }
        _ => {}
    }
}
pub fn evidence_input(root: &Path) -> Value {
    json!({
        "repositories":[{"key":"repo","root":root,"input_paths":[{"kind":"glob","pattern":"**"}],"change_scope":{"kind":"full"}}],
        "sources":{"skill":{"kind":"document","repository":"repo","path":"skill.md"}},
        "payload_schemas":[],"cursors":[],"native_queries":[],"package_bindings":[],"discovery":{"kind":"none"},
        "declarations":[{"id":"paragraph","input":"skill","requirement":"required","select":{"kind":"paragraph","target_id":"U001"}}],
        "limits":{"max_files":10,"max_file_bytes":10000,"max_total_bytes":100000,"max_matches":100,"max_transcript_bytes":10000,"include_ignored":false}
    })
}

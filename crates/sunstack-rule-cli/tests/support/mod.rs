#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Output, Stdio},
    time::Duration,
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
pub fn child(root: &Path) -> Child {
    Command::new(workspace_root().join("scripts/cargo-debug-signing-runner.sh"))
        .arg(env!("CARGO_BIN_EXE_sunstack-rule"))
        .current_dir(root)
        .env("OPENROUTER_API_KEY", "SYNTHETIC_PROCESS_CREDENTIAL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}
pub async fn run_raw(root: &Path, bytes: &[u8]) -> Output {
    let mut process = child(root);
    let mut stdin = process.stdin.take().unwrap();
    let write = stdin.write_all(bytes).await;
    if let Err(error) = write {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    drop(stdin);
    let output = tokio::time::timeout(Duration::from_secs(10), process.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    record_output(root, bytes, &output);
    output
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

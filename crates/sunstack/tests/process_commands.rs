#![allow(clippy::unwrap_used)]
use std::{path::Path, process::Stdio, time::Duration};
use tokio::process::Command;

async fn invoke(arguments: &[&str]) -> std::process::Output {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut command = Command::new(root.join("scripts/cargo-debug-signing-runner.sh"));
    command
        .arg(env!("CARGO_BIN_EXE_sunstack"))
        .args(arguments)
        .stdin(Stdio::null())
        .kill_on_drop(true);
    tokio::time::timeout(Duration::from_secs(5), command.output())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn version_is_the_binary_name_and_cargo_package_version() {
    let output = invoke(&["--version"]).await;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!("sunstack ", env!("CARGO_PKG_VERSION"), "\n")
    );
}

#[tokio::test]
async fn no_command_prints_help_and_exits_two() {
    let output = invoke(&[]).await;
    assert_eq!(output.status.code(), Some(2));
    let help = String::from_utf8(output.stderr).unwrap();
    assert!(help.contains("Usage:"), "{help}");
    assert!(help.contains("exec"), "{help}");
    assert!(help.contains("lint"), "{help}");
}

#![allow(clippy::unwrap_used)]
use std::{path::Path, process::Stdio, time::Duration};
#[path = "../support/mod.rs"]
mod support;

async fn invoke(arguments: &[&str]) -> std::process::Output {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut command = support::isolated_command(root);
    command
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
    assert!(help.contains("credentials"), "{help}");
}

#[tokio::test]
async fn credential_save_is_discoverable_and_requires_a_nonempty_environment_key() {
    let help = invoke(&["credentials", "--help"]).await;
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("save"));
    for key in [None, Some(""), Some(" \t")] {
        let mut command = support::isolated_command(&support::workspace_root());
        command
            .args(["credentials", "save"])
            .env_remove("OPENROUTER_API_KEY");
        if let Some(value) = key {
            command.env("OPENROUTER_API_KEY", value);
        }
        let output = tokio::time::timeout(Duration::from_secs(5), command.output())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("OPENROUTER_API_KEY is missing or empty")
        );
    }
}

#[tokio::test]
async fn disabled_store_refuses_explicit_saving_without_exposing_the_key() {
    let mut command = support::isolated_command(&support::workspace_root());
    command
        .args(["credentials", "save"])
        .env("OPENROUTER_API_KEY", "SYNTHETIC_EXPLICIT_SAVE");
    let output = tokio::time::timeout(Duration::from_secs(5), command.output())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("SUNSTACK_CREDENTIAL_STORE=off"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("SYNTHETIC_EXPLICIT_SAVE"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("SYNTHETIC_EXPLICIT_SAVE"));
}

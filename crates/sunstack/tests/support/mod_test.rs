#![allow(clippy::unwrap_used)]
use crate::support;
use std::ffi::OsStr;
use std::path::Path;

#[test]
fn process_test_commands_disable_the_real_credential_store() {
    let command = support::isolated_command(&support::workspace_root());
    assert!(command.as_std().get_envs().any(|(name, value)| {
        name == OsStr::new("SUNSTACK_CREDENTIAL_STORE") && value == Some(OsStr::new("off"))
    }));
}

#[test]
fn historical_binary_test_commands_keep_the_same_credential_isolation() {
    let binary = Path::new("/synthetic/baseline/sunstack");
    let command = support::isolated_command_with_binary(&support::workspace_root(), binary);
    assert_eq!(command.as_std().get_args().next(), Some(binary.as_os_str()));
    assert!(command.as_std().get_envs().any(|(name, value)| {
        name == OsStr::new("SUNSTACK_CREDENTIAL_STORE") && value == Some(OsStr::new("off"))
    }));
}

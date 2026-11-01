#![allow(clippy::unwrap_used)]
use crate::support;
use std::ffi::OsStr;

#[test]
fn process_test_commands_disable_the_real_credential_store() {
    let command = support::isolated_command(&support::workspace_root());
    assert!(command.as_std().get_envs().any(|(name, value)| {
        name == OsStr::new("SUNSTACK_CREDENTIAL_STORE") && value == Some(OsStr::new("off"))
    }));
}

#![allow(clippy::unwrap_used)]
mod support;

// The launch command is the boundary at which process readiness starts. A
// wrapper here measures signing work as application startup and fails this.
#[test]
fn timed_cli_launch_starts_the_signed_application_directly() {
    let command = support::isolated_command(&support::workspace_root());
    let executable = std::path::Path::new(command.as_std().get_program());
    assert_eq!(executable.file_name().unwrap(), "sunstack");
    assert!(executable.is_file());
    assert!(command.as_std().get_args().next().is_none());
}

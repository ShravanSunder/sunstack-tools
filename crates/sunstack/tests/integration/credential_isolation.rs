#![allow(clippy::unwrap_used)]
#[path = "../support/mod.rs"]
mod support;
use std::path::Path;

#[test]
fn all_binary_test_launches_use_the_isolated_command_owner() {
    fn check(directory: &Path, owner: &Path) {
        for item in std::fs::read_dir(directory).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                check(&path, owner);
            } else if path.extension().is_some_and(|extension| extension == "rs") && path != owner {
                let source = std::fs::read_to_string(&path).unwrap();
                let binary_reference = ["CARGO_BIN_EXE", "_sunstack"].concat();
                let runner_reference = ["cargo-debug-signing", "-runner.sh"].concat();
                assert!(
                    !source.contains(&binary_reference),
                    "{} must use support::isolated_command",
                    path.display()
                );
                assert!(
                    !source.contains(&runner_reference),
                    "{} must use support::isolated_command",
                    path.display()
                );
            }
        }
    }
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    check(&directory, &directory.join("support/mod.rs"));
}

#[cfg(test)]
#[path = "../support/mod_test.rs"]
mod command_builder_tests;

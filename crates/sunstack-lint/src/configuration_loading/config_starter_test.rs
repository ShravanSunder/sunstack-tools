#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn starter_refuses_existing_reserved_paths_without_touching_user_content() {
    for reserved in ["sunstack.lint.json", "sunstack-starter"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(reserved), "user content").unwrap();
        assert!(create_starter(root.path()).is_err());
        assert_eq!(
            std::fs::read_to_string(root.path().join(reserved)).unwrap(),
            "user content"
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}

#[test]
fn starter_refuses_an_existing_directory_and_repeated_init_preserves_edits() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("sunstack-starter")).unwrap();
    assert!(create_starter(root.path()).is_err());
    assert!(!root.path().join("sunstack.lint.json").exists());
    let fresh = tempfile::tempdir().unwrap();
    create_starter(fresh.path()).unwrap();
    let rules = fresh.path().join("sunstack-starter/rules.json");
    std::fs::write(&rules, "edited rules").unwrap();
    assert!(create_starter(fresh.path()).is_err());
    assert_eq!(std::fs::read_to_string(rules).unwrap(), "edited rules");
}

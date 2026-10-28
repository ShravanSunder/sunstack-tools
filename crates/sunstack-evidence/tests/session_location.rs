#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;
#[test]
fn recipe_roots_and_patterns_distinguish_missing_unique_and_ambiguous_sessions() {
    let root = tempfile::tempdir().unwrap();
    let recipe: ProviderRecipe = serde_json::from_value(serde_json::json!({"provider":"codex","locate":{"roots":[root.path()],"pattern":"**/rollout-*{session}.jsonl"},"read":{"format":"codex_rollout_jsonl"}})).unwrap();
    let session = SessionId::new("abc").unwrap();
    assert!(matches!(
        locate_session(&recipe, &session).unwrap(),
        SessionLookup::Unlocatable {
            reason: UnlocatableReason::Missing
        }
    ));
    std::fs::create_dir(root.path().join("nested")).unwrap();
    let first = root.path().join("nested/rollout-date-abc.jsonl");
    std::fs::write(&first, "{}\n").unwrap();
    assert!(
        matches!(locate_session(&recipe, &session).unwrap(), SessionLookup::Located { path, .. } if path == first.canonicalize().unwrap())
    );
    std::fs::write(root.path().join("rollout-other-abc.jsonl"), "{}\n").unwrap();
    assert!(matches!(
        locate_session(&recipe, &session).unwrap(),
        SessionLookup::Unlocatable {
            reason: UnlocatableReason::Ambiguous
        }
    ));
}

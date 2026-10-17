#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn real_actor_mutation_during_a_multi_file_git_capture_refuses_before_return() {
    let fixture = GitFixture::new();
    fixture.write("000-mutating.rs", "fn mutable() { value_0(); }\n");
    fixture.stage("000-mutating.rs");
    fixture.commit(1000);
    for index in 0..600 {
        fixture.write(
            &format!("later-{index:04}.rs"),
            &format!("fn later_{index}() {{}}\n{}", "// padding\n".repeat(100)),
        );
    }
    let stopped = Arc::new(AtomicBool::new(false));
    let writes = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(2));
    let path = fixture.directory.path().join("000-mutating.rs");
    // Publish unique complete versions; truncate/write would repeatedly expose empty bytes.
    let publication_directory = tempfile::tempdir().unwrap();
    let actor = {
        let stopped = stopped.clone();
        let writes = writes.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            let pending = publication_directory.path().join("next-version.rs");
            let mut sequence = 1usize;
            while !stopped.load(Ordering::Acquire) {
                std::fs::write(
                    &pending,
                    format!("fn mutable() {{ value_{sequence}(); }}\n"),
                )
                .unwrap();
                std::fs::rename(&pending, &path).unwrap();
                writes.store(sequence, Ordering::Release);
                sequence += 1;
            }
        })
    };
    barrier.wait();
    let mut limits = discovery_limits();
    limits.max_files = 2000.try_into().unwrap();
    let result = capture_change_selection(
        ChangeCaptureRequest {
            binding: fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged, DirtyLayer::Untracked],
            }),
            limits,
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await;
    stopped.store(true, Ordering::Release);
    actor.join().unwrap();
    assert!(
        writes.load(Ordering::Acquire) > 1,
        "the concurrent actor must actually write during acquisition"
    );
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!(
            "capture accepted while the actor published {} unique versions",
            writes.load(Ordering::Acquire)
        ),
    };
    assert_eq!(error.code, EvidenceCode::SourceChanged);
    assert_eq!(error.diagnostic().retry_advice, RetryAdvice::CaptureAgain);
}

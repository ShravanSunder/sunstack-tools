#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../support/mod.rs"]
mod support;
use std::{future::Future, time::Duration};
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
enum ComparisonWork {
    DefinitionLookup,
    ChangedUnitMapping,
}
fn assert_comparison_waits_for_blocking_worker(work: ComparisonWork) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async move {
        let fixture = GitFixture::new();
        fixture.write("source.ts", "const value = 1;\n");
        fixture.stage("source.ts");
        fixture.commit(1000);
        fixture.write("source.ts", "const value = 2;\n");
        let mut input = support::connected_inputs::plan(
            vec![fixture.binding(ChangeScope::Uncommitted {
                layers: vec![DirtyLayer::Unstaged],
            })],
            support::connected_inputs::related(RelationDirection::Callers),
            EvidenceRequirement::Required,
        );
        let declaration = input.declarations.remove(0);
        let mut session = EvidenceSession::open(
            input,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let outside_root = tempfile::tempdir().unwrap();
        let subject = match work {
            ComparisonWork::DefinitionLookup => ChangeComparisonSubject::DefinitionFile {
                path: outside_root.path().join("definition.json"),
            },
            ComparisonWork::ChangedUnitMapping => {
                let subject = ChangeComparisonSubject::Declaration { declaration };
                assert!(matches!(
                    session
                        .compare_change_sides(subject.clone(), ComparisonLevel::Path)
                        .await,
                    ChangeComparison::PathChanged { .. }
                ));
                subject
            }
        };
        let level = match work {
            ComparisonWork::DefinitionLookup => ComparisonLevel::Path,
            ComparisonWork::ChangedUnitMapping => ComparisonLevel::Content,
        };
        // Occupying the only blocking worker isolates the actual production offload.
        // Dropping release also unblocks this closure during assertion unwinding.
        let (release, held) = std::sync::mpsc::channel::<()>();
        let (occupied, started) = tokio::sync::oneshot::channel();
        let blocking_worker = tokio::task::spawn_blocking(move || {
            let _ = occupied.send(());
            let _ = held.recv();
        });
        tokio::time::timeout(Duration::from_secs(2), started)
            .await
            .unwrap()
            .unwrap();
        let (polling, polled) = tokio::sync::oneshot::channel();
        let comparison = tokio::spawn(async move {
            let comparison = session.compare_change_sides(subject, level);
            tokio::pin!(comparison);
            let mut polling = Some(polling);
            std::future::poll_fn(|context| {
                let result = comparison.as_mut().poll(context);
                if result.is_pending()
                    && let Some(polling) = polling.take()
                {
                    let _ = polling.send(());
                }
                result
            })
            .await
        });
        let yielded = tokio::time::timeout(Duration::from_secs(2), polled)
            .await
            .unwrap();
        assert!(
            yielded.is_ok(),
            "comparison completed without yielding to the occupied blocking worker"
        );
        // Pending was observed from the actual comparison future, not a task-start signal.
        assert!(
            !comparison.is_finished(),
            "comparison bypassed the occupied blocking worker"
        );
        tokio::time::timeout(Duration::from_secs(2), tokio::task::yield_now())
            .await
            .unwrap();
        assert!(!comparison.is_finished());
        drop(release);
        tokio::time::timeout(Duration::from_secs(2), blocking_worker)
            .await
            .unwrap()
            .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(2), comparison)
            .await
            .unwrap()
            .unwrap();
        match work {
            ComparisonWork::DefinitionLookup => assert!(matches!(
                result,
                ChangeComparison::NoBeforeSide {
                    reason: NoBeforeSideReason::OutsideChangeScopePaths
                }
            )),
            ComparisonWork::ChangedUnitMapping => {
                assert!(matches!(result, ChangeComparison::Unchanged {}))
            }
        }
    });
}
#[test]
fn definition_lookup_waits_for_the_blocking_worker_without_stalling_the_executor() {
    assert_comparison_waits_for_blocking_worker(ComparisonWork::DefinitionLookup);
}
#[test]
fn changed_unit_mapping_waits_for_the_blocking_worker_without_stalling_the_executor() {
    assert_comparison_waits_for_blocking_worker(ComparisonWork::ChangedUnitMapping);
}

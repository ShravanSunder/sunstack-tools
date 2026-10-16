#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use sunstack_evidence::*;
use support::connected_inputs::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn public_prepare_connects_changed_library_to_unchanged_full_scope_caller() {
    let library = GitFixture::new();
    library.write("src/lib.ts", "export function target() { return 1; }");
    library.stage("src/lib.ts");
    library.commit(1000);
    library.write("src/lib.ts", "export function target() { return 2; }");
    let caller = tempfile::tempdir().unwrap();
    std::fs::create_dir(caller.path().join("src")).unwrap();
    let caller_text =
        "import { target as alias } from '@library';\nfunction caller() { return alias(); }";
    std::fs::write(caller.path().join("src/app.ts"), caller_text).unwrap();
    let mut library_binding = library.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Unstaged],
    });
    library_binding.key = RepositoryId::new("library").unwrap();
    let caller_binding = RepositoryBinding {
        key: RepositoryId::new("caller").unwrap(),
        root: caller.path().into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "**/*.ts".into(),
        }],
        change_scope: ChangeScope::Full {},
    };
    let mut input = plan(
        vec![library_binding, caller_binding],
        related(RelationDirection::Callers),
        EvidenceRequirement::Required,
    );
    input.package_bindings.push(PackageSourceBinding {
        importing_repository_id: RepositoryId::new("caller").unwrap(),
        module_prefix: "@library".into(),
        target_repository_id: RepositoryId::new("library").unwrap(),
        target_path_prefix: "src/lib.ts".into(),
        language: "typescript".into(),
    });
    let mut exact_candidate_budget = input.clone();
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared {
        selections,
        measurements,
        ..
    } = outcome
    else {
        panic!("expected connected ready evidence: {outcome:?}");
    };
    let SelectionReadiness::Ready {
        content: SelectionContent::RelatedGroup { group },
        coverage,
        ..
    } = &selections[0].readiness
    else {
        panic!("expected one related group");
    };
    let ContextGroupOutcome::Complete { group, .. } = group.as_ref() else {
        panic!("required group incomplete");
    };
    assert!(
        group
            .excerpts
            .iter()
            .any(|excerpt| excerpt.text == "function caller() { return alias(); }")
    );
    assert!(
        group
            .excerpts
            .iter()
            .any(|excerpt| excerpt.text == "function target() { return 2; }")
    );
    assert!(
        matches!(coverage.as_ref(), SelectionCoverage::ChangeSet { sources, .. } if sources.iter().any(|source| matches!(source, SourceRef::RepositoryFile { repository, path, .. } if repository.as_str() == "caller" && path.to_str() == Some("src/app.ts"))))
    );
    assert_eq!(
        measurements.files_read, 3,
        "index, worktree and caller are distinct captured versions"
    );
    assert!(measurements.files_parsed >= 3 && measurements.bytes_scanned > 0);
    if let EvidenceDiscovery::Bounded { limits } = &mut exact_candidate_budget.discovery {
        limits.max_relationship_candidates = 1.try_into().unwrap();
    }
    let outcome = prepare_evidence(
        exact_candidate_budget,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(outcome, EvidencePlanOutcome::Prepared { .. }),
        "one requested caller candidate fits a budget of one; unrequested import edges must not consume it"
    );
}

#[tokio::test]
async fn public_full_scope_changed_units_refuses_instead_of_fabricating_seeds() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function unchanged() {}");
    let input = plan(
        vec![fixture.binding(ChangeScope::Full {})],
        EvidenceSelection::ChangedUnits {
            languages: vec![SourceLanguage::Typescript],
            unit_kinds: vec![UnitKindFilter::Function],
            source_sides: SourceSides::AfterOnly,
        },
        EvidenceRequirement::Required,
    );
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(outcome, EvidencePlanOutcome::Incomplete { diagnostics, .. } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::NoChangeSelection))
    );
}

#[tokio::test]
async fn public_deleted_before_unit_can_locate_a_current_full_scope_cross_repository_caller() {
    let library = GitFixture::new();
    library.write("target.ts", "export function target() { return 1; }");
    library.stage("target.ts");
    library.commit(1000);
    std::fs::remove_file(library.directory.path().join("target.ts")).unwrap();
    let caller = tempfile::tempdir().unwrap();
    std::fs::write(
        caller.path().join("caller.ts"),
        "import { target } from '@library'; function caller() { target(); }",
    )
    .unwrap();
    let mut library_binding = library.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Unstaged],
    });
    library_binding.key = RepositoryId::new("library").unwrap();
    let caller_binding = RepositoryBinding {
        key: RepositoryId::new("caller").unwrap(),
        root: caller.path().into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "*.ts".into(),
        }],
        change_scope: ChangeScope::Full {},
    };
    let mut selection = related(RelationDirection::Callers);
    if let EvidenceSelection::RelatedUnits { source_sides, .. } = &mut selection {
        *source_sides = SourceSides::BeforeOnly;
    }
    let mut input = plan(
        vec![library_binding, caller_binding],
        selection,
        EvidenceRequirement::Required,
    );
    input.package_bindings.push(PackageSourceBinding {
        importing_repository_id: RepositoryId::new("caller").unwrap(),
        module_prefix: "@library".into(),
        target_repository_id: RepositoryId::new("library").unwrap(),
        target_path_prefix: "target.ts".into(),
        language: "typescript".into(),
    });
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared {
        selections,
        change_selections,
        ..
    } = outcome
    else {
        panic!("deleted source must keep its before-side relationship: {outcome:?}");
    };
    assert!(
        change_selections[0].members[0].changed_paths[0]
            .after
            .is_none()
    );
    let SelectionReadiness::Ready {
        content: SelectionContent::RelatedGroup { group },
        ..
    } = &selections[0].readiness
    else {
        panic!("deleted unit group missing");
    };
    let ContextGroupOutcome::Complete { group, .. } = group.as_ref() else {
        panic!("before-side group incomplete");
    };
    assert!(
        group
            .excerpts
            .iter()
            .any(|excerpt| excerpt.text == "function target() { return 1; }"
                && matches!(
                    excerpt.unit.revision,
                    SourceRevision::Index {
                        side: SourceSide::Before,
                        ..
                    }
                ))
    );
    assert!(
        group
            .excerpts
            .iter()
            .any(|excerpt| excerpt.text == "function caller() { target(); }"
                && matches!(
                    excerpt.unit.revision,
                    SourceRevision::Worktree {
                        side: SourceSide::After,
                        ..
                    }
                ))
    );
}

#[tokio::test]
async fn public_source_failure_after_admission_keeps_successful_repository_attribution() {
    let library = GitFixture::new();
    library.write("target.ts", "export function target() { return 1; }");
    library.stage("target.ts");
    library.commit(1000);
    library.write("target.ts", "export function target() { return 2; }");
    let parent = tempfile::tempdir().unwrap();
    let caller_root = parent.path().join("caller");
    std::fs::create_dir(&caller_root).unwrap();
    std::fs::write(
        caller_root.join("caller.ts"),
        "import { target } from '@library'; function caller() { target(); }",
    )
    .unwrap();
    let mut library_binding = library.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Unstaged],
    });
    library_binding.key = RepositoryId::new("library").unwrap();
    let caller_binding = RepositoryBinding {
        key: RepositoryId::new("caller").unwrap(),
        root: caller_root.clone(),
        input_paths: vec![QueryPath::Glob {
            pattern: "*.ts".into(),
        }],
        change_scope: ChangeScope::Full {},
    };
    let mut input = plan(
        vec![library_binding, caller_binding],
        related(RelationDirection::Callers),
        EvidenceRequirement::Optional,
    );
    input.package_bindings.push(PackageSourceBinding {
        importing_repository_id: RepositoryId::new("caller").unwrap(),
        module_prefix: "@library".into(),
        target_repository_id: RepositoryId::new("library").unwrap(),
        target_path_prefix: "target.ts".into(),
        language: "typescript".into(),
    });
    let mut session = EvidenceSession::open(
        input.clone(),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    std::fs::rename(&caller_root, parent.path().join("moved-caller")).unwrap();
    let selected = session.select(&input.declarations[0]).await;
    let SelectionReadiness::Ready {
        content: SelectionContent::RelatedGroup { group },
        coverage,
        ..
    } = selected.readiness
    else {
        panic!("optional partial repository evidence missing");
    };
    let ContextGroupOutcome::Partial {
        group, diagnostics, ..
    } = group.as_ref()
    else {
        panic!("unavailable caller must be explicit");
    };
    assert!(
        group
            .excerpts
            .iter()
            .any(|excerpt| excerpt.text == "function target() { return 2; }")
    );
    assert!(diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::SourceUnavailable && matches!(&diagnostic.location, DiagnosticLocation::Field { field_path } if field_path == &["repositories", "caller"])));
    assert!(
        matches!(coverage.as_ref(), SelectionCoverage::ChangeSet { repository, sources, discovery: DiscoveryCoverage::Unavailable { .. } } if repository.as_str() == "library" && sources.iter().all(|source| matches!(source, SourceRef::RepositoryFile { repository, .. } if repository.as_str() == "library")))
    );
    let summary = session.close();
    assert_eq!(summary.measurements.files_read, 2);
    assert_eq!(summary.change_selections.len(), 1);
    let error = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(
        error.code,
        EvidenceCode::SourceUnavailable,
        "the same missing root is a preflight refusal before admission"
    );
}

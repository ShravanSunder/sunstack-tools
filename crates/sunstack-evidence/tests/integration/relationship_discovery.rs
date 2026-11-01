#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../support/mod.rs"]
mod support;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn real_cross_repository_alias_caller_and_shadow_contradiction() {
    let library = GitFixture::new();
    library.write("src/lib.ts", "export function target() { return 1; }");
    library.stage("src/lib.ts");
    library.commit(1000);
    library.write("src/lib.ts", "export function target() { return 2; }");
    let app = GitFixture::new();
    app.write("src/app.ts", "import { target as alias } from '@library';\nfunction caller() { return alias(); }\nfunction shadow(alias: () => number) { return alias(); }");
    let mut library_binding = library.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Unstaged],
    });
    library_binding.key = RepositoryId::new("library").unwrap();
    let mut app_binding = app.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Untracked],
    });
    app_binding.key = RepositoryId::new("app").unwrap();
    let captures = vec![capture(library_binding).await, capture(app_binding).await];
    let seeds = changed_source_units(
        &captures[0],
        &[SourceLanguage::Typescript],
        &[UnitKindFilter::Function],
        SourceSides::AfterOnly,
    )
    .unwrap();
    assert_eq!(seeds.len(), 1);
    let query = query(
        vec![
            RepositoryId::new("library").unwrap(),
            RepositoryId::new("app").unwrap(),
        ],
        RelationDirection::Callers,
    );
    let mapping = PackageSourceBinding {
        importing_repository_id: RepositoryId::new("app").unwrap(),
        module_prefix: "@library".into(),
        target_repository_id: RepositoryId::new("library").unwrap(),
        target_path_prefix: "src/lib.ts".into(),
        language: "typescript".into(),
    };
    let discovery = discover_relationships(RelationshipDiscoveryRequest {
        captures: &captures,
        seeds: &seeds,
        query: &query,
        package_bindings: &[mapping],
        limits: &discovery_limits(),
        cancellation: CancellationToken::new(),
    })
    .unwrap();
    assert!(
        discovery
            .units
            .iter()
            .any(|unit| unit.text.starts_with("function caller"))
    );
    assert!(discovery.relationships.iter().any(|relation| matches!(relation, RelationshipCandidate::Candidate { source_unit, target_unit, facts, .. } if source_unit.repository.as_str() == "app" && target_unit.repository.as_str() == "library" && facts.iter().any(|fact| matches!(fact, RelationshipFact::PackageMapping { .. })))));
    assert!(discovery.relationships.iter().any(|relation| matches!(
        relation,
        RelationshipCandidate::Contradicted {
            contradiction: RelationshipContradiction::LexicalShadow,
            ..
        }
    )));
    assert!(
        !discovery
            .units
            .iter()
            .any(|unit| unit.text.starts_with("function shadow"))
    );
}

#[tokio::test]
async fn real_static_import_directions_cycle_and_dynamic_uncertainty() {
    let fixture = GitFixture::new();
    fixture.write(
        "src/a.ts",
        "import run from './b';\nfunction entry() { run(); entry(); receiver.run(); }",
    );
    fixture.write(
        "src/b.ts",
        "export default function execute() { return 1; }",
    );
    let captured = capture(fixture.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Untracked],
    }))
    .await;
    let seeds = changed_source_units(
        &captured,
        &[SourceLanguage::Typescript],
        &[UnitKindFilter::Function],
        SourceSides::AfterOnly,
    )
    .unwrap()
    .into_iter()
    .filter(|unit| unit.text.starts_with("function entry"))
    .collect::<Vec<_>>();
    let captures = [captured];
    for direction in [RelationDirection::Callees, RelationDirection::Imports] {
        let query = query(vec![RepositoryId::new("fixture").unwrap()], direction);
        let discovery = discover_relationships(RelationshipDiscoveryRequest {
            captures: &captures,
            seeds: &seeds,
            query: &query,
            package_bindings: &[],
            limits: &discovery_limits(),
            cancellation: CancellationToken::new(),
        })
        .unwrap();
        assert_eq!(discovery.units.len(), 2);
        assert!(discovery.relationships.iter().any(|relation| matches!(relation, RelationshipCandidate::Candidate { relation_kind, .. } if *relation_kind == direction)));
        if direction == RelationDirection::Callees {
            assert!(discovery.relationships.iter().any(|relation| matches!(
                relation,
                RelationshipCandidate::Unresolved {
                    reason: RelationshipUncertainty::DynamicDispatch,
                    ..
                }
            )));
        }
    }
}
async fn capture(binding: RepositoryBinding) -> CapturedChangeSelection {
    capture_change_selection(
        ChangeCaptureRequest {
            binding,
            limits: discovery_limits(),
            anchor_utc: chrono::Utc::now(),
        },
        CancellationToken::new(),
    )
    .await
    .unwrap()
}
fn query(repository_ids: Vec<RepositoryId>, direction: RelationDirection) -> RelationshipQuery {
    RelationshipQuery {
        source_units: SourceUnitQuery {
            seeds: SourceUnitSeeds::ChangedUnits {
                repository_ids: repository_ids.clone(),
            },
            languages: vec![SourceLanguage::Typescript],
            unit_kinds: vec![UnitKindFilter::Function],
            source_sides: SourceSides::AfterOnly,
        },
        repository_ids,
        directions: vec![direction],
        requested_hops: 3.try_into().unwrap(),
    }
}

#[tokio::test]
async fn real_rust_alias_static_path_and_inverse_import_are_located() {
    let fixture = GitFixture::new();
    fixture.write("src/lib.rs", "mod helpers; use crate::helpers::{target as alias}; struct Thing; impl Thing { fn static_run() {} } fn entry() { alias(); crate::helpers::target(); Thing::static_run(); }");
    fixture.write("src/helpers.rs", "pub fn target() {}\n");
    let captured = capture(fixture.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Untracked],
    }))
    .await;
    let all = changed_source_units(
        &captured,
        &[SourceLanguage::Rust],
        &[UnitKindFilter::Function, UnitKindFilter::Method],
        SourceSides::AfterOnly,
    )
    .unwrap();
    let captures = [captured];
    for direction in [
        RelationDirection::Callees,
        RelationDirection::Imports,
        RelationDirection::Callers,
        RelationDirection::ImportedBy,
    ] {
        let seeds = all
            .iter()
            .filter(|unit| {
                if matches!(
                    direction,
                    RelationDirection::Callers | RelationDirection::ImportedBy
                ) {
                    unit.text.starts_with("pub fn target")
                } else {
                    unit.text.starts_with("fn entry")
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut query = query(vec![RepositoryId::new("fixture").unwrap()], direction);
        query.source_units.languages = vec![SourceLanguage::Rust];
        let discovery = discover_relationships(RelationshipDiscoveryRequest {
            captures: &captures,
            seeds: &seeds,
            query: &query,
            package_bindings: &[],
            limits: &discovery_limits(),
            cancellation: CancellationToken::new(),
        })
        .unwrap();
        assert!(
            discovery
                .units
                .iter()
                .any(|unit| unit.text.starts_with("fn entry"))
        );
        assert!(
            discovery
                .units
                .iter()
                .any(|unit| unit.text.starts_with("pub fn target"))
        );
        if direction == RelationDirection::Callees {
            assert!(
                discovery
                    .units
                    .iter()
                    .any(|unit| unit.text.starts_with("fn static_run"))
            );
        }
        assert!(discovery.relationships.iter().any(|relationship| matches!(relationship, RelationshipCandidate::Candidate { relation_kind, .. } if *relation_kind == direction)));
    }
}

#[tokio::test]
async fn ambiguity_namespace_imports_and_tsx_receiver_calls_stay_explicit() {
    let fixture = GitFixture::new();
    fixture.write("view.tsx", "import * as namespace from './other'; function repeated() {} function repeated() {} function View() { repeated(); namespace.run(); return <div/>; }");
    fixture.write("other.ts", "export function run() {}");
    let captured = capture(fixture.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Untracked],
    }))
    .await;
    let seeds = changed_source_units(
        &captured,
        &[SourceLanguage::Tsx],
        &[UnitKindFilter::Function],
        SourceSides::AfterOnly,
    )
    .unwrap()
    .into_iter()
    .filter(|unit| unit.text.starts_with("function View"))
    .collect::<Vec<_>>();
    let captures = [captured];
    let mut query = query(
        vec![RepositoryId::new("fixture").unwrap()],
        RelationDirection::Callees,
    );
    query.directions.push(RelationDirection::Imports);
    query.source_units.languages = vec![SourceLanguage::Tsx];
    let discovery = discover_relationships(RelationshipDiscoveryRequest {
        captures: &captures,
        seeds: &seeds,
        query: &query,
        package_bindings: &[],
        limits: &discovery_limits(),
        cancellation: CancellationToken::new(),
    })
    .unwrap();
    assert_eq!(discovery.relationships.iter().filter(|relationship| matches!(relationship, RelationshipCandidate::Candidate { uncertainties, .. } if uncertainties.contains(&RelationshipUncertainty::AmbiguousDeclaration))).count(), 2);
    assert!(discovery.relationships.iter().any(|relationship| matches!(
        relationship,
        RelationshipCandidate::Unresolved {
            reason: RelationshipUncertainty::UnsupportedImport,
            ..
        }
    )));
    assert!(discovery.relationships.iter().any(|relationship| matches!(
        relationship,
        RelationshipCandidate::Unresolved {
            reason: RelationshipUncertainty::DynamicDispatch,
            ..
        }
    )));
}

#[tokio::test]
async fn conditional_rust_modules_are_unresolved_even_when_the_source_file_exists() {
    let fixture = GitFixture::new();
    fixture.write("src/lib.rs", "#[cfg(feature = \"not-declared\")] mod conditional;\nfn entry() { conditional::target(); }");
    fixture.write("src/conditional.rs", "pub fn target() {}");
    let captured = capture(fixture.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Untracked],
    }))
    .await;
    let seeds = changed_source_units(
        &captured,
        &[SourceLanguage::Rust],
        &[UnitKindFilter::Function],
        SourceSides::AfterOnly,
    )
    .unwrap()
    .into_iter()
    .filter(|unit| unit.text.starts_with("fn entry"))
    .collect::<Vec<_>>();
    let captures = [captured];
    let mut query = query(
        vec![RepositoryId::new("fixture").unwrap()],
        RelationDirection::Callees,
    );
    query.source_units.languages = vec![SourceLanguage::Rust];
    let discovery = discover_relationships(RelationshipDiscoveryRequest {
        captures: &captures,
        seeds: &seeds,
        query: &query,
        package_bindings: &[],
        limits: &discovery_limits(),
        cancellation: CancellationToken::new(),
    })
    .unwrap();
    assert_eq!(discovery.units.len(), 1);
    assert!(discovery.relationships.iter().any(|relationship| matches!(relationship, RelationshipCandidate::Unresolved { target_hint, .. } if target_hint == "conditional::target")));
}

#[tokio::test]
async fn rust_relative_imports_use_the_logical_module_directory() {
    for module_file in ["src/foo.rs", "src/foo/mod.rs"] {
        let fixture = GitFixture::new();
        fixture.write(module_file, "use self::child::target as child_alias; use super::sibling::target as sibling_alias; fn entry() { child_alias(); sibling_alias(); }");
        fixture.write("src/foo/child.rs", "pub fn target() {}\n");
        fixture.write("src/sibling.rs", "pub fn target() {}\n");
        let captured = capture(fixture.binding(ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Untracked],
        }))
        .await;
        let seeds = changed_source_units(
            &captured,
            &[SourceLanguage::Rust],
            &[UnitKindFilter::Function],
            SourceSides::AfterOnly,
        )
        .unwrap()
        .into_iter()
        .filter(|unit| unit.text.starts_with("fn entry"))
        .collect::<Vec<_>>();
        let captures = [captured];
        let mut query = query(
            vec![RepositoryId::new("fixture").unwrap()],
            RelationDirection::Callees,
        );
        query.source_units.languages = vec![SourceLanguage::Rust];
        let discovered = discover_relationships(RelationshipDiscoveryRequest {
            captures: &captures,
            seeds: &seeds,
            query: &query,
            package_bindings: &[],
            limits: &discovery_limits(),
            cancellation: CancellationToken::new(),
        })
        .unwrap();
        let paths = discovered
            .units
            .iter()
            .map(|unit| unit.unit.key.path.to_string_lossy().into_owned())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            paths.contains("src/foo/child.rs"),
            "missing self child for {module_file}: {paths:?}"
        );
        assert!(
            paths.contains("src/sibling.rs"),
            "missing super sibling for {module_file}: {paths:?}"
        );
    }
}

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use sunstack_evidence::*;
use support::connected_inputs::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn public_connected_admission_refuses_duplicate_keys_and_canonical_roots() {
    let first = GitFixture::new();
    first.write("source.ts", "function chosen() {}");
    let second = tempfile::tempdir().unwrap();
    let first_binding = first.binding(ChangeScope::Uncommitted {
        layers: vec![DirtyLayer::Untracked],
    });
    let second_binding = RepositoryBinding {
        key: RepositoryId::new("second").unwrap(),
        root: second.path().into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "*.ts".into(),
        }],
        change_scope: ChangeScope::Full,
    };
    let valid = plan(
        vec![first_binding.clone(), second_binding.clone()],
        EvidenceSelection::ChangeSetLiteralText {
            paths: vec![QueryPath::Glob {
                pattern: "*.ts".into(),
            }],
            text: "chosen".into(),
        },
        EvidenceRequirement::Required,
    );
    let control = prepare_evidence(
        valid.clone(),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(control, EvidencePlanOutcome::Prepared { selections, .. } if matches!(&selections[0].readiness, SelectionReadiness::Ready { content: SelectionContent::Matches { matches }, .. } if matches.len() == 1))
    );
    for duplicate_key in [true, false] {
        let mut invalid = valid.clone();
        if duplicate_key {
            invalid.repositories[1].key = first_binding.key.clone();
        } else {
            invalid.repositories[1].root = first_binding.root.join(".");
        }
        let outcome = prepare_evidence(
            invalid,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        assert!(
            matches!(outcome, EvidencePlanOutcome::Refused { diagnostics } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::InvalidSelector)),
            "duplicate binding identity must refuse at admission"
        );
    }
}

#[tokio::test]
async fn public_package_discovery_keeps_unmapped_and_excluded_targets_unresolved() {
    let caller = GitFixture::new();
    caller.write(
        "caller.ts",
        "import { target } from '@external'; function caller() { return target(); }",
    );
    let provider = tempfile::tempdir().unwrap();
    std::fs::create_dir(provider.path().join("private")).unwrap();
    std::fs::write(
        provider.path().join("visible.ts"),
        "export function visible() {}",
    )
    .unwrap();
    std::fs::write(
        provider.path().join("private/target.ts"),
        "export function target() { return 'excluded-source-marker'; }",
    )
    .unwrap();
    for mode in ["mapped", "unmapped", "excluded"] {
        let mut allowed = vec![QueryPath::Exact {
            path: "visible.ts".into(),
        }];
        if mode != "excluded" {
            allowed.push(QueryPath::Exact {
                path: "private/target.ts".into(),
            });
        }
        let provider_binding = RepositoryBinding {
            key: RepositoryId::new("provider").unwrap(),
            root: provider.path().into(),
            input_paths: allowed,
            change_scope: ChangeScope::Full,
        };
        let mut input = plan(
            vec![
                caller.binding(ChangeScope::Uncommitted {
                    layers: vec![DirtyLayer::Untracked],
                }),
                provider_binding,
            ],
            related(RelationDirection::Imports),
            EvidenceRequirement::Optional,
        );
        if mode != "unmapped" {
            input.package_bindings.push(PackageSourceBinding {
                importing_repository_id: RepositoryId::new("fixture").unwrap(),
                module_prefix: "@external".into(),
                target_repository_id: RepositoryId::new("provider").unwrap(),
                target_path_prefix: "private/target.ts".into(),
                language: "typescript".into(),
            });
        }
        let outcome = prepare_evidence(
            input.clone(),
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        let wire = serde_json::to_string(&outcome).unwrap();
        let EvidencePlanOutcome::Prepared {
            selections,
            captures,
            measurements,
            ..
        } = outcome
        else {
            panic!("optional evidence should remain labelled: {mode}: {outcome:?}")
        };
        let SelectionReadiness::Ready {
            content: SelectionContent::RelatedGroup { group },
            coverage,
            ..
        } = &selections[0].readiness
        else {
            panic!("related group missing: {mode}")
        };
        if mode == "excluded" {
            assert_eq!(
                measurements.files_read, 2,
                "only caller and allowed visible context may be read"
            );
            assert!(
                !wire.contains("excluded-source-marker"),
                "excluded source bytes must never enter evidence"
            );
            assert!(
                captures
                    .iter()
                    .all(|capture| !is_private_target(&capture.source))
            );
            assert!(
                matches!(coverage.as_ref(), SelectionCoverage::ChangeSet { sources, .. } if sources.iter().all(|source| !is_private_target(source)))
            );
        }
        if mode == "mapped" {
            let ContextGroupOutcome::Complete { group, .. } = group.as_ref() else {
                panic!("explicit in-scope mapping must resolve")
            };
            assert!(
                group.excerpts.iter().any(|excerpt| excerpt.text
                    == "function target() { return 'excluded-source-marker'; }")
            );
            assert!(group.relationships.iter().any(|relationship| matches!(relationship, RelationshipCandidate::Candidate { target_unit, relation_kind: RelationDirection::Imports, .. } if target_unit.repository.as_str() == "provider" && target_unit.path.to_str() == Some("private/target.ts"))));
        } else {
            let ContextGroupOutcome::Partial {
                group, diagnostics, ..
            } = group.as_ref()
            else {
                panic!("unresolved package must remain partial: {mode}")
            };
            assert!(
                diagnostics
                    .items
                    .iter()
                    .any(|diagnostic| diagnostic.code == EvidenceCode::RelationshipUnresolved)
            );
            assert!(group.relationships.iter().any(|relationship| matches!(relationship, RelationshipCandidate::Unresolved { target_hint, reason: RelationshipUncertainty::UnresolvedPackage, .. } if target_hint == "@external")));
            assert!(!group.relationships.iter().any(|relationship| matches!(relationship, RelationshipCandidate::Candidate { target_unit, .. } if target_unit.repository.as_str() == "provider")));
            assert!(
                group
                    .excerpts
                    .iter()
                    .all(|excerpt| excerpt.unit.repository.as_str() != "provider")
            );
            input.declarations[0].requirement = EvidenceRequirement::Required;
            let required = prepare_evidence(
                input,
                AdapterRegistry::builtins().unwrap(),
                CancellationToken::new(),
            )
            .await;
            assert!(
                matches!(required, EvidencePlanOutcome::Incomplete { diagnostics, .. } if diagnostics.items.iter().any(|diagnostic| diagnostic.code == EvidenceCode::RelationshipUnresolved))
            );
        }
    }
}
fn is_private_target(source: &SourceRef) -> bool {
    matches!(source, SourceRef::RepositoryFile { repository, path, .. } if repository.as_str() == "provider" && path.to_str() == Some("private/target.ts"))
}

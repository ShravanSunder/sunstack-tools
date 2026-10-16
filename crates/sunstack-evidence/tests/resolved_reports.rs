#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use serde_json::json;
use sunstack_evidence::*;
use support::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn public_unknown_recency_policy_is_nonfatal_and_exclusions_freeze_no_sources() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function old() {}");
    fixture.stage("source.ts");
    fixture.commit(1000);
    fixture.write("source.ts", "function current() {}");
    future_mtime(&fixture);
    for policy in [
        UnknownRecencyPolicy::IncludeWithDiagnostic,
        UnknownRecencyPolicy::ExcludeWithOmission,
    ] {
        let scope = recent(policy);
        let outcome = prepare_evidence(
            request(&fixture, scope),
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        let EvidencePlanOutcome::Prepared {
            selections,
            captures,
            change_selections,
            measurements,
            ..
        } = outcome
        else {
            panic!("explicit unknown policy must remain ready: {outcome:?}");
        };
        assert_eq!(change_selections.len(), 1);
        let report = &change_selections[0];
        let member = &report.members[0];
        assert_eq!(member.scope_position, 0);
        let ResolvedChangeBounds::RecentUncommitted {
            since_utc,
            until_utc,
            layers,
            ..
        } = &member.bounds
        else {
            panic!("resolved recent bounds missing");
        };
        assert_eq!(
            ((*until_utc - *since_utc).num_seconds(), *until_utc),
            (60, report.anchor_utc)
        );
        assert_eq!(layers, &[DirtyLayer::Unstaged]);
        let SelectionReadiness::Ready {
            content: SelectionContent::ChangedUnits { units },
            coverage,
            ..
        } = &selections[0].readiness
        else {
            panic!("changed units not ready");
        };
        assert!(
            matches!(coverage.as_ref(), SelectionCoverage::ChangeSet { repository, .. } if repository == &report.repository)
        );
        match policy {
            UnknownRecencyPolicy::IncludeWithDiagnostic => {
                assert_eq!(units.len(), 1);
                assert_eq!(units[0].text, "function current() {}");
                assert!(
                    matches!(&member.recency, ChangeRecencyBasis::ApproximateFileTime { unknown: UnknownFileRecency::Included { paths } } if paths == &[std::path::PathBuf::from("source.ts")])
                );
                assert_eq!(member.diagnostics.items.len(), 1);
                assert_eq!(
                    member.diagnostics.items[0].code,
                    EvidenceCode::RecencyUnknown
                );
                assert_eq!(captures.len(), 2);
                assert_eq!(measurements.files_read, 2);
                assert_eq!(measurements.files_parsed, 2);
            }
            UnknownRecencyPolicy::ExcludeWithOmission => {
                assert!(units.is_empty());
                assert!(member.changed_paths.is_empty());
                assert!(matches!(
                    member.recency,
                    ChangeRecencyBasis::ApproximateFileTime {
                        unknown: UnknownFileRecency::Excluded { count: 1 }
                    }
                ));
                assert!(captures.is_empty());
                assert_eq!(measurements.files_read, 0);
                assert!(
                    matches!(coverage.as_ref(), SelectionCoverage::ChangeSet { sources, .. } if sources.is_empty())
                );
            }
        }
    }
}

#[tokio::test]
async fn public_union_reports_bounds_member_order_and_root_commit_provenance() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function committed() {}");
    fixture.stage("source.ts");
    let root = fixture.commit(1000);
    fixture.write("source.ts", "function current() {}");
    future_mtime(&fixture);
    let scope = ChangeScope::Union {
        members: vec![
            recent(UnknownRecencyPolicy::IncludeWithDiagnostic),
            ChangeScope::CommitSet {
                commit_ids: vec![GitObjectId::new(root.to_string()).unwrap()],
                parent_policy: CommitParentPolicy::EachParent,
            },
        ],
    };
    let outcome = prepare_evidence(
        request(&fixture, scope),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared {
        change_selections, ..
    } = outcome
    else {
        panic!("union should prepare: {outcome:?}");
    };
    let report = &change_selections[0];
    assert_eq!(report.members.len(), 2);
    assert_eq!(
        report
            .members
            .iter()
            .map(|member| member.scope_position)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(matches!(
        report.members[0].bounds,
        ResolvedChangeBounds::RecentUncommitted { .. }
    ));
    assert!(
        matches!(&report.members[1].bounds, ResolvedChangeBounds::CommitSet { commit_ids, parent_policy: CommitParentPolicy::EachParent } if commit_ids[0].as_str() == root.to_string())
    );
    assert!(
        matches!(&report.members[1].changed_paths[0].provenance, ChangeMemberBasis::Commit { commit_id, parent: ParentBasis::Root } if commit_id.as_str() == root.to_string())
    );
    assert!(report.members[1].changed_paths[0].before.is_none());
    assert!(
        matches!(&report.members[1].changed_paths[0].after, Some(SourceRef::RepositoryFile { revision: SourceRevision::Commit { commit_id, side: SourceSide::After }, .. }) if commit_id.as_str() == root.to_string())
    );
}

#[tokio::test]
async fn public_two_repository_windows_share_one_invocation_anchor() {
    let first = GitFixture::new();
    let second = GitFixture::new();
    for fixture in [&first, &second] {
        fixture.write("source.ts", "function old() {}");
        fixture.stage("source.ts");
        fixture.commit(1000);
        fixture.write("source.ts", "function changed() {}");
        future_mtime(fixture);
    }
    let mut input = request(&first, recent(UnknownRecencyPolicy::IncludeWithDiagnostic));
    let mut second_binding = second.binding(recent(UnknownRecencyPolicy::IncludeWithDiagnostic));
    second_binding.key = RepositoryId::new("second").unwrap();
    input.repositories.push(second_binding);
    input.sources.insert(
        SourceName::new("second_changes").unwrap(),
        SourceSpec::ChangeSet {
            repository: RepositoryId::new("second").unwrap(),
        },
    );
    let mut declaration = input.declarations[0].clone();
    declaration.id = DeclarationId::new("second_selection").unwrap();
    declaration.input = SourceName::new("second_changes").unwrap();
    input.declarations.push(declaration);
    let outcome = prepare_evidence(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared {
        change_selections, ..
    } = outcome
    else {
        panic!("two repositories not prepared: {outcome:?}");
    };
    assert_eq!(change_selections.len(), 2);
    assert_eq!(
        change_selections[0].anchor_utc,
        change_selections[1].anchor_utc
    );
    let bounds = change_selections
        .iter()
        .map(|report| match &report.members[0].bounds {
            ResolvedChangeBounds::RecentUncommitted {
                since_utc,
                until_utc,
                ..
            } => (*since_utc, *until_utc),
            _ => panic!("window missing"),
        })
        .collect::<Vec<_>>();
    assert_eq!(bounds[0], bounds[1]);
}
fn recent(policy: UnknownRecencyPolicy) -> ChangeScope {
    ChangeScope::RecentUncommitted {
        layers: vec![DirtyLayer::Unstaged],
        window: TimeWindow::Lookback {
            seconds: 60.try_into().unwrap(),
        },
        unknown_recency: policy,
    }
}
fn future_mtime(fixture: &GitFixture) {
    std::fs::File::options()
        .write(true)
        .open(fixture.directory.path().join("source.ts"))
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(3600)),
        )
        .unwrap();
}
fn request(fixture: &GitFixture, scope: ChangeScope) -> EvidencePlanInput {
    serde_json::from_value(json!({ "repositories": [fixture.binding(scope)], "sources": {"changes": {"kind": "change_set", "repository": "fixture"}}, "payload_schemas": [], "cursors": [], "declarations": [{"id":"changed", "input":"changes", "select":{"kind":"changed_units", "languages":["typescript"], "unit_kinds":["function"], "source_sides":"after_only"}}], "native_queries": [], "package_bindings": [], "limits":{"max_files":100,"max_file_bytes":1000000,"max_total_bytes":10000000,"max_matches":100,"max_transcript_bytes":100000,"include_ignored":false}, "discovery":{"kind":"bounded","limits":discovery_limits()} })).unwrap()
}

struct CancelAfterGitCapture {
    format: TranscriptFormatName,
    cancellation: CancellationToken,
}
impl TranscriptAdapter for CancelAfterGitCapture {
    fn format_name(&self) -> &TranscriptFormatName {
        &self.format
    }
    fn normalize(&self, capture: &CapturedTranscript<'_>) -> AdapterOutput {
        assert_eq!(capture.bytes, b"cancel after the completed Git selection\n");
        self.cancellation.cancel();
        AdapterOutput::default()
    }
}
#[tokio::test]
async fn public_cancelled_admitted_partial_retains_the_completed_change_report() {
    let fixture = GitFixture::new();
    fixture.write("source.ts", "function committed() {}");
    fixture.stage("source.ts");
    fixture.commit(1000);
    fixture.write("source.ts", "function current() {}");
    let mut input = request(
        &fixture,
        ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Unstaged],
        },
    );
    let transcript = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        transcript.path(),
        b"cancel after the completed Git selection\n",
    )
    .unwrap();
    let format = TranscriptFormatName::new("cancel-after-git").unwrap();
    input.sources.insert(
        SourceName::new("cancel_source").unwrap(),
        SourceSpec::Conversation {
            transcript_path: transcript.path().into(),
            format: format.clone(),
            final_assistant_text: None,
        },
    );
    input.declarations.push(EvidenceDeclaration {
        id: DeclarationId::new("cancel_selection").unwrap(),
        input: SourceName::new("cancel_source").unwrap(),
        requirement: EvidenceRequirement::Optional,
        select: EvidenceSelection::Conversation {
            turns: TurnWindow::All,
            user_messages: Inclusion::Included,
            assistant_messages: AssistantSelection::All,
            tool_use: ToolUseSelection::Excluded,
            hook_injected: Inclusion::Excluded,
        },
        max_items: None,
        max_bytes: None,
    });
    let cancellation = CancellationToken::new();
    let registry = AdapterRegistry::builtins()
        .unwrap()
        .with_adapter(std::sync::Arc::new(CancelAfterGitCapture {
            format,
            cancellation: cancellation.clone(),
        }))
        .unwrap();
    let outcome = prepare_evidence(input, registry, cancellation).await;
    let EvidencePlanOutcome::Cancelled {
        progress: EvidenceCancellationProgress::PlanAdmitted { partial },
        ..
    } = outcome
    else {
        panic!("expected cancellation after admitted Git selection: {outcome:?}");
    };
    assert_eq!(partial.change_selections.len(), 1);
    assert_eq!(
        partial.change_selections[0].members[0].changed_paths.len(),
        1
    );
    assert!(
        matches!(&partial.selections[0].readiness, SelectionReadiness::Ready { content: SelectionContent::ChangedUnits { units }, .. } if units[0].text == "function current() {}")
    );
    assert_eq!(partial.captures.len(), 2);
}

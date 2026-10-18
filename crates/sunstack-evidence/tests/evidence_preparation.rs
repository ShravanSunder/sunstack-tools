#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::{collections::BTreeMap, num::NonZeroUsize, path::Path};
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;
fn positive(value: usize) -> NonZeroUsize {
    value.try_into().unwrap()
}
fn input(root: &Path) -> EvidencePlanInput {
    EvidencePlanInput {
        repositories: vec![RepositoryBinding {
            key: RepositoryId::new("repo").unwrap(),
            root: root.into(),
            input_paths: vec![QueryPath::Glob {
                pattern: "**".into(),
            }],
            change_scope: ChangeScope::Full {},
        }],
        sources: BTreeMap::from([(
            SourceName::new("skill").unwrap(),
            SourceSpec::Document {
                repository: RepositoryId::new("repo").unwrap(),
                path: "skill.md".into(),
            },
        )]),
        payload_schemas: vec![],
        cursors: vec![],
        declarations: vec![declaration(
            "target",
            EvidenceSelection::Paragraph {
                target_id: TargetId::new("U001").unwrap(),
            },
        )],
        native_queries: vec![],
        package_bindings: vec![],
        limits: InvocationLimits {
            max_files: positive(10),
            max_file_bytes: positive(10000),
            max_total_bytes: positive(100000),
            max_matches: positive(100),
            max_transcript_bytes: positive(10000),
            include_ignored: false,
        },
        discovery: EvidenceDiscovery::None {},
    }
}
fn declaration(id: &str, select: EvidenceSelection) -> EvidenceDeclaration {
    EvidenceDeclaration {
        id: DeclarationId::new(id).unwrap(),
        input: SourceName::new("skill").unwrap(),
        requirement: EvidenceRequirement::Required,
        select,
        max_items: None,
        max_bytes: None,
    }
}

#[tokio::test]
async fn markdown_occurrence_item_limits_count_each_call_without_extra_acquisitions() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("skill.md"),
        "- Use one; Use two; Use three.\n",
    )
    .unwrap();
    for limit in [2, 3] {
        let mut request = input(root.path());
        request.declarations[0].select = serde_json::from_value(serde_json::json!({"kind":"markdown_units","mode":"call_site","units":["list_item"],"pattern":"Use"})).unwrap();
        request.declarations[0].max_items = Some(positive(limit));
        let outcome = prepare_evidence(
            request,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        if limit == 2 {
            let EvidencePlanOutcome::Incomplete {
                measurements,
                diagnostics,
                ..
            } = outcome
            else {
                panic!("{outcome:?}")
            };
            assert_eq!(measurements.files_read, 1);
            assert_eq!(diagnostics.items[0].code, EvidenceCode::SelectorLimit);
        } else {
            let EvidencePlanOutcome::Prepared {
                selections,
                measurements,
                ..
            } = outcome
            else {
                panic!("{outcome:?}")
            };
            assert_eq!(measurements.files_read, 1);
            assert!(
                matches!(&selections[0].readiness, SelectionReadiness::Ready { content: SelectionContent::MarkdownUnits { units }, .. } if units.len()==3)
            );
        }
    }
}
#[tokio::test]
async fn native_plan_prepares_real_sources_once_and_serializes_requested_content() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("skill.md"), "U001: selected SENTINEL\n").unwrap();
    let mut request = input(root.path());
    request
        .declarations
        .push(declaration("all", EvidenceSelection::WholeDocument {}));
    let outcome = prepare_evidence(
        request,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared {
        selections,
        measurements,
        ..
    } = &outcome
    else {
        panic!("{outcome:?}")
    };
    assert_eq!(selections.len(), 2);
    assert_eq!(measurements.files_read, 1);
    let serialized = serde_json::to_string(&outcome).unwrap();
    assert!(serialized.contains("SENTINEL"));
    assert!(
        matches!(&selections[0].readiness,SelectionReadiness::Ready{content:SelectionContent::Paragraph{text,..},..} if text=="U001: selected SENTINEL")
    );
}
#[tokio::test]
async fn admission_refuses_bad_selectors_before_read_and_required_limits_stay_incomplete() {
    let root = tempfile::tempdir().unwrap();
    let mut request = input(root.path());
    request.declarations[0].select = EvidenceSelection::Conversation {
        turns: TurnWindow::All {},
        user_messages: Inclusion::Excluded,
        assistant_messages: AssistantSelection::Excluded,
        tool_use: ToolUseSelection::Excluded {},
        hook_injected: Inclusion::Excluded,
    };
    let outcome = prepare_evidence(
        request,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(matches!(outcome, EvidencePlanOutcome::Refused { .. }));
    std::fs::write(root.path().join("skill.md"), "U001: required\n").unwrap();
    let mut request = input(root.path());
    request.declarations[0].max_bytes = Some(positive(1));
    let outcome = prepare_evidence(
        request,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(outcome,EvidencePlanOutcome::Incomplete{diagnostics,..} if diagnostics.items[0].code==EvidenceCode::SelectorLimit)
    );
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        prepare_evidence(
            input(root.path()),
            AdapterRegistry::builtins().unwrap(),
            token
        )
        .await,
        EvidencePlanOutcome::Cancelled {
            progress: EvidenceCancellationProgress::BeforePlanAdmission,
            ..
        }
    ));
}
#[tokio::test]
async fn actual_literal_regex_native_queries_and_path_scope_are_enforced() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("skill.md"), "U001: first\n").unwrap();
    let mut request = input(root.path());
    request.native_queries = vec![NativeQuery {
        id: DeclarationId::new("matches").unwrap(),
        selector: EvidenceSelector::RegexText {
            repository_id: RepositoryId::new("repo").unwrap(),
            paths: vec![QueryPath::Exact {
                path: "skill.md".into(),
            }],
            pattern: "(?P<word>first)".into(),
        },
    }];
    let outcome = prepare_evidence(
        request,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared { selections, .. } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(
        matches!(&selections[1].readiness,SelectionReadiness::Ready{content:SelectionContent::Matches{matches},..} if matches.len()==1&&matches[0].text=="first"&&matches[0].captures.contains_key("word"))
    );
    let mut request = input(root.path());
    request.sources.insert(
        SourceName::new("skill").unwrap(),
        SourceSpec::Document {
            repository: RepositoryId::new("repo").unwrap(),
            path: "../outside".into(),
        },
    );
    assert!(
        matches!(prepare_evidence(request,AdapterRegistry::builtins().unwrap(),CancellationToken::new()).await,EvidencePlanOutcome::Refused{diagnostics} if diagnostics.items[0].code==EvidenceCode::PathOutsideRoot)
    );
}
#[tokio::test]
async fn optional_unavailable_is_prepared_but_required_unavailable_is_incomplete() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("skill.md"), "U001: available\n").unwrap();
    let mut request = input(root.path());
    request.sources.insert(
        SourceName::new("optional").unwrap(),
        SourceSpec::Document {
            repository: RepositoryId::new("repo").unwrap(),
            path: "unreadable.md".into(),
        },
    );
    let mut optional = declaration("optional_context", EvidenceSelection::WholeDocument {});
    optional.input = SourceName::new("optional").unwrap();
    optional.requirement = EvidenceRequirement::Optional;
    request.declarations.push(optional);
    let outcome = prepare_evidence(
        request.clone(),
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    let EvidencePlanOutcome::Prepared { selections, .. } = outcome else {
        panic!("Optional unavailable must not make plan incomplete: {outcome:?}")
    };
    assert!(
        matches!(&selections[1].readiness,SelectionReadiness::Refused{diagnostics} if diagnostics.items[0].code==EvidenceCode::SourceUnavailable)
    );
    request.declarations[1].requirement = EvidenceRequirement::Required;
    assert!(matches!(
        prepare_evidence(
            request,
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new()
        )
        .await,
        EvidencePlanOutcome::Incomplete { .. }
    ));
}
#[tokio::test]
async fn normalized_document_aliases_and_native_queries_share_frozen_capture() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("skill.md"), "U001: shared\n").unwrap();
    let mut request = input(root.path());
    request.limits.max_files = positive(1);
    request.sources.insert(
        SourceName::new("alias").unwrap(),
        SourceSpec::Document {
            repository: RepositoryId::new("repo").unwrap(),
            path: "./skill.md".into(),
        },
    );
    let mut alias = declaration("alias", EvidenceSelection::WholeDocument {});
    alias.input = SourceName::new("alias").unwrap();
    request.declarations.push(alias);
    request.native_queries.push(NativeQuery {
        id: DeclarationId::new("native").unwrap(),
        selector: EvidenceSelector::FileText {
            repository_id: RepositoryId::new("repo").unwrap(),
            source_path: "skill.md".into(),
        },
    });
    let outcome = prepare_evidence(
        request,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(outcome,EvidencePlanOutcome::Prepared{measurements,selections,..} if measurements.files_read==1&&selections.len()==3)
    );
}
#[tokio::test]
async fn conversation_aliases_share_capture_and_cursor_through_public_plan() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("conversation.jsonl");
    let bytes = include_bytes!("fixtures/test_parse_user_message_with_mixed_content.jsonl");
    std::fs::write(&path, bytes).unwrap();
    let mut request = input(root.path());
    request.sources.clear();
    request.declarations.clear();
    for (name, path) in [
        ("first", path),
        ("second", root.path().join(".").join("conversation.jsonl")),
    ] {
        request.sources.insert(
            SourceName::new(name).unwrap(),
            SourceSpec::Conversation {
                transcript_path: path,
                format: TranscriptFormatName::new("claude_code_jsonl").unwrap(),
                final_assistant_text: None,
            },
        );
        let mut selected = declaration(
            name,
            EvidenceSelection::Conversation {
                turns: TurnWindow::All {},
                user_messages: Inclusion::Included,
                assistant_messages: AssistantSelection::All,
                tool_use: ToolUseSelection::Included {
                    tools: ToolFilter::All {},
                    results: Inclusion::Included,
                },
                hook_injected: Inclusion::Excluded,
            },
        );
        selected.input = SourceName::new(name).unwrap();
        request.declarations.push(selected);
    }
    let outcome = prepare_evidence(
        request,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await;
    assert!(
        matches!(outcome,EvidencePlanOutcome::Prepared{captures,cursors,measurements,selections,change_selections} if change_selections.is_empty()&&captures.len()==1&&cursors.len()==1&&measurements.transcript_bytes_read==bytes.len()as u64&&selections.len()==2)
    );
}

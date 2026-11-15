use super::*;

#[tokio::test]
async fn clean_model_selected_evidence_without_asking_rules_finishes_before_evidence() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["evidence"]["kinds"]["skill"]["select"] = json!({
        "question":{"type":"noul","instructions":{"task":"Keep this skill?"}},
        "yes_at_or_above":0.7,"no_at_or_below":0.3
    });
    fixture::config_file(root.path(), &config);
    fixture::commit(root.path(), "model-selected evidence with mechanical rule");
    for scope in [
        ScopeChoice::Default,
        ScopeChoice::Since { rev: "HEAD".into() },
    ] {
        let mut request = fixture::request(root.path());
        request.scope = scope;
        let (reader, reads) = source(root.path());

        let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;

        assert_eq!(report.status, LintStatus::NothingSelected, "{report:?}");
        assert!(report.diagnostics.is_empty());
        assert_no_evidence(&report, &reads);
    }
}

fn assert_no_evidence(report: &LintReport, reads: &Arc<Mutex<Vec<PathBuf>>>) {
    assert!(reads.lock().unwrap().is_empty());
    assert_eq!(report.usage.checked_files_acquired, 0);
    assert_eq!(report.usage.model_requests, 0);
    assert!(report.results.is_empty());
    assert!(report.evidence_listing.is_none());
}

#[tokio::test]
async fn clean_default_and_since_scopes_finish_keyless_before_evidence() {
    let root = tempfile::tempdir().unwrap();
    fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    fixture::commit(root.path(), "clean asking rule");
    for scope in [
        ScopeChoice::Default,
        ScopeChoice::Since { rev: "HEAD".into() },
    ] {
        let mut request = fixture::request(root.path());
        request.scope = scope;
        let (reader, reads) = source(root.path());

        let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;

        assert_eq!(report.status, LintStatus::NothingSelected, "{report:?}");
        assert_eq!(report.status.exit_code(), 0);
        assert!(report.diagnostics.is_empty());
        assert_no_evidence(&report, &reads);
    }
}

#[tokio::test]
async fn excluded_unnamed_targets_finish_keyless_even_outside_git() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    config["check"]["ignore"] = json!(["docs/**"]);
    fixture::config_file(root.path(), &config);
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    let (reader, reads) = source(root.path());

    let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;

    assert_eq!(report.status, LintStatus::NothingSelected, "{report:?}");
    assert_eq!(report.exclusions.ignored, 1);
    assert_no_evidence(&report, &reads);
}

#[tokio::test]
async fn named_empty_asking_targets_keep_the_keyless_incomplete_contract() {
    let root = tempfile::tempdir().unwrap();
    fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    std::fs::write(root.path().join("unmatched.txt"), "not a configured target").unwrap();
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    request.paths = vec![root.path().join("unmatched.txt")];
    let (reader, reads) = source(root.path());

    let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;

    assert_eq!(report.status, LintStatus::Incomplete, "{report:?}");
    assert_eq!(report.status.exit_code(), 3);
    assert!(matches!(report.scope.kind, ScopeChoice::All));
    assert!(report.diagnostics[0].message.contains("nothing selected"));
    assert_no_evidence(&report, &reads);
}

#[tokio::test]
async fn named_keyless_asking_file_acquires_unfrozen_evidence_and_keeps_its_unavailable_result() {
    let root = tempfile::tempdir().unwrap();
    fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    request.paths = vec![root.path().join("docs/one.md")];
    let (reader, reads) = source(root.path());

    let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;

    assert_eq!(report.status, LintStatus::Incomplete, "{report:?}");
    assert_eq!(report.status.exit_code(), 3);
    assert!(report.diagnostics.is_empty());
    assert!(matches!(report.scope.kind, ScopeChoice::All));
    assert_eq!(*reads.lock().unwrap(), vec![PathBuf::from("docs/one.md")]);
    assert_eq!(report.usage.checked_files_acquired, 1);
    assert_eq!(report.usage.model_requests, 0);
    assert!(report.requests.is_empty());
    assert_eq!(report.results.len(), 1);
    let result = &report.results[0];
    assert_eq!(result.name.as_str(), "description");
    assert_eq!(result.file, Path::new("docs/one.md"));
    assert_eq!((result.first_line, result.last_line), (1, 3));
    assert!(result.text_sha256.is_some());
    assert!(matches!(
        result.ending,
        RulePieceEnding::Unavailable {
            reason: sunstack_rules::WorkUnavailableReason::CredentialUnavailable {}
        }
    ));
    assert_eq!(result.step.as_ref().unwrap().as_str(), "complies");
    assert!(report.piece_evidence.contains_key(&result.id));
    assert!(result.request_ids.is_empty());
    assert_eq!(report.unavailable_counts().rule_pieces, 1);
}

#[tokio::test]
async fn named_path_scope_is_all_even_when_config_loading_fails() {
    let root = tempfile::tempdir().unwrap();
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Since { rev: "HEAD".into() };
    request.paths = vec![root.path().join("docs/one.md")];

    let report = run(request, CancellationToken::new(), None).await;

    assert_eq!(report.status, LintStatus::Invalid);
    assert_eq!(report.diagnostics[0].code, LintInvalidCode::NoConfigFound);
    assert!(matches!(report.scope.kind, ScopeChoice::All));
}

#[tokio::test]
async fn keyless_change_scope_acquires_frozen_and_fallback_content_but_only_selected_rules_are_unavailable()
 {
    for changed in ["docs/one.md", "target.md", "unrelated.txt", "rules.json"] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
        config["evidence"]["kinds"]["skill"]["fields"]["target"] =
            json!({"from":"target","regex":"`([^`]+)`","resolve":"path","opening_chars":100});
        fixture::config_file(root.path(), &config);
        std::fs::write(
            root.path().join("docs/one.md"),
            "---\ndescription: Wrong `../target.md`\n---\n",
        )
        .unwrap();
        std::fs::write(root.path().join("target.md"), "Reference\n").unwrap();
        // The unchanged checked peer makes fallback acquisition observable even
        // when the changed file is served from the frozen Git after view.
        std::fs::write(
            root.path().join("docs/peer.md"),
            "---\ndescription: Wrong peer\n---\n",
        )
        .unwrap();
        fixture::commit(root.path(), "asking rule with an outside reference");
        if changed == "target.md" {
            let mut listing_request = fixture::request(root.path());
            listing_request.evidence_listing = Some(None);
            let listing_report = run(listing_request, CancellationToken::new(), None).await;
            assert_eq!(
                listing_report.status,
                LintStatus::Clean,
                "{listing_report:?}"
            );
            let listing = listing_report.evidence_listing.unwrap();
            assert!(listing.pieces.iter().any(|piece| {
                piece.location.file == Path::new("docs/one.md")
                    && piece.fields.values().any(|field| matches!(field,
                        sunstack_evidence::FieldValue::Targets { values }
                        if values.iter().any(|target| target.path == Path::new("target.md") && target.exists)
                    ))
            }), "outside reference must resolve through the real evidence reader");
        }
        let path = root.path().join(changed);
        let previous = std::fs::read_to_string(&path).unwrap_or_default();
        std::fs::write(&path, format!("{previous}\n")).unwrap();
        let mut request = fixture::request(root.path());
        request.scope = ScopeChoice::Default;
        let (reader, reads) = source(root.path());

        let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;

        let selected_files = match changed {
            "target.md" => vec![PathBuf::from("docs/one.md")],
            "rules.json" => vec![PathBuf::from("docs/one.md"), PathBuf::from("docs/peer.md")],
            // The appended newline is outside the frontmatter piece; unrelated files select no rule.
            _ => vec![],
        };
        assert_eq!(
            report.status,
            if selected_files.is_empty() {
                LintStatus::NothingSelected
            } else {
                LintStatus::Incomplete
            },
            "{changed}: {report:?}"
        );
        assert_eq!(
            report.status.exit_code(),
            if selected_files.is_empty() { 0 } else { 3 }
        );
        assert!(report.diagnostics.is_empty());
        assert_eq!(
            report
                .results
                .iter()
                .map(|result| result.file.clone())
                .collect::<Vec<_>>(),
            selected_files
        );
        for result in &report.results {
            assert_eq!(result.name.as_str(), "description");
            assert!(matches!(
                result.ending,
                RulePieceEnding::Unavailable {
                    reason: sunstack_rules::WorkUnavailableReason::CredentialUnavailable {}
                }
            ));
            assert_eq!(result.step.as_ref().unwrap().as_str(), "complies");
            assert!(result.text_sha256.is_some());
            assert!(report.piece_evidence.contains_key(&result.id));
            assert!(result.request_ids.is_empty());
        }
        assert!(report.findings.is_empty());
        assert_eq!(
            report.unavailable_counts().rule_pieces,
            selected_files.len()
        );
        assert_eq!(
            report.exclusions.outside_change_scope,
            u32::try_from(2 - selected_files.len()).unwrap()
        );
        assert_eq!(report.file_counts.len(), 2);
        assert_eq!(report.usage.checked_files_acquired, 3);
        assert_eq!(report.usage.model_requests, 0);
        assert!(report.requests.is_empty());
        let mut expected_reads = vec![
            PathBuf::from("docs/one.md"),
            PathBuf::from("docs/peer.md"),
            PathBuf::from("target.md"),
        ];
        if changed == "docs/one.md" || changed == "target.md" {
            expected_reads.retain(|path| path != Path::new(changed));
        }
        expected_reads.sort();
        let mut actual_reads = reads.lock().unwrap().clone();
        actual_reads.sort();
        assert_eq!(
            actual_reads, expected_reads,
            "changed after text stays frozen; each uncaptured path uses fallback once"
        );
        if changed == "rules.json" {
            assert!(matches!(report.scope.kind, ScopeChoice::All));
            assert!(matches!(
                report.scope.widened,
                Some(WidenedReason::ConfigChanged)
            ));
        } else {
            assert!(matches!(report.scope.kind, ScopeChoice::Default));
            assert!(report.scope.widened.is_none());
        }
    }
}

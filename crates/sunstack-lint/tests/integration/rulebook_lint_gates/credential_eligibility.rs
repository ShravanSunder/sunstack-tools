use super::*;

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
async fn named_asking_file_refuses_before_the_unfrozen_checked_source() {
    let root = tempfile::tempdir().unwrap();
    fixture::judged(root.path(), "http://127.0.0.1:1/v1/systemone");
    let mut request = fixture::request(root.path());
    request.scope = ScopeChoice::Default;
    request.paths = vec![root.path().join("docs/one.md")];
    let (reader, reads) = source(root.path());

    let report = run_with_source(request, CancellationToken::new(), None, Some(reader)).await;

    assert_eq!(report.status, LintStatus::Invalid);
    assert_eq!(
        report.diagnostics[0].code,
        LintInvalidCode::CredentialMissing
    );
    assert!(matches!(report.scope.kind, ScopeChoice::All));
    assert_no_evidence(&report, &reads);
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
async fn changed_paths_and_rulebook_widening_keep_the_credential_refusal_before_evidence() {
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

        assert_eq!(report.status, LintStatus::Invalid, "{changed}: {report:?}");
        assert_eq!(report.status.exit_code(), 2);
        assert_eq!(
            report.diagnostics[0].code,
            LintInvalidCode::CredentialMissing
        );
        if changed == "rules.json" {
            assert!(matches!(report.scope.kind, ScopeChoice::All));
            assert!(matches!(
                report.scope.widened,
                Some(WidenedReason::ConfigChanged)
            ));
        }
        assert_no_evidence(&report, &reads);
    }
}

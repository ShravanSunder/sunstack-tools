//! Conclusive source identity survives unrelated coverage gaps; unobserved owners do not.
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_lint::test_support::lint_fixture as fixture;
use sunstack_lint::*;
use sunstack_rules::test_support::controlled_http::ControlledServer;

const HEALTHY_SOURCE: &str = "---\ndescription: Wrong opening\n---\n\n# One\n";

fn rule_binding(id: &str, rule: &str, file: &str, lines: [usize; 2], digest: &str) -> CaseBinding {
    serde_json::from_value(json!({
        "id":id,"kind":"rule","rule":rule,"file":file,
        "lines":lines,"text_sha256":digest
    }))
    .unwrap()
}

fn negative_bindings() -> Vec<CaseBinding> {
    vec![
        rule_binding(
            "missing-owner",
            "description",
            "docs/one.md",
            [5, 5],
            &sunstack_evidence::content_digest(b"# One\n"),
        ),
        rule_binding(
            "changed-hash",
            "description",
            "docs/one.md",
            [5, 5],
            &sunstack_evidence::content_digest(b"# Changed\n"),
        ),
        rule_binding(
            "invalid-range",
            "description",
            "docs/one.md",
            [99, 99],
            &sunstack_evidence::content_digest(b"not in captured source\n"),
        ),
    ]
}

fn add_unrelated_skipped_rule(root: &Path) {
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.join("rules.json")).unwrap()).unwrap();
    entries["K002"] = entries["K001"].clone();
    entries["K002"]["name"] = json!("bad");
    entries["K002"]["evidence"] = json!("undefined");
    fixture::entry_file(root, &entries);
    std::fs::write(
        root.join("rulebook.md"),
        "Description must begin with Use when. <!-- K001 -->\n\nBad standard. <!-- K002 -->\n",
    )
    .unwrap();
}

async fn observed_run(
    server: &mut ControlledServer,
    request: LintRequest,
) -> (LintReport, Vec<Vec<u8>>) {
    let mut operation = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC_BINDING_FAILURES".into()),
    ));
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                report = &mut operation => break (report.unwrap(), captured),
                request = server.receive() => {
                    captured.push(request.raw_body.clone());
                    request.noul(0.9);
                }
            }
        }
    })
    .await
    .unwrap()
}

fn binding_status<'a>(report: &'a LintReport, id: &str) -> &'a CaseBindingStatus {
    let matching = report
        .case_selection
        .as_ref()
        .unwrap()
        .bindings
        .iter()
        .filter(|entry| entry.binding.id == id)
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "selected id must remain exactly once: {id}"
    );
    &matching[0].outcome
}

fn require_conclusive_negatives(report: &LintReport) {
    assert!(
        matches!(
            binding_status(report, "missing-owner"),
            CaseBindingStatus::Unmatched
        ),
        "{report:?}"
    );
    assert!(
        matches!(
            binding_status(report, "changed-hash"),
            CaseBindingStatus::Stale
        ),
        "{report:?}"
    );
    assert!(
        matches!(
            binding_status(report, "invalid-range"),
            CaseBindingStatus::Stale
        ),
        "{report:?}"
    );
    assert!(
        report.results.is_empty(),
        "negative identities must not fabricate an execution owner"
    );
    assert!(
        report.requests.is_empty(),
        "negative identities must not reserve model requests"
    );
}

#[tokio::test]
async fn unrelated_skipped_rule_preserves_conclusive_missing_and_stale_bindings_without_phantom_dispatch()
 {
    // Arrange: a real admitted asking rule has a frontmatter owner at lines 1..3.
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    fixture::judged(root.path(), &server.endpoint);
    assert_eq!(
        std::fs::read_to_string(root.path().join("docs/one.md")).unwrap(),
        HEALTHY_SOURCE
    );

    // Act / Assert: ordinary HTTP is a positive observer control before each negative run.
    let (ordinary, ordinary_http) = observed_run(&mut server, fixture::request(root.path())).await;
    assert_eq!(ordinary.status, LintStatus::Clean, "{ordinary:?}");
    assert_eq!(ordinary_http.len(), 1);
    assert!(!ordinary_http[0].is_empty());
    assert_eq!(ordinary.results.len(), 1);
    assert_eq!(
        (
            ordinary.results[0].first_line,
            ordinary.results[0].last_line
        ),
        (1, 3)
    );
    let mut request = fixture::request(root.path());
    request.case_selection = Some(negative_bindings());
    let (baseline, baseline_http) = observed_run(&mut server, request).await;
    assert!(baseline_http.is_empty());
    assert_eq!(baseline.case_selection.as_ref().unwrap().bindings.len(), 3);
    require_conclusive_negatives(&baseline);
    assert_eq!(baseline.usage.checked_files_acquired, 1);
    assert_eq!(baseline.file_counts.len(), 1);
    assert_eq!(baseline.file_counts[0].file, Path::new("docs/one.md"));

    // A separate unadmitted rule makes overall coverage partial, not the healthy file unknowable.
    add_unrelated_skipped_rule(root.path());
    let (ordinary_partial, partial_http) =
        observed_run(&mut server, fixture::request(root.path())).await;
    assert_eq!(
        ordinary_partial.status,
        LintStatus::Incomplete,
        "{ordinary_partial:?}"
    );
    assert_eq!(
        partial_http.len(),
        1,
        "the admitted peer still really asks the model"
    );
    assert_eq!(
        partial_http[0], ordinary_http[0],
        "unrelated skipped rule preserves the actual peer request"
    );
    assert_eq!(ordinary_partial.skipped_rules.len(), 1);
    assert_eq!(ordinary_partial.skipped_rules[0].key, "K002");
    let mut bindings = negative_bindings();
    bindings.push(rule_binding(
        "unadmitted-rule",
        "bad",
        "docs/one.md",
        [5, 5],
        &sunstack_evidence::content_digest(b"# One\n"),
    ));
    let mut request = fixture::request(root.path());
    request.case_selection = Some(bindings);
    let (partial, selected_http) = observed_run(&mut server, request).await;
    assert!(
        selected_http.is_empty(),
        "no negative binding is a dispatchable owner"
    );
    assert_eq!(partial.status, LintStatus::Incomplete, "{partial:?}");
    assert_eq!(partial.case_selection.as_ref().unwrap().bindings.len(), 4);
    require_conclusive_negatives(&partial);
    assert!(
        matches!(
            binding_status(&partial, "unadmitted-rule"),
            CaseBindingStatus::Incomplete {
                reason: IncompleteReason::CoveragePartial
            }
        ),
        "{partial:?}"
    );
    assert!(partial.incomplete.iter().any(
        |part| part.file.is_none() && matches!(part.reason, IncompleteReason::CoveragePartial)
    ));
    assert_eq!(partial.skipped_rules.len(), 1);
    assert_eq!(partial.usage.checked_files_acquired, 1);
    assert_eq!(partial.file_counts.len(), 1);
    assert_eq!(partial.file_counts[0].file, Path::new("docs/one.md"));
    assert_eq!(
        server.finish().await,
        2,
        "only the ordinary positive controls contacted HTTP"
    );
}

#[tokio::test]
async fn file_limit_distinguishes_captured_missing_owner_from_omitted_file_without_dispatch() {
    // Arrange: the established sorted checked-file selection retains one.md before two.md.
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut config = fixture::judged(root.path(), &server.endpoint);
    std::fs::write(
        root.path().join("docs/two.md"),
        "---\ndescription: Other opening\n---\n\n# Two\n",
    )
    .unwrap();
    config["limits"] = json!({"max_files":1});
    fixture::config_file(root.path(), &config);
    let mut request = fixture::request(root.path());
    request.case_selection = Some(vec![
        rule_binding(
            "captured-missing",
            "description",
            "docs/one.md",
            [5, 5],
            &sunstack_evidence::content_digest(b"# One\n"),
        ),
        rule_binding(
            "omitted-file",
            "description",
            "docs/two.md",
            [5, 5],
            &sunstack_evidence::content_digest(b"# Two\n"),
        ),
    ]);

    // Act: the global selection failure belongs to the omitted file, not captured identity.
    let (report, http) = observed_run(&mut server, request).await;
    assert!(http.is_empty());
    assert_eq!(report.status, LintStatus::Incomplete, "{report:?}");
    assert_eq!(report.usage.checked_files_acquired, 1);
    assert_eq!(report.file_counts.len(), 1);
    assert_eq!(report.file_counts[0].file, Path::new("docs/one.md"));
    assert_eq!(report.case_selection.as_ref().unwrap().bindings.len(), 2);
    assert!(
        matches!(
            binding_status(&report, "captured-missing"),
            CaseBindingStatus::Unmatched
        ),
        "{report:?}"
    );
    assert!(
        matches!(
            binding_status(&report, "omitted-file"),
            CaseBindingStatus::Incomplete {
                reason: IncompleteReason::SelectionFailed
            }
        ),
        "{report:?}"
    );
    assert!(report.incomplete.iter().any(
        |part| part.file.is_none() && matches!(part.reason, IncompleteReason::SelectionFailed)
    ));
    assert!(report.results.is_empty());
    assert!(report.requests.is_empty());
    assert_eq!(server.finish().await, 0);
}

#[tokio::test]
async fn match_limit_keeps_captured_unknown_owner_incomplete_but_conclusive_changed_hash_stale() {
    // Arrange: a single matcher sees two paragraphs, so max_matches=1 truncates this file.
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut config = fixture::judged(root.path(), &server.endpoint);
    config["limits"] = json!({"max_matches":1});
    config["evidence"]["kinds"]["skill"] = json!({
        "rule":"skill","fields":{"description":{"from":"first_line"}}
    });
    fixture::config_file(root.path(), &config);
    std::fs::write(
        root.path().join("evidence/rules/skill.yml"),
        "id: skill\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("docs/one.md"),
        "First café.\n\nSecond β.\n",
    )
    .unwrap();
    let mut request = fixture::request(root.path());
    request.case_selection = Some(vec![
        rule_binding(
            "limited-owner",
            "description",
            "docs/one.md",
            [3, 3],
            &sunstack_evidence::content_digest("Second β.\n".as_bytes()),
        ),
        rule_binding(
            "limited-changed-hash",
            "description",
            "docs/one.md",
            [3, 3],
            &sunstack_evidence::content_digest(b"Changed second paragraph.\n"),
        ),
    ]);

    // Act / Assert: incomplete scanning cannot prove absence, but original captured bytes prove mismatch.
    let (report, http) = observed_run(&mut server, request).await;
    assert!(http.is_empty());
    assert_eq!(report.status, LintStatus::Incomplete, "{report:?}");
    assert_eq!(report.usage.checked_files_acquired, 1);
    assert_eq!(report.file_counts.len(), 1);
    assert_eq!(report.file_counts[0].file, Path::new("docs/one.md"));
    assert!(
        report
            .diagnostics
            .iter()
            .any(
                |diagnostic| diagnostic.file.as_deref() == Some(Path::new("docs/one.md"))
                    && diagnostic.message == "max_matches"
            )
    );
    assert_eq!(report.case_selection.as_ref().unwrap().bindings.len(), 2);
    assert!(
        matches!(
            binding_status(&report, "limited-owner"),
            CaseBindingStatus::Incomplete { .. }
        ),
        "{report:?}"
    );
    assert!(
        matches!(
            binding_status(&report, "limited-changed-hash"),
            CaseBindingStatus::Stale
        ),
        "{report:?}"
    );
    assert!(report.results.is_empty());
    assert!(report.requests.is_empty());
    assert_eq!(server.finish().await, 0);
}

fn selected_paragraph_project(root: &Path, endpoint: &str, two_files: bool) -> LintRequest {
    let mut config = fixture::judged(root, endpoint);
    config["evidence"]["kinds"]["skill"] = json!({
        "markdown_units":{"mode":"unit","units":["paragraph"]},
        "fields":{"description":{"from":"first_line"}},
        "select":{"question":{"type":"noul","instructions":{
            "check":"description","task":"Keep this exact paragraph?"
        }},"yes_at_or_above":0.8,"no_at_or_below":0.2}
    });
    fixture::config_file(root, &config);
    let mut request = fixture::request(root);
    let mut bindings = Vec::new();
    for (file, text, id) in [
        ("docs/one.md", "Selectable one café.\n", "settled-no"),
        ("docs/two.md", "Selectable two β.\n", "failed-selector"),
    ]
    .into_iter()
    .take(if two_files { 2 } else { 1 })
    {
        std::fs::write(root.join(file), text).unwrap();
        bindings.push(rule_binding(
            id,
            "description",
            file,
            [1, 1],
            &sunstack_evidence::content_digest(text.as_bytes()),
        ));
    }
    request.case_selection = Some(bindings);
    request
}

async fn observe_selectors(
    server: &mut ControlledServer,
    request: LintRequest,
    probability: f64,
    fail_second: bool,
) -> (LintReport, Vec<Vec<u8>>) {
    let mut operation = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC_SELECTOR_SETTLEMENT".into()),
    ));
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                report = &mut operation => break (report.unwrap(), captured),
                request = server.receive() => {
                    assert!(request.body["state"]["candidates"].is_object(),
                        "no final rule request is valid after negative, uncertain or failed selection");
                    let second = request.body["state"]["where"]["file"] == "docs/two.md";
                    captured.push(request.raw_body.clone());
                    if fail_second && second {
                        request.respond(400, json!({"error":"controlled permanent selection refusal"}));
                    } else {
                        request.noul(probability);
                    }
                }
            }
        }
    }).await.unwrap()
}

fn selection_count<'a>(
    report: &'a LintReport,
    file: &str,
) -> &'a sunstack_evidence::SelectionCount {
    let count = report
        .file_counts
        .iter()
        .find(|count| count.file == Path::new(file))
        .unwrap();
    assert_eq!(count.kinds.len(), 1);
    count
        .kinds
        .values()
        .next()
        .unwrap()
        .selection
        .as_ref()
        .unwrap()
}

#[tokio::test]
async fn settled_negative_selector_remains_unmatched_when_another_file_selector_fails() {
    // Both bindings legitimately request selection; one settles no and the peer receives HTTP 400.
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let request = selected_paragraph_project(root.path(), &server.endpoint, true);
    let (report, http) = observe_selectors(&mut server, request, 0.05, true).await;
    assert_eq!(http.len(), 2);
    assert!(http.iter().all(|raw| !raw.is_empty()));
    assert_eq!(report.status, LintStatus::Incomplete, "{report:?}");
    assert_eq!(report.usage.checked_files_acquired, 2);
    assert_eq!(report.file_counts.len(), 2);
    assert_eq!(report.case_selection.as_ref().unwrap().bindings.len(), 2);
    assert!(
        matches!(
            binding_status(&report, "settled-no"),
            CaseBindingStatus::Unmatched
        ),
        "{report:?}"
    );
    assert!(
        matches!(
            binding_status(&report, "failed-selector"),
            CaseBindingStatus::Incomplete { .. }
        ),
        "{report:?}"
    );
    assert!(
        report.incomplete.iter().any(|part| part.file.is_none()
            && matches!(part.reason, IncompleteReason::SelectionIncomplete))
    );
    let settled = selection_count(&report, "docs/one.md");
    assert_eq!(
        (
            settled.candidates,
            settled.asked,
            settled.selected,
            settled.not_selected,
            settled.uncertain,
            settled.incomplete
        ),
        (1, 1, 0, 1, 0, 0)
    );
    let failed = selection_count(&report, "docs/two.md");
    assert_eq!(
        (
            failed.candidates,
            failed.asked,
            failed.selected,
            failed.not_selected,
            failed.uncertain,
            failed.incomplete
        ),
        (1, 1, 0, 0, 0, 1)
    );
    assert!(report.results.is_empty());
    assert_eq!(report.requests.len(), 2);
    assert!(
        report
            .requests
            .iter()
            .all(|request| matches!(request.owner, RequestOwner::Selection { .. }))
    );
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn uncertain_selector_stays_incomplete_under_unrelated_partial_rule_coverage() {
    // Uncertainty cannot prove absence; the existing coverage gap keeps conservative accounting.
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut request = selected_paragraph_project(root.path(), &server.endpoint, false);
    request.case_selection.as_mut().unwrap()[0].id = "uncertain-selector".into();
    add_unrelated_skipped_rule(root.path());
    let (report, http) = observe_selectors(&mut server, request, 0.5, false).await;
    assert_eq!(http.len(), 1);
    assert!(!http[0].is_empty());
    assert_eq!(report.status, LintStatus::Incomplete, "{report:?}");
    assert_eq!(report.skipped_rules.len(), 1);
    assert_eq!(report.skipped_rules[0].key, "K002");
    assert_eq!(report.case_selection.as_ref().unwrap().bindings.len(), 1);
    assert!(
        matches!(
            binding_status(&report, "uncertain-selector"),
            CaseBindingStatus::Incomplete {
                reason: IncompleteReason::CoveragePartial
            }
        ),
        "{report:?}"
    );
    assert!(report.incomplete.iter().any(
        |part| part.file.is_none() && matches!(part.reason, IncompleteReason::CoveragePartial)
    ));
    let uncertain = selection_count(&report, "docs/one.md");
    assert_eq!(
        (
            uncertain.candidates,
            uncertain.asked,
            uncertain.selected,
            uncertain.not_selected,
            uncertain.uncertain,
            uncertain.incomplete
        ),
        (1, 1, 0, 0, 1, 0)
    );
    assert!(report.results.is_empty());
    assert_eq!(report.requests.len(), 1);
    assert!(matches!(
        report.requests[0].owner,
        RequestOwner::Selection { .. }
    ));
    assert_eq!(server.finish().await, 1);
}

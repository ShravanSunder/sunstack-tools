#![allow(clippy::panic)]
use super::*;
use std::collections::BTreeSet;
use sunstack_rules::test_support::controlled_http::ControlledServer;
use sunstack_rules::{FilterAnswerState, SelectionOutcome, WorkUnavailableReason};

fn entries(root: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(root.join("rules.json")).unwrap()).unwrap()
}

fn assert_no_dispatch(report: &LintReport) {
    assert!(report.requests.is_empty(), "{report:?}");
    assert!(report.evidence_seen.is_empty());
    assert_eq!(report.usage.model_requests, 0);
    assert_eq!(report.usage.input_tokens, None);
    assert_eq!(report.usage.output_tokens, None);
    assert_eq!(report.usage.cost_usd, None);
    assert_eq!(report.usage.cost_reported_requests, 0);
    assert_eq!(report.usage.cost_missing_requests, 0);
}

#[tokio::test]
async fn ordinary_keyless_mixed_run_retains_the_independent_finding_and_only_reached_ask_is_unavailable()
 {
    // Arrange: establish the actual mechanical finding before adding model-capable neighbors.
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut config = fixture::write(root.path());
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    let baseline = run(
        fixture::request(root.path()),
        CancellationToken::new(),
        None,
    )
    .await;
    assert_eq!(baseline.status, LintStatus::Findings);
    assert_eq!(baseline.findings.len(), 1);
    let mut rules = entries(root.path());
    rules["K002"] = json!({
        "name":"judged","evidence":"skill","severity":"error",
        "questions":{"complies":{"type":"noul","instructions":{"check":"description","task":"Does it comply?"}}},
        "decision":{"start":"ask","steps":{"ask":{"ask":"complies","yes_at_or_above":0.8,"no_at_or_below":0.2,
            "yes":"pass","no":"not_applicable","uncertain":"uncertain"}}}
    });
    rules["K003"] = json!({
        "name":"unused_ask","evidence":"skill","severity":"error",
        "questions":{"unused":{"type":"noul","instructions":{"check":"description","task":"Unused authored question"}}},
        "decision":{"start":"local","steps":{"local":{"mechanical_check":{"tool":"contains","field":"description","text":"Wrong"},
            "yes":"pass","no":"not_applicable"}}}
    });
    fixture::entry_file(root.path(), &rules);
    std::fs::write(root.path().join("rulebook.md"),
        "Description must begin with Use when. <!-- K001 -->\n\nJudge description. <!-- K002 -->\n\nSettle locally. <!-- K003 -->\n").unwrap();

    // A nonempty keyed witness proves this exact fixture can reach the real HTTP listener.
    let mut keyed_operation = tokio::spawn(run(
        fixture::request(root.path()),
        CancellationToken::new(),
        Some("SYNTHETIC_PARTIAL".into()),
    ));
    let pending = tokio::select! {
        pending = server.next() => pending,
        report = &mut keyed_operation => panic!("keyed witness did not dispatch: {:?}", report.unwrap()),
    };
    assert!(pending.body["questions"].get("complies").is_some());
    assert!(pending.body["questions"].get("unused").is_none());
    pending.noul(0.9);
    let keyed = tokio::time::timeout(std::time::Duration::from_secs(3), keyed_operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(keyed.usage.model_requests, 1);

    // Act: remove only the credential; the ordinary entry must still acquire and evaluate evidence.
    let (reader, reads) = source(root.path());
    let partial = run_with_source(
        fixture::request(root.path()),
        CancellationToken::new(),
        None,
        Some(reader),
    )
    .await;

    // Assert: useful finding contents/identity match the independent baseline, while real gaps remain.
    assert_eq!(partial.status, LintStatus::Incomplete, "{partial:?}");
    assert_eq!(partial.status.exit_code(), 3);
    assert_eq!(
        serde_json::to_value(&partial.findings).unwrap(),
        serde_json::to_value(&baseline.findings).unwrap()
    );
    assert_eq!(*reads.lock().unwrap(), vec![PathBuf::from("docs/one.md")]);
    assert_eq!(partial.usage.checked_files_acquired, 1);
    assert!(partial.preparation.is_none());
    assert!(partial.diagnostics.is_empty());
    assert_eq!(partial.results.len(), 3);
    let judged = partial
        .results
        .iter()
        .find(|result| result.name.as_str() == "judged")
        .unwrap();
    assert!(matches!(
        judged.ending,
        RulePieceEnding::Unavailable {
            reason: WorkUnavailableReason::CredentialUnavailable {}
        }
    ));
    assert_eq!(judged.file, Path::new("docs/one.md"));
    assert_eq!((judged.first_line, judged.last_line), (1, 3));
    assert!(judged.text_sha256.is_some());
    assert!(judged.id > 0);
    assert!(partial.piece_evidence.contains_key(&judged.id));
    assert!(judged.request_ids.is_empty());
    assert_eq!(judged.step.as_ref().unwrap().as_str(), "ask");
    assert_eq!(
        judged
            .path
            .iter()
            .map(|step| step.as_str())
            .collect::<Vec<_>>(),
        ["ask"]
    );
    assert!(judged.basis.is_none());
    assert!(judged.answers.is_empty());
    let unused = partial
        .results
        .iter()
        .find(|result| result.name.as_str() == "unused_ask")
        .unwrap();
    assert!(matches!(unused.ending, RulePieceEnding::Pass));
    assert_eq!(
        partial.unavailable_counts(),
        UnavailableCounts {
            rule_pieces: 1,
            ..Default::default()
        }
    );
    assert_eq!(partial.incomplete.len(), 1);
    let saved = lint_reporting::saved_reports::show(&partial, judged.id).unwrap();
    assert_eq!(saved["result"]["id"], judged.id);
    assert_eq!(saved["result"]["reason"]["kind"], "credential_unavailable");
    assert_eq!(saved["rewalkable"], false);
    assert_no_dispatch(&partial);
    assert_eq!(
        server.finish().await,
        1,
        "only the keyed witness contacted HTTP"
    );
}

#[tokio::test]
async fn unavailable_filter_keeps_its_actual_answer_and_known_gated_piece_beside_the_independent_finding()
 {
    // Arrange: reuse the real gated-context fixture and add an ungated copy of its mechanical rule.
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    std::fs::create_dir_all(root.path().join("packages/one/references")).unwrap();
    std::fs::write(
        root.path().join("packages/one/about.md"),
        "---\ndescription: Package purpose\n---\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("packages/one/references/member.md"),
        "---\ndescription: Member text\n---\n",
    )
    .unwrap();
    config["check"]["files"] = json!(["packages/*/references/*.md"]);
    config["filters"] = json!({"purpose":{"per":"packages/{name}","reads":{"file":"about.md","kind":"skill","fields":["description"]},
        "labels":["a","b"],"questions":{"filter":{"type":"choice","instructions":{"task":"Classify","check":"description"},"criteria":{"a":"A","b":"B"}}},
        "decision":{"start":"filter","steps":{"filter":{"ask":"filter","min_probability":0.7,"routes":{"a":{"label":"a"},"b":{"label":"b"}},"uncertain":"unsure"}}}}});
    config["decision_model"]["endpoint"] = json!("http://127.0.0.1:1/v1/systemone");
    fixture::config_file(root.path(), &config);
    let mut rules = entries(root.path());
    rules["K001"]["only_when"] = json!({"purpose":["a"]});
    rules["K002"] = rules["K001"].clone();
    rules["K002"].as_object_mut().unwrap().remove("only_when");
    rules["K002"]["name"] = json!("independent");
    fixture::entry_file(root.path(), &rules);
    std::fs::write(
        root.path().join("rulebook.md"),
        "Check gated description. <!-- K001 -->\n\nCheck independent description. <!-- K002 -->\n",
    )
    .unwrap();

    // Act: ordinary no-key execution must reach both the filter and independent rule.
    let report = run(
        fixture::request(root.path()),
        CancellationToken::new(),
        None,
    )
    .await;

    // Assert: retain the actual filter folder and gated subject, without guessing a filter label.
    assert_eq!(report.status, LintStatus::Incomplete, "{report:?}");
    assert_eq!(report.status.exit_code(), 3);
    assert_eq!(report.filter_answers.len(), 1);
    let answer = &report.filter_answers[0];
    assert_eq!(answer.filter.as_str(), "purpose");
    assert_eq!(answer.folder, Path::new("packages/one"));
    assert!(answer.id > 0);
    assert!(answer.reads_sha256.is_some());
    assert!(answer.request_ids.is_empty());
    assert!(matches!(
        answer.state,
        FilterAnswerState::Unavailable {
            reason: WorkUnavailableReason::CredentialUnavailable {},
            ..
        }
    ));
    assert!(
        report.filter_skips.is_empty(),
        "known gated pieces must have unavailable results"
    );
    assert_eq!(report.results.len(), 2);
    let gated = report
        .results
        .iter()
        .find(|result| result.name.as_str() == "description")
        .unwrap();
    assert!(
        matches!(&gated.ending, RulePieceEnding::Unavailable { reason:WorkUnavailableReason::FilterUnavailable { filter,folder } }
        if filter.as_str() == "purpose" && folder == Path::new("packages/one"))
    );
    assert_eq!(gated.file, Path::new("packages/one/references/member.md"));
    assert!(report.piece_evidence.contains_key(&gated.id));
    assert!(report.piece_evidence.contains_key(&answer.id));
    assert_ne!(gated.id, answer.id);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule.as_str(), "independent");
    assert_eq!(
        report.findings[0].message,
        "Description must begin with Use when."
    );
    assert_eq!(report.counts.error, 1);
    assert_eq!(
        report.unavailable_counts(),
        UnavailableCounts {
            rule_pieces: 1,
            filter_folders: 1,
            ..Default::default()
        }
    );
    assert_no_dispatch(&report);
}

#[tokio::test]
async fn unavailable_selector_keeps_candidate_and_scope_units_without_inventing_child_case_results()
{
    // Arrange: two selected paragraphs feed one section, whose own body remains independently usable.
    const DOCUMENT: &str = "# One\n\nFirst child.\n\nSecond child.\n";
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["decision_model"]["endpoint"] = json!("http://127.0.0.1:1/v1/systemone");
    config["evidence"]["kinds"] = json!({
        "child":{"markdown_units":{"mode":"unit","units":["paragraph"]},"fields":{"body":{"from":"first_line"}},
            "select":{"question":{"type":"noul","instructions":{"check":"body","task":"Keep this child?"}},"yes_at_or_above":0.8,"no_at_or_below":0.2}},
        "document":{"markdown_units":{"mode":"unit","units":["section"]},"fields":{"body":{"from":"section","text":"full"},"inside":{"from":"inside","kind":"child"}}}
    });
    fixture::config_file(root.path(), &config);
    std::fs::write(root.path().join("docs/one.md"), DOCUMENT).unwrap();
    fixture::entry_file(
        root.path(),
        &json!({
            "K001":{"name":"child_rule","evidence":"child","severity":"error","decision":{"start":"local","steps":{"local":{"mechanical_check":{"tool":"contains","field":"body","text":"child"},"yes":"pass","no":"not_applicable"}}}},
            "K002":{"name":"independent","evidence":"document","severity":"error","decision":{"start":"local","steps":{"local":{"mechanical_check":{"tool":"contains","field":"body","text":"absent text"},"yes":"pass","no":{"finding":"Independent section finding.","at":"body"}}}}}
        }),
    );
    std::fs::write(
        root.path().join("rulebook.md"),
        "Check selected child. <!-- K001 -->\n\nCheck independent body. <!-- K002 -->\n",
    )
    .unwrap();
    let mut request = fixture::request(root.path());
    request.case_selection = Some(serde_json::from_value(json!([
        {"id":"unknown-child","kind":"rule","rule":"child_rule","file":"docs/one.md","lines":[3,3],"text_sha256":sunstack_evidence::content_digest(b"First child.\n")},
        {"id":"known-section","kind":"rule","rule":"independent","file":"docs/one.md","lines":[1,5],"text_sha256":sunstack_evidence::content_digest(DOCUMENT.as_bytes())}
    ])).unwrap());

    // Act: use actual capture, selection, evidence deferral, case binding and rule projection.
    let report = run(request, CancellationToken::new(), None).await;

    // Assert: two actual candidates produce one scope per affected kind/file, never a child result.
    assert_eq!(report.status, LintStatus::Incomplete, "{report:?}");
    assert_eq!(report.status.exit_code(), 3);
    assert_eq!(
        report.exclusions.zero_matches, 0,
        "unavailable selection is unknown coverage, not observed absence"
    );
    assert_eq!(report.selections.len(), 2);
    assert!(report.selections.iter().all(|entry| matches!(
        entry.outcome,
        SelectionOutcome::Unavailable {
            reason: WorkUnavailableReason::CredentialUnavailable {}
        }
    )));
    let scopes = report
        .incomplete
        .iter()
        .filter_map(|part| match &part.reason {
            IncompleteReason::SelectionUnavailable {
                evidence_kind,
                file,
                selection_kind,
            } => {
                assert_eq!(part.file.as_ref(), Some(file));
                assert_eq!(file, Path::new("docs/one.md"));
                assert_eq!(selection_kind.as_str(), "child");
                Some((evidence_kind.as_str(), file.as_path()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(scopes.len(), 2);
    assert_eq!(
        scopes.iter().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ("child", Path::new("docs/one.md")),
            ("document", Path::new("docs/one.md"))
        ])
    );
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].name.as_str(), "independent");
    assert!(matches!(report.results[0].ending, RulePieceEnding::Finding));
    assert_eq!(report.findings[0].message, "Independent section finding.");
    let bindings = &report.case_selection.as_ref().unwrap().bindings;
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].binding.id, "unknown-child");
    assert!(
        matches!(&bindings[0].outcome, CaseBindingStatus::Unavailable { reason:WorkUnavailableReason::SelectionUnavailable { evidence_kind,file } }
        if evidence_kind.as_str() == "child" && file == Path::new("docs/one.md"))
    );
    assert_eq!(bindings[1].binding.id, "known-section");
    assert!(matches!(
        bindings[1].outcome,
        CaseBindingStatus::Selected { .. }
    ));
    assert!(
        report
            .results
            .iter()
            .all(|result| result.name.as_str() != "child_rule")
    );
    assert_eq!(
        report.piece_evidence.len(),
        1,
        "no child evidence/result ID may be fabricated"
    );
    let selected_count = report.file_counts[0]
        .kinds
        .iter()
        .find(|(kind, _)| kind.as_str() == "child")
        .unwrap()
        .1
        .selection
        .as_ref()
        .unwrap();
    assert_eq!(
        (
            selected_count.candidates,
            selected_count.asked,
            selected_count.selected,
            selected_count.not_selected,
            selected_count.unavailable
        ),
        (2, 0, 0, 0, 2)
    );
    assert_eq!(
        report.unavailable_counts(),
        UnavailableCounts {
            selection_candidates: 2,
            evidence_scopes: 2,
            ..Default::default()
        }
    );
    assert_no_dispatch(&report);
}

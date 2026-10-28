#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use support::*;

// Literal oracle: TP=1, FP=1, TN=1, clear FN=2 (one filter miss),
// inconclusive yes/no=3; unknown truth never becomes a negative.
#[test]
fn hand_worked_table_scores_every_report_field() {
    let root = tempfile::tempdir().unwrap();
    let labels = [
        Label::Yes,
        Label::Yes,
        Label::No,
        Label::No,
        Label::Yes,
        Label::Unknown,
        Label::Yes,
        Label::No,
        Label::Yes,
    ];
    let set = freeze(
        root.path(),
        labels
            .into_iter()
            .enumerate()
            .map(|(index, label)| case(index + 1, label, Split::HeldOut))
            .collect(),
    );
    let entries = vec![
        entry(
            1,
            json!({"kind":"finding","path":[]}),
            Some("passed"),
            Some(10),
        ),
        entry(
            2,
            json!({"kind":"dropped","at":{"kind":"verdict"},"answer":"answer_1"}),
            Some("did_not_pass"),
            Some(30),
        ),
        entry(
            3,
            json!({"kind":"finding","path":[]}),
            Some("passed"),
            Some(80),
        ),
        entry(
            4,
            json!({"kind":"dropped","at":{"kind":"verdict"},"answer":"answer_1"}),
            Some("did_not_pass"),
            Some(20),
        ),
        entry(
            5,
            json!({"kind":"uncertain","at":{"kind":"verdict"},"answers":[]}),
            Some("uncertain"),
            Some(50),
        ),
        entry(
            6,
            json!({"kind":"dropped","at":{"kind":"verdict"},"answer":"answer_1"}),
            Some("did_not_pass"),
            None,
        ),
        entry(
            8,
            json!({"kind":"unresolved","at":{"kind":"verdict"},"missing":{"kind":"page_missing","pages":[1]}}),
            None,
            None,
        ),
        entry(
            9,
            json!({"kind":"not_processed","reason":"evaluation_failed"}),
            None,
            None,
        ),
    ];
    let mut observed = bundle(entries, Some(0.25), 1000);
    observed.inputs.push(located_input(&set));
    observed
        .dropped_samples
        .push(sunstack_rules::DroppedSample {
            rule: rule(),
            pass: sunstack_rules::SamplePass::Mechanical,
            occurrence: occurrence(7),
            excerpt: "actual mechanical drop".into(),
        });
    let report = score(&[observed], &set).unwrap();
    let actual = serde_json::to_value(report).unwrap();
    assert_eq!(actual["rule"], json!(rule()));
    assert_eq!(
        actual["rule_version"],
        json!("a05d97f6d12f14582ce56010cd430ba0b31cd2ffabdf8f42306bad6edf6c6969")
    );
    let manifest_bytes = std::fs::read(root.path().join("cases/manifest.json")).unwrap();
    assert_eq!(
        actual["case_set"],
        json!(sunstack_evidence::content_digest(&manifest_bytes))
    );
    assert_eq!(actual["split"], json!("held_out"));
    assert_eq!(
        actual["filter"],
        json!({"positive_cases":5,"kept":4,"missed":["case_7"],"unknown":[]})
    );
    assert_eq!(
        actual["steps"],
        json!([{ "question":{"kind":"verdict"},"correct":["case_1","case_4"],
        "wrong":["case_2","case_3"],"unknown":["case_5","case_7","case_8","case_9"] }])
    );
    assert_eq!(
        actual["end_result"],
        json!({"true_findings":["case_1"],"false_findings":["case_3"],
        "correct_negatives":["case_4"],"missed_findings":["case_2","case_7"],
        "inconclusive":["case_5","case_8","case_9"]})
    );
    assert_eq!(actual["unknown_labels"], json!(1));
    assert_eq!(
        actual["cost"],
        json!({"requests":4,"attempts":5,"input_tokens":100,"output_tokens":20,"cost_usd":0.25,"wall_time_ms":1000})
    );
    assert_eq!(
        actual["speed"],
        json!({"run_wall_time_ms":1000,"p50_ms":30,"p95_ms":80,"measured_candidates":5,"missing_candidates":3,"percentile":"nearest_rank"})
    );
}

#[test]
fn ambiguous_answers_and_missing_measurements_stay_unknown() {
    let root = tempfile::tempdir().unwrap();
    let set = freeze(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let mut candidate = entry(1, json!({"kind":"finding","path":[]}), Some("passed"), None);
    let mut conflicting = candidate.answers[0].clone();
    conflicting.settlement = sunstack_rules::QuestionSettlement::DidNotPass;
    candidate.answers.push(conflicting);
    let report = score(
        &[
            bundle(vec![candidate], None, 70),
            bundle(vec![], Some(0.5), 30),
        ],
        &set,
    )
    .unwrap();
    assert_eq!(report.steps.len(), 1);
    assert_eq!(
        serde_json::to_value(&report.steps[0]).unwrap()["unknown"],
        json!(["case_1"])
    );
    assert_eq!(report.speed.p50_ms, None);
    assert_eq!(report.speed.p95_ms, None);
    assert_eq!(report.speed.missing_candidates, 1);
    assert_eq!(report.speed.run_wall_time_ms, 100);
    assert_eq!(report.cost.cost_usd, None);
}

#[test]
fn step_only_labels_are_not_end_truth_and_filter_occurrences_count_once() {
    let root = tempfile::tempdir().unwrap();
    let mut gate = case(1, Label::Yes, Split::HeldOut);
    gate.case_id = CaseId::new("gate").unwrap();
    gate.question = sunstack_rules::QuestionRef::RuleGate { position: 0 };
    let mut second_gate = gate.clone();
    second_gate.case_id = CaseId::new("second_gate").unwrap();
    second_gate.question = sunstack_rules::QuestionRef::RuleSetGate { position: 0 };
    let mut development = case(2, Label::No, Split::Development);
    development.session.session = sunstack_evidence::SessionId::new("dev").unwrap();
    development.project = Some(ProjectKey::new("dev").unwrap());
    development.source = CaseSourceId::new("dev_source").unwrap();
    let mut specification = input(root.path(), vec![gate, second_gate, development]);
    let mut dev_source = specification.sources[0].clone();
    dev_source.source_id = CaseSourceId::new("dev_source").unwrap();
    dev_source.source =
        serde_json::from_value(json!({"kind":"session_pair","provider":"codex","session":"dev"}))
            .unwrap();
    specification.sources.push(dev_source);
    let set = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    let mut candidate = entry(
        1,
        json!({"kind":"finding","path":[]}),
        Some("passed"),
        Some(123),
    );
    candidate.answers[0].question = sunstack_rules::QuestionRef::RuleGate { position: 0 };
    let report = score(&[bundle(vec![candidate], Some(0.1), 500)], &set).unwrap();
    assert_eq!(report.filter.positive_cases, 1);
    assert_eq!(report.filter.kept, 1);
    assert_eq!(report.steps.len(), 2);
    assert_eq!(report.steps[0].correct, vec![CaseId::new("gate").unwrap()]);
    assert_eq!(
        report.steps[1].unknown,
        vec![CaseId::new("second_gate").unwrap()]
    );
    assert_eq!(
        serde_json::to_value(report.end_result).unwrap(),
        json!({"true_findings":[],"false_findings":[],
        "correct_negatives":[],"missed_findings":[],"inconclusive":[]})
    );
}

#[test]
fn absent_or_duplicate_candidates_are_inconclusive_not_negative() {
    let root = tempfile::tempdir().unwrap();
    let set = freeze(
        root.path(),
        vec![
            case(1, Label::Yes, Split::HeldOut),
            case(2, Label::No, Split::HeldOut),
        ],
    );
    let candidate = entry(
        1,
        json!({"kind":"finding","path":[]}),
        Some("passed"),
        Some(5),
    );
    let report = score(
        &[bundle(vec![candidate.clone(), candidate], Some(0.0), 10)],
        &set,
    )
    .unwrap();
    assert_eq!(
        report.end_result.inconclusive,
        vec![
            CaseId::new("case_1").unwrap(),
            CaseId::new("case_2").unwrap()
        ]
    );
    assert!(report.end_result.missed_findings.is_empty());
    assert!(report.end_result.correct_negatives.is_empty());
    assert_eq!(report.steps[0].unknown.len(), 2);
}

#[test]
fn filter_misses_require_complete_input_discovery_and_other_absence_is_unknown() {
    let root = tempfile::tempdir().unwrap();
    let set = freeze(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let mut observed = bundle(vec![], None, 0);
    let located = located_input(&set);
    observed.inputs = vec![located.clone()];
    let report = score(&[observed.clone()], &set).unwrap();
    assert_eq!(report.filter.missed, vec![CaseId::new("case_1").unwrap()]);
    let mut unlocatable = located.clone();
    unlocatable.location = sunstack_rules::InputLocation::Unlocatable {
        reason: sunstack_evidence::UnlocatableReason::Missing,
    };
    let mut unreadable = unlocatable.clone();
    unreadable.location = sunstack_rules::InputLocation::Unlocatable {
        reason: sunstack_evidence::UnlocatableReason::Unreadable,
    };
    let mut partial = located.clone();
    partial
        .unfit
        .push(sunstack_evidence::CoverageGap::MalformedRecord { position: 1 });
    for inputs in [
        vec![unlocatable.clone()],
        vec![unreadable],
        vec![partial],
        vec![],
        vec![located, unlocatable],
    ] {
        observed.inputs = inputs;
        let report = score(&[observed.clone()], &set).unwrap();
        assert!(
            report.filter.missed.is_empty(),
            "unknown input coverage cannot prove a filter miss"
        );
        assert_eq!(
            serde_json::to_value(report.filter).unwrap(),
            json!({"positive_cases":1,"kept":0,"missed":[],"unknown":["case_1"]})
        );
        assert_eq!(
            report.end_result.inconclusive,
            vec![CaseId::new("case_1").unwrap()]
        );
    }
}

#[test]
fn filter_unknown_uses_explicit_capture_and_own_rule_discovery_status() {
    let root = tempfile::tempdir().unwrap();
    let set = freeze(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let mut observed = bundle(vec![], None, 0);
    let complete = located_input(&set);
    observed.inputs = vec![complete.clone()];
    assert_eq!(
        score(&[observed.clone()], &set).unwrap().filter.missed,
        vec![CaseId::new("case_1").unwrap()]
    );

    // An unfinished JSONL tail need not emit an unfit record. Location and the gap list
    // therefore cannot stand in for the captured bytes' explicit completeness receipt.
    let mut incomplete_capture = complete.clone();
    incomplete_capture.capture = sunstack_rules::InputCaptureStatus::Incomplete;
    assert!(incomplete_capture.unfit.is_empty());
    let mut not_started = complete.clone();
    not_started.discovery[0].status = sunstack_rules::RuleDiscoveryStatus::NotStarted;
    let mut stopped = complete.clone();
    stopped.discovery[0].status = sunstack_rules::RuleDiscoveryStatus::Stopped;
    let mut different_rule = complete.clone();
    different_rule.discovery[0].rule.rule_id = sunstack_rules::RuleId::new("other_rule").unwrap();
    let mut no_receipt = complete.clone();
    no_receipt.discovery.clear();
    let mut ambiguous = complete.clone();
    ambiguous.discovery.push(ambiguous.discovery[0].clone());
    let mut observations = vec![];
    for (name, input) in [
        ("incomplete_capture", incomplete_capture),
        ("not_started", not_started),
        ("stopped", stopped),
        ("different_rule", different_rule),
        ("no_receipt", no_receipt),
        ("ambiguous", ambiguous),
    ] {
        observed.inputs = vec![input];
        let filter = score(&[observed.clone()], &set).unwrap().filter;
        observations.push((name, serde_json::to_value(filter).unwrap()));
    }
    // An unrelated rule's stopped receipt does not undo this rule's completed discovery.
    let mut both_rules = complete;
    let mut other = both_rules.discovery[0].clone();
    other.rule.rule_id = sunstack_rules::RuleId::new("other_rule").unwrap();
    other.status = sunstack_rules::RuleDiscoveryStatus::Stopped;
    both_rules.discovery.push(other);
    observed.inputs = vec![both_rules];
    let filter = score(&[observed], &set).unwrap().filter;
    assert_eq!(filter.missed, vec![CaseId::new("case_1").unwrap()]);
    assert!(filter.unknown.is_empty());
    let unknown = json!({"positive_cases":1,"kept":0,"missed":[],"unknown":["case_1"]});
    let expected = [
        "incomplete_capture",
        "not_started",
        "stopped",
        "different_rule",
        "no_receipt",
        "ambiguous",
    ]
    .into_iter()
    .map(|name| (name, unknown.clone()))
    .collect::<Vec<_>>();
    assert_eq!(
        observations, expected,
        "explicit capture and own-rule discovery must establish each miss"
    );
}

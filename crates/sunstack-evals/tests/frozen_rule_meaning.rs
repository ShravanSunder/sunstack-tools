#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use support::*;

fn definitions(root: &std::path::Path, meaning: &str) {
    std::fs::write(root.join("set.json"),json!({"schema_version":1,"name":"frozen","description":"Frozen conformance meaning","inputs":{"subject":{"kind":"conversation"}},"applicability":[],"common_ruleset_instructions":null,"rule_paths":["rule.jsonl"]}).to_string()).unwrap();
    std::fs::write(root.join("rule.jsonl"),json!({"rule_id":"test_rule","evidence":[{"id":"target","input":"subject","select":{"kind":"conversation","turns":{"kind":"all"},"user_messages":"included","assistant_messages":"all","tool_use":{"kind":"included","tools":{"kind":"all"},"results":"included"},"hook_injected":"included"}}],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","pass_means":meaning,"question":{"type":"noul","instructions":"Does it conform?"},"threshold":0.8}}).to_string()).unwrap();
}
#[test]
fn frozen_conformance_meaning_drives_end_truth_and_recall_but_step_answers_stay_raw() {
    let root = tempfile::tempdir().unwrap();
    let mut specification = input(
        root.path(),
        vec![
            case(1, Label::No, Split::HeldOut),
            case(2, Label::Yes, Split::HeldOut),
        ],
    );
    definitions(root.path(), "conforms");
    specification.rule.rule_set = "set.json".into();
    specification.definition_files =
        vec![root.path().join("set.json"), root.path().join("rule.jsonl")];
    let set = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    let mut defect = entry(
        1,
        json!({"kind":"finding","path":["answer_1"]}),
        Some("did_not_pass"),
        None,
    );
    let mut conforms = entry(
        2,
        json!({"kind":"dropped","at":{"kind":"verdict"},"answer":"answer_1"}),
        Some("passed"),
        None,
    );
    for candidate in [&mut defect, &mut conforms] {
        candidate.rule = specification.rule.clone();
        candidate.candidate_id.rule = specification.rule.clone();
    }
    let report = score(&[bundle(&set, vec![defect, conforms], Some(0.0), 0)], &set).unwrap();
    assert_eq!(
        report.end_result.true_findings,
        vec![CaseId::new("case_1").unwrap()]
    );
    assert_eq!(
        report.end_result.correct_negatives,
        vec![CaseId::new("case_2").unwrap()]
    );
    assert_eq!(report.steps[0].correct.len(), 2);
    assert_eq!(report.filter.positive_cases, 1);
    assert_eq!(report.filter.kept, 1);
}

#[test]
fn frozen_definition_copies_survive_live_edits_and_refuse_tampered_copies() {
    let root = tempfile::tempdir().unwrap();
    let specification = input(root.path(), vec![case(1, Label::No, Split::HeldOut)]);
    definitions(root.path(), "conforms");
    let set = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    assert_eq!(set.pass_means(), sunstack_rules::PassMeans::Conforms);
    assert_eq!(
        set.manifest().definition_files[0].path,
        std::path::PathBuf::from("set.json")
    );
    definitions(root.path(), "finding");
    let loaded = load_case_set(&root.path().join("cases/manifest.json")).unwrap();
    assert_eq!(loaded.pass_means(), sunstack_rules::PassMeans::Conforms);
    assert_eq!(loaded.rule_version(), set.rule_version());
    let copied = root.path().join("cases/definitions/rule.jsonl");
    let original = std::fs::read_to_string(&copied).unwrap();
    std::fs::write(copied, original.replace("conforms", "finding")).unwrap();
    assert!(matches!(
        load_case_set(&root.path().join("cases/manifest.json")),
        Err(EvalError::Integrity { .. })
    ));
}
#[test]
fn missing_named_rule_or_unfrozen_dependency_refuses_freeze_and_load_by_name() {
    for (missing_rule, missing_dependency) in [(true, false), (false, true)] {
        let root = tempfile::tempdir().unwrap();
        let mut specification = input(root.path(), vec![case(1, Label::No, Split::HeldOut)]);
        if missing_rule {
            specification.rule.rule_id = sunstack_rules::RuleId::new("missing.rule").unwrap();
        }
        if missing_dependency {
            specification.definition_files.truncate(1);
        }
        let error = freeze_cases(&root.path().join("cases"), &specification).unwrap_err();
        let EvalError::Definitions { diagnostics } = error else {
            panic!("{error:?}")
        };
        let wire = serde_json::to_value(diagnostics).unwrap();
        assert!(wire.to_string().contains(if missing_rule {
            "missing.rule"
        } else {
            "rule.jsonl"
        }));
        assert!(!root.path().join("cases").exists());
    }
    let root = tempfile::tempdir().unwrap();
    let set = freeze(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let path = root.path().join("cases/manifest.json");
    let mut manifest = serde_json::to_value(set.manifest()).unwrap();
    manifest["rule"]["rule_id"] = json!("missing.rule");
    std::fs::write(&path, manifest.to_string()).unwrap();
    let error = load_case_set(&path).unwrap_err();
    let EvalError::Definitions { diagnostics } = error else {
        panic!("{error:?}")
    };
    assert!(
        serde_json::to_string(&diagnostics)
            .unwrap()
            .contains("missing.rule")
    );
}
#[test]
fn definition_root_ownership_and_relative_paths_are_required_at_freeze_and_load() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let mut specification = input(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    std::fs::write(outside.path().join("outside.json"), "{}").unwrap();
    specification
        .definition_files
        .push(outside.path().join("outside.json"));
    assert!(
        matches!(freeze_cases(&root.path().join("outside"),&specification),Err(EvalError::Invalid {field}) if field=="definition_files.outside_root")
    );
    specification.definition_files.pop();
    specification
        .definition_files
        .push(root.path().join("../escaping.json"));
    assert!(
        matches!(freeze_cases(&root.path().join("escape"),&specification),Err(EvalError::Invalid {field}) if field=="definition_files.path")
    );
    specification.definition_files.pop();
    specification.rule.rule_set = root.path().join("set.json");
    assert!(
        matches!(freeze_cases(&root.path().join("absolute"),&specification),Err(EvalError::Invalid {field}) if field=="rule.rule_set")
    );
    specification.rule.rule_set = "set.json".into();
    let set = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    let path = root.path().join("cases/manifest.json");
    let mut manifest = serde_json::to_value(set.manifest()).unwrap();
    manifest["definition_files"][0]["path"] = json!("../escaping.json");
    std::fs::write(&path, manifest.to_string()).unwrap();
    assert!(
        matches!(load_case_set(&path),Err(EvalError::Invalid {field}) if field=="manifest.definition_files.path")
    );
}

#[test]
fn conformance_defects_use_no_labels_for_repository_recall_with_completion_guards() {
    use sunstack_evidence::*;
    use sunstack_rules::*;
    let root = tempfile::tempdir().unwrap();
    let mut specification = input(root.path(), vec![case(1, Label::No, Split::HeldOut)]);
    definitions(root.path(), "conforms");
    specification.sources[0].source = SourceInput::Code {
        repository: RepositoryId::new("docs").unwrap(),
        revision: RevisionHead::Commit {
            revision: "a".repeat(40),
        },
        paths: vec![QueryPath::Exact {
            path: "guide.md".into(),
        }],
    };
    let source_id = specification.sources[0].source.source_identity().unwrap();
    specification.labels[0].source_anchor=serde_json::from_value(json!({"kind":"document_unit","input_id":source_id,"unit_kind":"paragraph","occurrence":{"kind":"unit","span":{"source":{"kind":"repository_file","repository":"docs","revision":{"kind":"commit","commit_id":"a".repeat(40),"side":"after"},"path":"guide.md","content_identity":"frozen-guide"},"byte_start":0,"byte_end":5,"start_line":1,"start_column":1,"end_line":1,"end_column":6}}})).unwrap();
    std::fs::write(root.path().join("guide.md"), "alpha\n").unwrap();
    specification.source_files = vec![root.path().join("guide.md")];
    let frozen = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    for (capture, status, missed, unknown) in [
        (
            InputCaptureStatus::Complete,
            RuleDiscoveryStatus::Completed,
            1,
            0,
        ),
        (
            InputCaptureStatus::Incomplete,
            RuleDiscoveryStatus::Completed,
            0,
            1,
        ),
        (
            InputCaptureStatus::Complete,
            RuleDiscoveryStatus::NotStarted,
            0,
            1,
        ),
    ] {
        let mut receipt = located_input(&frozen);
        receipt.location = InputLocation::Repository {
            repository: RepositoryId::new("docs").unwrap(),
        };
        receipt.capture = capture;
        receipt.discovery[0].status = status;
        let mut observed = bundle(&frozen, vec![], Some(0.0), 0);
        observed.inputs = vec![receipt];
        let score = score(&[observed], &frozen).unwrap();
        assert_eq!(score.filter.positive_cases, 1);
        assert_eq!(score.filter.kept, 0);
        assert_eq!(score.filter.missed.len(), missed);
        assert_eq!(score.filter.unknown.len(), unknown);
    }
}

#[test]
fn conformance_keeps_nonverdict_labels_raw_and_unknown_labels_out_of_truth_and_recall() {
    use sunstack_rules::*;
    let root = tempfile::tempdir().unwrap();
    let mut gate = case(1, Label::Yes, Split::HeldOut);
    gate.question = QuestionRef::RuleGate { position: 0 };
    let mut information = case(2, Label::Yes, Split::HeldOut);
    information.question =
        serde_json::from_value(json!({"kind":"informational","name":"detail"})).unwrap();
    let unknown = case(3, Label::Unknown, Split::HeldOut);
    let defect = case(4, Label::No, Split::HeldOut);
    let specification = input(
        root.path(),
        vec![gate.clone(), information.clone(), unknown, defect],
    );
    definitions(root.path(), "conforms");
    let frozen = freeze_cases(&root.path().join("cases"), &specification).unwrap();
    let mut gate_answer = entry(
        1,
        json!({"kind":"finding","path":["answer_1","verdict_1"]}),
        Some("passed"),
        None,
    );
    gate_answer.answers[0].question = gate.question.clone();
    gate_answer.answers[0].answer =
        Some(serde_json::from_value(json!({"type":"noul","yes_probability":0.9})).unwrap());
    gate_answer.answers.push(serde_json::from_value(json!({"answer_id":"verdict_1","question":{"kind":"verdict"},"page":null,"answer":{"type":"noul","yes_probability":0.1},"settlement":"did_not_pass"})).unwrap());
    let mut information_answer = entry(
        2,
        json!({"kind":"finding","path":["verdict_2"]}),
        Some("passed"),
        None,
    );
    information_answer.answers[0].question = information.question.clone();
    information_answer.answers[0].answer =
        Some(serde_json::from_value(json!({"type":"noul","yes_probability":0.9})).unwrap());
    information_answer.answers.push(serde_json::from_value(json!({"answer_id":"verdict_2","question":{"kind":"verdict"},"page":null,"answer":{"type":"noul","yes_probability":0.1},"settlement":"did_not_pass"})).unwrap());
    let unknown_answer = entry(
        3,
        serde_json::json!({"kind":"finding","path":["answer_1"]}),
        Some("did_not_pass"),
        None,
    );
    let mut observed = bundle(
        &frozen,
        vec![gate_answer, information_answer, unknown_answer],
        Some(0.0),
        0,
    );
    observed.inputs = vec![located_input(&frozen)];
    let grade = score(&[observed], &frozen).unwrap();
    assert_eq!(grade.filter.positive_cases, 3);
    assert_eq!(grade.filter.kept, 2);
    assert_eq!(grade.filter.missed, vec![CaseId::new("case_4").unwrap()]);
    assert!(grade.filter.unknown.is_empty());
    assert_eq!(grade.unknown_labels, 1);
    let gate_step = grade
        .steps
        .iter()
        .find(|step| step.question == gate.question)
        .unwrap();
    assert_eq!(gate_step.correct, vec![gate.case_id]);
    assert!(gate_step.wrong.is_empty());
    assert!(gate_step.unknown.is_empty());
    // Informational operands have no independent binary threshold; their label remains raw for recall.
    let info_step = grade
        .steps
        .iter()
        .find(|step| step.question == information.question)
        .unwrap();
    assert!(info_step.correct.is_empty());
    assert!(info_step.wrong.is_empty());
    assert_eq!(info_step.unknown, vec![information.case_id]);
    assert!(grade.end_result.true_findings.is_empty());
    assert!(grade.end_result.false_findings.is_empty());
    assert!(grade.end_result.missed_findings.is_empty());
    assert!(grade.end_result.correct_negatives.is_empty());
    assert_eq!(
        grade.end_result.inconclusive,
        vec![CaseId::new("case_4").unwrap()]
    );
}

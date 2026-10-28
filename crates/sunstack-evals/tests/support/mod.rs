#![allow(dead_code)]
use serde_json::json;
use std::path::Path;
use sunstack_evals::*;
use sunstack_rules::*;

pub fn occurrence(number: usize) -> sunstack_evidence::SourceOccurrence {
    serde_json::from_value(json!({"kind":"record", "record":format!("record_{number}"),
        "author":"assistant", "positions":[{"line":number,"byte_offset":0}],
        "matched_at":{"line":number,"byte_offset":0}, "field":"/body", "span":{"byte_start":0,"byte_end":4}})).unwrap()
}
pub fn rule() -> RuleRef {
    serde_json::from_value(json!({"rule_set":"rule.jsonl", "rule_id":"test_rule"})).unwrap()
}
pub fn case(number: usize, label: Label, split: Split) -> LabeledCase {
    LabeledCase {
        case_id: CaseId::new(format!("case_{number}")).unwrap(),
        source: CaseSourceId::new("source_1").unwrap(),
        session: SessionKey {
            provider: sunstack_evidence::ProviderName::new("codex").unwrap(),
            session: sunstack_evidence::SessionId::new("session_1").unwrap(),
        },
        project: Some(ProjectKey::new("project_1").unwrap()),
        source_anchor: occurrence(number),
        question: QuestionRef::Verdict {},
        label,
        split,
    }
}
pub fn input(root: &Path, cases: Vec<LabeledCase>) -> FreezeInput {
    std::fs::write(root.join("transcript.jsonl"), "frozen source bytes\n").unwrap();
    std::fs::write(root.join("rule.jsonl"), "frozen question bytes\n").unwrap();
    FreezeInput {
        rule: rule(),
        split: Split::HeldOut,
        sources: vec![CaseSource {
            source_id: CaseSourceId::new("source_1").unwrap(),
            source: serde_json::from_value(
                json!({"kind":"session_pair","provider":"codex","session":"session_1"}),
            )
            .unwrap(),
            evidence: Default::default(),
        }],
        labels: cases,
        source_files: vec![root.join("transcript.jsonl")],
        definition_files: vec![root.join("rule.jsonl")],
    }
}
pub fn freeze(root: &Path, cases: Vec<LabeledCase>) -> FrozenCaseSet {
    freeze_cases(&root.join("cases"), &input(root, cases)).unwrap()
}
pub fn entry(
    number: usize,
    outcome: serde_json::Value,
    settlement: Option<&str>,
    elapsed: Option<u64>,
) -> CandidateEntry {
    let occurrence = occurrence(number);
    let id = occurrence.occurrence_identity().unwrap();
    serde_json::from_value(json!({"candidate_id":{"rule":rule(),"occurrence":id},
        "rule":rule(),"occurrence":occurrence,"outcome":outcome,
        "answers":settlement.map(|settlement| vec![json!({"answer_id":"answer_1", "question":{"kind":"verdict"},
             "page":null, "answer":null, "settlement":settlement})]).unwrap_or_default(),
        "privacy_view":{"grants":[]}, "elapsed_ms":elapsed})).unwrap()
}
pub fn bundle(
    entries: Vec<CandidateEntry>,
    cost: Option<f64>,
    wall_time_ms: u64,
) -> CandidateRunReport {
    CandidateRunReport {
        completeness: RunCompleteness::NothingFound {},
        candidates: entries,
        dropped_samples: vec![],
        inputs: vec![],
        privacy: PrivacyReport::default(),
        usage: RunUsage {
            requests: 4,
            attempts: 5,
            input_tokens: 100,
            output_tokens: 20,
            cost_usd: cost,
            wall_time_ms,
        },
    }
}

pub fn located_input(set: &FrozenCaseSet) -> InputReport {
    InputReport {
        input_id: set.sources()[0].source.source_identity().unwrap(),
        location: InputLocation::Located {
            path: "transcript.jsonl".into(),
            format: sunstack_evidence::TranscriptFormatName::new("codex_rollout_jsonl").unwrap(),
        },
        records: 9,
        unfit: vec![],
        capture: InputCaptureStatus::Complete,
        discovery: vec![RuleDiscovery {
            rule: rule(),
            status: RuleDiscoveryStatus::Completed,
        }],
    }
}

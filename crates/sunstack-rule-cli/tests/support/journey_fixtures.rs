#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::support::workspace_root;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub const EXAMPLES: &str = "docs/specs/2026-10-12-rust-rule-engine/examples";
pub const FINAL_REVIEW: &str = "Review complete. Three findings follow, each with evidence and route. 1. The Program Design assigns framing, signals and exits to sunstack-rule-cli; evidence: its crate table; route: implementation. 2. Specification C14 separates cancelled or incomplete execution from unfavorable verdicts; route: tests. 3. Specification C8 uses the caller token limit when no provider ceiling is declared; route: implementation. The Program Design at the pinned commit is fully addressed. No additional issues found.";
fn example(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(workspace_root().join(EXAMPLES).join(name)).unwrap())
        .unwrap()
}
pub fn skill_input(endpoint: &str) -> Value {
    let mut input = example("skill-lint.invocation.json");
    settings(&mut input, endpoint);
    input
}
fn settings(input: &mut Value, endpoint: &str) {
    input["backend"]["endpoint"] = json!(endpoint);
    input["attempts"]["attempt_timeout_ms"] = json!(60000);
    input["attempts"]["retry_count"] = json!(0);
}
pub struct HandoffFixture {
    _directory: tempfile::TempDir,
    pub input: Value,
}
impl HandoffFixture {
    pub fn new(endpoint: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let transcript_path = directory.path().join("reviewer-codex.jsonl");
        // The accepted S2 public Codex composition stays byte-identical. The
        // reviewer turn/read attempts/handback below are synthetic additions,
        // not a live host transcript or a claim that an agent understood it.
        let mut bytes =
            include_bytes!("../../../sunstack-evidence/tests/fixtures/codex-composed.jsonl")
                .to_vec();
        let mut records = vec![
            json!({"timestamp":"2026-10-15T21:00:00Z","type":"response_item","payload":{"type":"message","role":"user","id":"review-request","content":[{"type":"input_text","text":"Review the declared Program Design and return the finished review with evidence and routes."}]}}),
        ];
        for (index, path) in ["specification.md", "program-design.md"]
            .into_iter()
            .enumerate()
        {
            let relative = format!("docs/specs/2026-10-12-rust-rule-engine/{path}");
            let call_id = format!("review-read-{index}");
            records.push(json!({"timestamp":"2026-10-15T21:01:00Z","type":"response_item","payload":{"type":"function_call","call_id":call_id,"name":"exec_command","arguments":serde_json::to_string(&json!({"cmd":format!("sed -n '1,5p' {relative}")})).unwrap()}}));
            let excerpt = std::fs::read_to_string(workspace_root().join(&relative))
                .unwrap()
                .lines()
                .take(5)
                .collect::<Vec<_>>()
                .join("\n");
            records.push(json!({"timestamp":"2026-10-15T21:01:01Z","type":"response_item","payload":{"type":"function_call_output","call_id":call_id,"output":excerpt}}));
        }
        records.push(json!({"timestamp":"2026-10-15T21:04:05Z","type":"response_item","payload":{"type":"message","role":"assistant","id":"review-handback","content":[{"type":"output_text","text":FINAL_REVIEW}]}}));
        for record in records {
            bytes.extend_from_slice(serde_json::to_string(&record).unwrap().as_bytes());
            bytes.push(b'\n');
        }
        std::fs::write(&transcript_path, bytes).unwrap();
        let mut input = example("handoff-monitor.invocation.json");
        settings(&mut input, endpoint);
        input["definitions"]["binding_groups"][0]["bindings"]["session"]["transcript_path"] =
            json!(transcript_path);
        input["definitions"]["binding_groups"][0]["bindings"]["session"]["final_assistant_text"] =
            json!(FINAL_REVIEW);
        input["payload_schemas"][0]["path"] = json!(
            workspace_root()
                .join("crates/sunstack-evidence/tests/fixtures/agent-job-packet.schema.json")
        );
        Self {
            _directory: directory,
            input,
        }
    }
}
pub fn fingerprints(input: &Value) -> Value {
    let mut fingerprints = serde_json::Map::new();
    for name in [
        "SKILL.md",
        "skill-quality.ruleset.json",
        "skill-rules.jsonl",
        "skill-lint.invocation.json",
        "review-handoff.ruleset.json",
        "review-handoff-rules.jsonl",
        "commission-completeness.ruleset.json",
        "commission-rules.jsonl",
        "agent-handoff.router.json",
        "handoff-monitor.invocation.json",
    ] {
        fingerprints.insert(
            name.into(),
            json!(sunstack_evidence::content_digest(
                &std::fs::read(workspace_root().join(EXAMPLES).join(name)).unwrap()
            )),
        );
    }
    if let Some(path) =
        input["definitions"]["binding_groups"][0]["bindings"]["session"]["transcript_path"].as_str()
    {
        fingerprints.insert(
            "reviewer_transcript".into(),
            json!(sunstack_evidence::content_digest(
                &std::fs::read(path).unwrap()
            )),
        );
    }
    for schema in input["payload_schemas"].as_array().unwrap() {
        fingerprints.insert(
            "generated_agent_job_schema".into(),
            json!(sunstack_evidence::content_digest(
                &std::fs::read(schema["path"].as_str().unwrap()).unwrap()
            )),
        );
    }
    Value::Object(fingerprints)
}
pub fn complete_report(wire: &Value) -> &Value {
    assert_eq!(
        wire["result"]["outcome"]["kind"], "complete_report",
        "{wire}"
    );
    &wire["result"]["outcome"]["report"]
}
pub fn coverage(report: &Value) -> Value {
    let mut questions = BTreeMap::<String, usize>::new();
    for request in report["requests"].as_array().unwrap() {
        if request["status"]["kind"] == "sent" {
            for question in request["questions"].as_array().unwrap() {
                *questions.entry(question["owner"].to_string()).or_default() += 1;
            }
        }
    }
    let occurrences: Vec<Value> = report["occurrences"].as_array().unwrap().iter().map(|occurrence| {
        assert_eq!(occurrence["kind"], "judged");
        assert_eq!(occurrence["selected_by"],json!(["full_scope"]));
        json!({"key":occurrence["key"],"selected_by":occurrence["selected_by"],"target":occurrence["judgment"]["target"],"severity":occurrence["judgment"]["declared_severity"],"value":occurrence["judgment"]["verdict"]["value"],"result":occurrence["judgment"]["verdict"]["result"]})
    }).collect();
    json!({"occurrences":occurrences,"question_owners":questions})
}
pub fn check_journey(report: &Value, handoff: bool) {
    assert_eq!(report["change_selections"], json!([]));
    let expected = if handoff {
        vec![
            "reviewer.read-required-sources",
            "reviewer.stayed-read-only",
            "reviewer.handback-addresses-target",
        ]
    } else {
        vec![
            "skill.activation",
            "skill.output.specificity",
            "skill.verification.present",
        ]
    };
    let occurrences = report["occurrences"].as_array().unwrap();
    assert_eq!(occurrences.len(), expected.len());
    for (occurrence, rule_id) in occurrences.iter().zip(expected) {
        assert_eq!(occurrence["key"]["rule_id"], rule_id);
        assert_eq!(occurrence["kind"], "judged");
        let judgment = &occurrence["judgment"];
        let expected_names: BTreeSet<&str> = match rule_id {
            "skill.activation" => vec!["task_kind"],
            "skill.output.specificity" => vec!["specificity", "names_file"],
            "reviewer.read-required-sources" => vec!["unread_sources"],
            "reviewer.handback-addresses-target" => vec!["coverage", "has_findings"],
            _ => vec![],
        }
        .into_iter()
        .collect();
        assert_eq!(
            judgment["answers"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            expected_names
        );
        assert!(matches!(
            judgment["declared_severity"].as_str(),
            Some("block" | "nudge")
        ));
        assert!(!judgment["target"].is_null());
        for reference in judgment["answers"].as_object().unwrap().values() {
            let index = usize::try_from(reference["request_index"].as_u64().unwrap()).unwrap();
            let question = reference["question_name"].as_str().unwrap();
            assert!(
                report["requests"][index]["status"]["outcome"]["answers"]
                    .get(question)
                    .is_some()
            );
        }
        check_verdict_arithmetic(report, occurrence);
    }
    if handoff {
        assert_eq!(report["origin"]["kind"], "hook");
        assert_eq!(report["origin"]["session"]["kind"], "observed");
        assert_eq!(report["origin"]["agent"]["type"]["value"], "reviewer");
        assert_eq!(report["origin"]["host"], "codex");
        assert_eq!(report["origin"]["event"], "Stop");
        assert_eq!(
            report["origin"]["session"]["value"],
            "019a0000-0000-7000-8000-000000000001"
        );
        assert_eq!(
            report["origin"]["turn"]["value"],
            "019a0000-0000-7000-8000-000000000042"
        );
        assert_eq!(report["routing"]["groups"][0]["outcome"]["kind"], "routed");
        assert_eq!(
            report["routing"]["groups"][0]["outcome"]["label"],
            "review_handback"
        );
        assert_eq!(report["cursors"].as_array().unwrap().len(), 1);
    } else {
        let bytes = std::fs::read(workspace_root().join(EXAMPLES).join("SKILL.md")).unwrap();
        for (index, occurrence) in occurrences.iter().enumerate() {
            let target = &occurrence["judgment"]["target"];
            assert_eq!(target["target_id"], format!("U{:03}", index + 1));
            let span = &target["span"];
            let start = usize::try_from(span["byte_start"].as_u64().unwrap()).unwrap();
            let end = usize::try_from(span["byte_end"].as_u64().unwrap()).unwrap();
            let selected = std::str::from_utf8(&bytes[start..end]).unwrap();
            assert!(selected.starts_with(&format!("U{:03}:", index + 1)));
            assert!(Path::new(span["source"]["path"].as_str().unwrap()).ends_with("SKILL.md"));
        }
    }
}
fn native_answer<'report>(report: &'report Value, reference: &Value) -> &'report Value {
    let index = usize::try_from(reference["request_index"].as_u64().unwrap()).unwrap();
    &report["requests"][index]["status"]["outcome"]["answers"]
        [reference["question_name"].as_str().unwrap()]
}
fn check_verdict_arithmetic(report: &Value, occurrence: &Value) {
    let judgment = &occurrence["judgment"];
    let verdict = &judgment["verdict"];
    let (expected, threshold, block, band) = match occurrence["key"]["rule_id"].as_str().unwrap() {
        "skill.output.specificity" => {
            let score = native_answer(report, &judgment["answers"]["specificity"])["score"]
                .as_f64()
                .unwrap();
            let names =
                native_answer(report, &judgment["answers"]["names_file"])["yes_probability"]
                    .as_f64()
                    .unwrap();
            (0.75 * score / 3.0 + 0.25 * names, 0.67, false, None)
        }
        "reviewer.handback-addresses-target" => {
            let score = native_answer(report, &judgment["answers"]["coverage"])["score"]
                .as_f64()
                .unwrap();
            let findings =
                native_answer(report, &judgment["answers"]["has_findings"])["yes_probability"]
                    .as_f64()
                    .unwrap();
            (0.7 * score / 2.0 + 0.3 * findings, 0.6, false, None)
        }
        rule_id => {
            let probability = native_answer(report, &verdict["basis"]["answer"])["yes_probability"]
                .as_f64()
                .unwrap();
            match rule_id {
                "skill.activation" => (probability, 0.7, false, None),
                "skill.verification.present" => (probability, 0.8, true, Some(0.3)),
                "reviewer.read-required-sources" => (probability, 0.8, true, Some(0.3)),
                "reviewer.stayed-read-only" => (probability, 0.8, true, None),
                _ => panic!("unexpected example rule {rule_id}"),
            }
        }
    };
    assert!((verdict["value"].as_f64().unwrap() - expected).abs() < 1e-12);
    assert_eq!(
        judgment["declared_severity"],
        if block { "block" } else { "nudge" }
    );
    assert_eq!(verdict["threshold"], threshold);
    if expected >= threshold {
        assert_eq!(verdict["result"]["kind"], "pass");
    } else {
        assert_eq!(verdict["result"]["kind"], "fail");
        let activation = if !block {
            "nudge"
        } else if band.is_some_and(|band| expected > band) {
            "nudge_above_band"
        } else {
            "block"
        };
        assert_eq!(verdict["result"]["activation"]["kind"], activation);
    }
}
pub fn receipt_summary(report: &Value) -> Vec<Value> {
    report["requests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|request| request["status"]["kind"] == "sent")
        .map(|request| {
            let outcome = &request["status"]["outcome"];
            assert_eq!(outcome["kind"], "completed");
            assert_eq!(outcome["attempts"].as_array().unwrap().len(), 1);
            assert!(
                !outcome["receipt"]["resolved_model"]
                    .as_str()
                    .unwrap()
                    .is_empty()
            );
            assert!(
                outcome["receipt"]["usage"]["input_tokens"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            outcome["receipt"].clone()
        })
        .collect()
}

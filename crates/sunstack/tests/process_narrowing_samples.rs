#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
#[path = "support/narrowing_sample_fixture.rs"]
mod narrowing_sample_fixture;
#[path = "support/narrowing_sample_runtime.rs"]
mod narrowing_sample_runtime;
mod support;

use controlled_http::ControlledServer;
use narrowing_sample_runtime::{noul_response, report_from_process_output};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

const EXPECTED_FIFTY_DROP_INDICES: [usize; 20] = [
    0, 2, 5, 7, 10, 12, 15, 17, 20, 22, 25, 27, 30, 32, 35, 37, 40, 42, 45, 47,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateDisposition {
    ClearNarrowingDrop,
    UncertainAtNarrowing,
    Finding,
    DropAtRuleGate,
}

struct CandidateCase {
    text: String,
    disposition: CandidateDisposition,
}

fn cases_with_fifty_drops() -> Vec<CandidateCase> {
    let mut cases = (0..50)
        .map(|index| CandidateCase {
            text: format!("MATCH CLEAR_DROP_{index:03}"),
            disposition: CandidateDisposition::ClearNarrowingDrop,
        })
        .collect::<Vec<_>>();
    cases.insert(
        7,
        CandidateCase {
            text: "MATCH UNCERTAIN_NARROWING".into(),
            disposition: CandidateDisposition::UncertainAtNarrowing,
        },
    );
    cases.insert(
        23,
        CandidateCase {
            text: "MATCH KEPT_FINDING".into(),
            disposition: CandidateDisposition::Finding,
        },
    );
    cases.insert(
        39,
        CandidateCase {
            text: "MATCH RULE_GATE_DROP".into(),
            disposition: CandidateDisposition::DropAtRuleGate,
        },
    );
    cases
}

fn case_index_in_request(body: &Value, cases: &[CandidateCase]) -> usize {
    let serialized = body.to_string();
    let matching = cases
        .iter()
        .enumerate()
        .filter_map(|(index, case)| serialized.contains(&case.text).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1, "one candidate per request: {serialized}");
    matching[0]
}

fn response_for_clear_narrowing_drop(body: &Value) -> Value {
    noul_response(body, |question| {
        if question.to_string().contains("NARROW_SAMPLE") {
            0.1
        } else {
            0.9
        }
    })
}

fn mechanical_rejection_input(prefix: &str) -> Vec<String> {
    let mut records = (0..4)
        .map(|index| format!("MECHANICAL_REJECT_{prefix}_{index}"))
        .collect::<Vec<_>>();
    records.extend((0..5).map(|index| format!("MATCH NARROW_DROP_{prefix}_{index}")));
    records
}

fn response_for_case(body: &Value, case: &CandidateCase) -> Value {
    noul_response(body, |question| {
        let instructions = question.to_string();
        if instructions.contains("NARROW_SAMPLE") {
            match case.disposition {
                CandidateDisposition::ClearNarrowingDrop => 0.1,
                CandidateDisposition::UncertainAtNarrowing => 0.55,
                CandidateDisposition::Finding | CandidateDisposition::DropAtRuleGate => 0.9,
            }
        } else if instructions.contains("MIXED_GATE") {
            if case.disposition == CandidateDisposition::DropAtRuleGate {
                0.1
            } else {
                0.9
            }
        } else if instructions.contains("Is this occurrence a finding?") {
            0.9
        } else {
            panic!("unexpected request question: {question}")
        }
    })
}

fn case_is_terminal_in_request(body: &Value, case: &CandidateCase) -> bool {
    let instructions = body["questions"].to_string();
    match case.disposition {
        CandidateDisposition::ClearNarrowingDrop | CandidateDisposition::UncertainAtNarrowing => {
            instructions.contains("NARROW_SAMPLE")
        }
        CandidateDisposition::Finding => instructions.contains("Is this occurrence a finding?"),
        CandidateDisposition::DropAtRuleGate => instructions.contains("MIXED_GATE"),
    }
}

fn add_mixed_rule_gate(root: &Path) {
    let path = root.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let evidence = rule["evidence"].clone();
    rule["applicability"] = json!([{
        "kind":"decision",
        "question":{"type":"noul","instructions":"MIXED_GATE"},
        "evidence":evidence,
        "threshold":0.8,
        "uncertain_below":0.7
    }]);
    std::fs::write(path, rule.to_string()).unwrap();
}

async fn run_fifty_drop_matrix(root: &Path, cases: &[CandidateCase], hold_earliest: bool) -> Value {
    let mut server = ControlledServer::start().await;
    let records = cases
        .iter()
        .map(|case| case.text.clone())
        .collect::<Vec<_>>();
    let input = narrowing_sample_fixture::record_run(root, &server.endpoint, &[records], 20);
    add_mixed_rule_gate(root);
    let mut run = narrowing_sample_runtime::start_candidate_process(root, input).await;

    if hold_earliest {
        // The plan creates candidate groups and requests in source order. At
        // concurrency two, keep the first clear drop in flight while candidate
        // one finishes, then observe candidate two's new admission event.
        let first = server.next().await;
        let first_index = case_index_in_request(&first.body, cases);
        let second = server.next().await;
        let second_index = case_index_in_request(&second.body, cases);
        assert_eq!(
            BTreeSet::from([first_index, second_index]),
            BTreeSet::from([0, 1]),
            "concurrency two must expose the first two source candidates"
        );
        let (early, later, later_index) = if first_index == 0 {
            (first, second, second_index)
        } else {
            (second, first, first_index)
        };
        assert_eq!(case_index_in_request(&early.body, cases), 0);
        assert_eq!(case_index_in_request(&later.body, cases), 1);
        assert!(
            early.body["questions"]
                .to_string()
                .contains("NARROW_SAMPLE")
        );
        assert!(case_is_terminal_in_request(
            &later.body,
            &cases[later_index]
        ));
        let later_response = response_for_case(&later.body, &cases[later_index]);
        later.respond(200, later_response);
        let mut completed = BTreeSet::from([later_index]);

        let newly_admitted = server.next().await;
        let newly_admitted_index = case_index_in_request(&newly_admitted.body, cases);
        assert_eq!(newly_admitted_index, 2);
        assert!(case_is_terminal_in_request(
            &newly_admitted.body,
            &cases[newly_admitted_index]
        ));
        let newly_admitted_response =
            response_for_case(&newly_admitted.body, &cases[newly_admitted_index]);
        newly_admitted.respond(200, newly_admitted_response);
        completed.insert(newly_admitted_index);

        while completed.len() < cases.len() - 1 {
            let request = server.next().await;
            let index = case_index_in_request(&request.body, cases);
            assert_ne!(index, 0, "held first candidate must not reappear");
            let is_terminal = case_is_terminal_in_request(&request.body, &cases[index]);
            let response = response_for_case(&request.body, &cases[index]);
            request.respond(200, response);
            if is_terminal {
                completed.insert(index);
            }
        }
        assert!(!run.is_finished(), "candidate zero is still held");
        let early_response = response_for_case(&early.body, &cases[0]);
        early.respond(200, early_response);
        let output = run.await.unwrap();
        let report = report_from_process_output(&output);
        assert_eq!(
            server.finish().await,
            report["usage"]["requests"].as_u64().unwrap() as usize
        );
        report
    } else {
        let output = narrowing_sample_runtime::answer_until_process_finishes(
            &mut server,
            &mut run,
            |body| {
                let index = case_index_in_request(body, cases);
                response_for_case(body, &cases[index])
            },
        )
        .await;
        let report = report_from_process_output(&output);
        assert_eq!(
            server.finish().await,
            report["usage"]["requests"].as_u64().unwrap() as usize
        );
        report
    }
}

fn assert_fifty_drop_outcomes(report: &Value, cases: &[CandidateCase]) {
    let candidates = report["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), cases.len());
    for (index, case) in cases.iter().enumerate() {
        let candidate = candidates
            .iter()
            .find(|candidate| narrowing_sample_runtime::candidate_line(candidate) == index + 1)
            .unwrap_or_else(|| panic!("missing candidate at source line {}", index + 1));
        match case.disposition {
            CandidateDisposition::ClearNarrowingDrop => {
                let answer = narrowing_sample_runtime::narrowing_answer(candidate);
                assert_eq!(candidate["outcome"]["kind"], "dropped");
                assert_eq!(candidate["outcome"]["at"]["kind"], "narrowing");
                assert_eq!(answer["answer"]["yes_probability"], 0.1);
                assert_eq!(candidate["outcome"]["answer"], answer["answer_id"]);
            }
            CandidateDisposition::UncertainAtNarrowing => {
                let answer = narrowing_sample_runtime::narrowing_answer(candidate);
                assert_eq!(candidate["outcome"]["kind"], "uncertain");
                assert_eq!(candidate["outcome"]["at"]["kind"], "narrowing");
                assert_eq!(answer["answer"]["yes_probability"], 0.55);
                assert_eq!(
                    candidate["outcome"]["answers"],
                    json!([answer["answer_id"]])
                );
            }
            CandidateDisposition::Finding => {
                assert_eq!(candidate["outcome"]["kind"], "finding");
                assert_eq!(
                    narrowing_sample_runtime::narrowing_answer(candidate)["answer"]["yes_probability"],
                    0.9
                );
            }
            CandidateDisposition::DropAtRuleGate => {
                let gate = candidate["answers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|answer| answer["question"]["kind"] == "rule_gate")
                    .unwrap();
                assert_eq!(candidate["outcome"]["kind"], "dropped");
                assert_eq!(candidate["outcome"]["at"]["kind"], "rule_gate");
                assert_eq!(candidate["outcome"]["answer"], gate["answer_id"]);
                assert_eq!(gate["answer"]["yes_probability"], 0.1);
            }
        }
    }
}

fn assert_fifty_drop_samples(report: &Value, cases: &[CandidateCase]) {
    let clear_drop_lines = cases
        .iter()
        .enumerate()
        .filter_map(|(index, case)| {
            (case.disposition == CandidateDisposition::ClearNarrowingDrop).then_some(index + 1)
        })
        .collect::<Vec<_>>();
    let expected_lines = EXPECTED_FIFTY_DROP_INDICES
        .map(|index| clear_drop_lines[index])
        .to_vec();
    let samples = report["dropped_samples"].as_array().unwrap();
    assert_eq!(samples.len(), EXPECTED_FIFTY_DROP_INDICES.len());

    let sample_lines = samples
        .iter()
        .map(narrowing_sample_runtime::candidate_line)
        .collect::<Vec<_>>();
    assert_eq!(sample_lines, expected_lines);
    for sample in samples {
        assert_eq!(sample["pass"], "narrowing");
        assert_eq!(sample["rule"]["rule_id"], "candidate.one");
        let line = narrowing_sample_runtime::candidate_line(sample);
        let expected_case = &cases[line - 1];
        let candidate = report["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| narrowing_sample_runtime::candidate_line(candidate) == line)
            .unwrap();
        assert_eq!(candidate["rule"], sample["rule"]);
        assert_eq!(candidate["occurrence"], sample["occurrence"]);
        assert_eq!(
            serde_json::from_str::<Value>(sample["excerpt"].as_str().unwrap()).unwrap(),
            json!({
                "kind":"message",
                "blocks":[{
                "block":{"kind":"text","text":expected_case.text.clone()},
                    "private_kind":null
                }]
            })
        );
    }

    let mut lexical_id_lines = report["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|candidate| {
            candidate["outcome"]["kind"] == "dropped"
                && candidate["outcome"]["at"]["kind"] == "narrowing"
        })
        .map(|candidate| {
            (
                candidate["occurrence"]["record"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                narrowing_sample_runtime::candidate_line(candidate),
            )
        })
        .collect::<Vec<_>>();
    lexical_id_lines.sort_by(|left, right| left.0.cmp(&right.0));
    assert_ne!(
        lexical_id_lines
            .into_iter()
            .map(|(_, line)| line)
            .collect::<Vec<_>>(),
        clear_drop_lines,
        "fixture must make lexical opaque-ID order differ from source order"
    );
}

#[tokio::test]
async fn narrowing_samples_match_r10_and_ignore_completion_order() {
    let root = tempfile::tempdir().unwrap();
    let cases = cases_with_fifty_drops();

    let baseline = run_fifty_drop_matrix(root.path(), &cases, false).await;
    // Assert every intended narrowing answer and terminal before inspecting the
    // sample list. On the pre-fix build, this reaches 50 actual drops, then the
    // exact roster assertion fails because no narrowing samples are returned.
    assert_fifty_drop_outcomes(&baseline, &cases);
    assert_fifty_drop_samples(&baseline, &cases);

    let delayed = run_fifty_drop_matrix(root.path(), &cases, true).await;
    assert_fifty_drop_outcomes(&delayed, &cases);
    assert_fifty_drop_samples(&delayed, &cases);
    assert_eq!(baseline["dropped_samples"], delayed["dropped_samples"]);
}

#[tokio::test]
async fn sample_limits_are_per_input_and_pass_and_zero_keeps_real_outcomes() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let input = narrowing_sample_fixture::record_run(
        root.path(),
        &server.endpoint,
        &[
            mechanical_rejection_input("FIRST"),
            mechanical_rejection_input("SECOND"),
        ],
        2,
    );
    let mut run = narrowing_sample_runtime::start_candidate_process(root.path(), input).await;
    let output = narrowing_sample_runtime::answer_until_process_finishes(
        &mut server,
        &mut run,
        response_for_clear_narrowing_drop,
    )
    .await;
    let report = report_from_process_output(&output);
    assert_eq!(report["candidates"].as_array().unwrap().len(), 10);
    assert_eq!(server.finish().await, 10);

    let samples = report["dropped_samples"].as_array().unwrap();
    assert_eq!(
        samples.len(),
        8,
        "two passes times two inputs times cap two"
    );
    for input_report in report["inputs"].as_array().unwrap() {
        let input_id = input_report["input_id"].as_str().unwrap();
        let input_samples = samples
            .iter()
            .filter(|sample| {
                sample["occurrence"]["record"]
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("{input_id}:"))
            })
            .collect::<Vec<_>>();
        let mechanical_lines = input_samples
            .iter()
            .filter(|sample| sample["pass"] == "mechanical")
            .map(|sample| narrowing_sample_runtime::sample_line(sample))
            .collect::<Vec<_>>();
        let narrowing_lines = input_samples
            .iter()
            .filter(|sample| sample["pass"] == "narrowing")
            .map(|sample| narrowing_sample_runtime::sample_line(sample))
            .collect::<Vec<_>>();
        assert_eq!(mechanical_lines, vec![1, 3]);
        assert_eq!(narrowing_lines, vec![5, 7]);
    }
    for candidate in report["candidates"].as_array().unwrap() {
        assert_eq!(candidate["outcome"]["kind"], "dropped");
        assert_eq!(candidate["outcome"]["at"]["kind"], "narrowing");
        let answer = narrowing_sample_runtime::narrowing_answer(candidate);
        assert_eq!(answer["answer"]["yes_probability"], 0.1);
        assert_eq!(candidate["outcome"]["answer"], answer["answer_id"]);
    }

    let zero_root = tempfile::tempdir().unwrap();
    let mut zero_server = ControlledServer::start().await;
    let zero_input = narrowing_sample_fixture::record_run(
        zero_root.path(),
        &zero_server.endpoint,
        &[vec![
            "MATCH ZERO_DROP_0".into(),
            "MATCH ZERO_DROP_1".into(),
            "MATCH ZERO_DROP_2".into(),
        ]],
        0,
    );
    let mut zero_run =
        narrowing_sample_runtime::start_candidate_process(zero_root.path(), zero_input).await;
    let zero_output = narrowing_sample_runtime::answer_until_process_finishes(
        &mut zero_server,
        &mut zero_run,
        response_for_clear_narrowing_drop,
    )
    .await;
    let zero_report = report_from_process_output(&zero_output);
    assert!(
        zero_report["dropped_samples"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(zero_report["candidates"].as_array().unwrap().len(), 3);
    for candidate in zero_report["candidates"].as_array().unwrap() {
        assert_eq!(candidate["outcome"]["kind"], "dropped");
        assert_eq!(candidate["outcome"]["at"]["kind"], "narrowing");
        assert_eq!(
            narrowing_sample_runtime::narrowing_answer(candidate)["answer"]["yes_probability"],
            0.1
        );
    }
    assert_eq!(zero_server.finish().await, 3);
}

#[tokio::test]
async fn carried_narrowing_answer_discarded_by_an_ancestor_gate_is_not_sampled() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = narrowing_sample_fixture::record_run(
        root.path(),
        &server.endpoint,
        &[vec![
            "MATCH SET_GATE_DROP".into(),
            "MATCH CLEAR_NARROWING_DROP".into(),
        ]],
        20,
    );

    let rule_path = root.path().join("rules.jsonl");
    let rule: Value = serde_json::from_slice(&std::fs::read(&rule_path).unwrap()).unwrap();
    let evidence = rule["evidence"].clone();
    let set_path = root.path().join("set.json");
    let mut set: Value = serde_json::from_slice(&std::fs::read(&set_path).unwrap()).unwrap();
    set["applicability"] = json!([{
        "kind":"decision",
        "question":{"type":"noul","instructions":"SET_GATE_CARRY"},
        "evidence":evidence,
        "threshold":0.8
    }]);
    std::fs::write(set_path, set.to_string()).unwrap();
    input["execution"]["speculation"] = json!("on");

    let mut run = narrowing_sample_runtime::start_candidate_process(root.path(), input).await;
    let mut received = BTreeSet::new();
    let output =
        narrowing_sample_runtime::answer_until_process_finishes(&mut server, &mut run, |body| {
            let serialized = body.to_string();
            let expected = ["SET_GATE_DROP", "CLEAR_NARROWING_DROP"]
                .into_iter()
                .find(|marker| serialized.contains(marker))
                .unwrap_or_else(|| panic!("missing carried-gate candidate: {serialized}"));
            assert_eq!(
                ["SET_GATE_DROP", "CLEAR_NARROWING_DROP"]
                    .into_iter()
                    .filter(|marker| serialized.contains(marker))
                    .count(),
                1,
                "one candidate per request: {serialized}"
            );
            assert!(
                serialized.contains("SET_GATE_CARRY"),
                "ancestor gate must be carried"
            );
            assert!(
                serialized.contains("NARROW_SAMPLE"),
                "narrowing answer must be carried"
            );
            received.insert(expected);
            noul_response(body, |question| {
                let instructions = question.to_string();
                if instructions.contains("SET_GATE_CARRY") {
                    if expected == "SET_GATE_DROP" {
                        0.1
                    } else {
                        0.9
                    }
                } else if instructions.contains("NARROW_SAMPLE") {
                    0.1
                } else {
                    0.9
                }
            })
        })
        .await;
    assert_eq!(
        received,
        BTreeSet::from(["SET_GATE_DROP", "CLEAR_NARROWING_DROP"])
    );
    let report = report_from_process_output(&output);
    assert_eq!(server.finish().await, 2);

    let candidates = report["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2);
    let ancestor_drop = candidates
        .iter()
        .find(|candidate| candidate["occurrence"]["positions"][0]["line"] == 1)
        .unwrap();
    assert_eq!(ancestor_drop["outcome"]["kind"], "dropped");
    assert_eq!(ancestor_drop["outcome"]["at"]["kind"], "rule_set_gate");
    assert!(
        ancestor_drop["answers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|answer| answer["question"]["kind"] != "narrowing"),
        "the in-flight child answer was discarded by the failed parent gate"
    );

    let narrowing_drop = candidates
        .iter()
        .find(|candidate| candidate["occurrence"]["positions"][0]["line"] == 2)
        .unwrap();
    assert_eq!(narrowing_drop["outcome"]["kind"], "dropped");
    assert_eq!(narrowing_drop["outcome"]["at"]["kind"], "narrowing");
    let answer = narrowing_sample_runtime::narrowing_answer(narrowing_drop);
    assert_eq!(answer["answer"]["yes_probability"], 0.1);
    assert_eq!(narrowing_drop["outcome"]["answer"], answer["answer_id"]);

    let samples = report["dropped_samples"].as_array().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(narrowing_sample_runtime::sample_line(&samples[0]), 2);
    assert_eq!(samples[0]["pass"], "narrowing");
}

#[path = "support/narrowing_sample_source_tests.rs"]
mod narrowing_sample_source_tests;

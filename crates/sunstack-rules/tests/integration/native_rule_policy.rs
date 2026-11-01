#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
    time::Duration,
};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

fn off_policy(root: &Path, rule: &str) -> RulePolicy {
    RulePolicy {
        off: BTreeMap::from([(
            GroupKey {
                rule_set: root.join("set.json"),
                group: 0,
            },
            BTreeSet::from([RuleId::new(rule).unwrap()]),
        )]),
    }
}

async fn run_observed(
    input: RuleInvocationInput,
    policy: RulePolicy,
    mut server: controlled::ControlledServer,
) -> (PolicyEvaluation, Vec<Value>) {
    let context = RuleExecutionContext {
        backend: Arc::new(transport::backend(&server.endpoint)),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    };
    let mut execution = tokio::spawn(evaluate_rules_with_policy(input, policy, context));
    let mut bodies = Vec::new();
    let evaluation = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            tokio::select! {
                result = &mut execution => break result.unwrap(),
                request = server.receive() => {
                    bodies.push(request.body.clone());
                    request.noul(0.9);
                }
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(server.finish().await, bodies.len());
    (evaluation, bodies)
}

#[tokio::test]
async fn off_rules_cost_no_request_or_gate_and_keep_coverage_full_in_both_expansion_paths() {
    for deferred in [false, true] {
        for gated in [false, true] {
            for peer in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let mut rules = vec![fixture::rule("disabled")];
                if peer {
                    rules.push(fixture::rule("enabled"));
                }
                if deferred {
                    for rule in &mut rules {
                        rule["evidence"][0]["select"] =
                            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]});
                    }
                }
                fixture::write_set(
                    root.path(),
                    &rules,
                    if gated {
                        vec![fixture::decision("subject")]
                    } else {
                        vec![]
                    },
                );
                let server = controlled::ControlledServer::start().await;
                let input = fixture::input(root.path(), &server.endpoint);
                let (evaluation, bodies) =
                    run_observed(input, off_policy(root.path(), "disabled"), server).await;
                assert_eq!(
                    bodies.len(),
                    usize::from(peer),
                    "deferred={deferred} gate={gated} peer={peer}"
                );
                let RuleInvocationOutcome::CompleteReport { report } = evaluation.outcome else {
                    panic!("{:?}", evaluation.outcome)
                };
                assert!(matches!(report.rule_coverage, RuleCoverage::Full));
                assert!(
                    report
                        .occurrences
                        .iter()
                        .all(|occurrence| occurrence.key().rule_id.as_str() == "enabled")
                );
                assert_eq!(report.occurrences.len(), usize::from(peer));
                assert_eq!(report.groups[0].entries.len(), usize::from(gated && peer));
                for body in bodies {
                    let questions = body["questions"].as_object().unwrap();
                    assert_eq!(questions.len(), 1 + usize::from(gated));
                    assert!(!serde_json::to_string(&body).unwrap().contains("disabled"));
                }
            }
        }
    }
}

#[tokio::test]
async fn off_policy_applies_only_to_its_binding_group() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("one")], vec![]);
    let server = controlled::ControlledServer::start().await;
    let mut input = fixture::input(root.path(), &server.endpoint);
    let InvocationDefinitions::RuleSets { rule_sets } = &mut input.definitions else {
        panic!("rule sets")
    };
    let peer = rule_sets[0].binding_groups[0].clone();
    rule_sets[0].binding_groups.push(peer);
    let (evaluation, bodies) = run_observed(input, off_policy(root.path(), "one"), server).await;
    assert_eq!(bodies.len(), 1);
    let RuleInvocationOutcome::CompleteReport { report } = evaluation.outcome else {
        panic!("{:?}", evaluation.outcome)
    };
    assert_eq!(report.occurrences.len(), 1);
    assert_eq!(report.occurrences[0].key().group, 1);
    assert!(matches!(report.rule_coverage, RuleCoverage::Full));
}

#[tokio::test]
async fn unknown_off_rule_is_invalid_with_group_and_rule_set_and_no_dispatch() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("enabled")], vec![]);
    let server = controlled::ControlledServer::start().await;
    let input = fixture::input(root.path(), &server.endpoint);
    let (evaluation, bodies) =
        run_observed(input, off_policy(root.path(), "unknown"), server).await;
    assert!(bodies.is_empty());
    let RuleInvocationOutcome::Invalid { diagnostics } = evaluation.outcome else {
        panic!("{:?}", evaluation.outcome)
    };
    assert!(diagnostics.items.iter().any(|item| matches!(item, ReportDiagnostic::Rule(diagnostic)
        if matches!(&diagnostic.location, RuleDiagnosticLocation::Definition { file, field_path, .. }
            if file == &root.path().canonicalize().unwrap().join("set.json") && field_path == &["binding_groups", "0"]))));
}

#[tokio::test]
async fn request_evidence_equals_final_sent_state_at_public_indices_after_budget_splitting() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(
        root.path(),
        &[
            fixture::rule("one"),
            fixture::rule("two"),
            fixture::rule("three"),
        ],
        vec![],
    );
    let server = controlled::ControlledServer::start().await;
    let mut input = fixture::input(root.path(), &server.endpoint);
    input.request_budget.max_questions = 1;
    let (evaluation, bodies) = run_observed(input, RulePolicy::default(), server).await;
    let RuleInvocationOutcome::CompleteReport { report } = evaluation.outcome else {
        panic!("{:?}", evaluation.outcome)
    };
    assert_eq!(report.requests.len(), 3);
    assert_eq!(evaluation.request_evidence.len(), 3);
    for request in &report.requests {
        let QuestionOwner::Verdict { key } = &request.questions[0].owner else {
            panic!("wrong owner")
        };
        let owner = format!("rule:{}", key.rule_id.as_str());
        let body = bodies.iter().find(|body| body["questions"][request.questions[0].request_name.as_str()]["instructions"]["owner"] == owner).unwrap();
        assert_eq!(
            serde_json::to_value(&evaluation.request_evidence[&request.index]).unwrap(),
            body["state"]
        );
    }
}

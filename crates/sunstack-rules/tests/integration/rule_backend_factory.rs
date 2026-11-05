#![allow(clippy::unwrap_used, clippy::panic)]
use sunstack_rules::test_support::controlled_http as controlled;
use sunstack_rules::test_support::rule_fixture as fixture;
#[path = "../support/decision_http.rs"]
mod transport;
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use sunstack_decisions::{BackendSelection, CancellationToken, DecisionBackend};
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;

enum FactoryBehavior {
    Ready,
    Refused,
}
struct CountedFactory {
    selection: BackendSelection,
    calls: Arc<AtomicUsize>,
    behavior: FactoryBehavior,
}
impl RuleBackendFactory for CountedFactory {
    fn selection(&self) -> &BackendSelection {
        &self.selection
    }
    fn prepare(self: Arc<Self>) -> Result<Arc<dyn DecisionBackend>, DiagnosticList> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.behavior {
            FactoryBehavior::Refused => Err(DiagnosticList::rule(
                RuleDiagnosticCode::InvalidConfiguration,
            )),
            FactoryBehavior::Ready => {
                let BackendSelection::JevOpenrouter { endpoint, .. } = &self.selection;
                Ok(Arc::new(transport::backend(endpoint)))
            }
        }
    }
}
fn local_rule() -> serde_json::Value {
    let mut rule = fixture::rule("local.one");
    rule["evidence"][0]["select"] =
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]});
    rule["verdict"] = json!({"kind":"mechanical","explanation":"Use a clear prefix","check":{"kind":"expect","expect":{"starts_with":"U001:"}}});
    rule
}
fn context(
    input: &RuleInvocationInput,
    behavior: FactoryBehavior,
    calls: &Arc<AtomicUsize>,
) -> RuleExecutionContext {
    RuleExecutionContext {
        backend: Arc::new(CountedFactory {
            selection: input.backend.clone(),
            calls: calls.clone(),
            behavior,
        }),
        adapters: AdapterRegistry::builtins().unwrap(),
        cancellation: CancellationToken::new(),
    }
}

#[tokio::test]
async fn local_only_preparation_and_evaluation_never_call_a_factory_that_would_fail() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[local_rule()], vec![]);
    std::fs::write(
        root.path().join("source.md"),
        "U001: one\n\nU001: two\n\nU001: three\n",
    )
    .unwrap();
    let mut input = fixture::input(root.path(), "http://127.0.0.1:9/v1/systemone");
    input.request_budget.max_questions = 1;
    input.request_budget.max_requests = 1;
    let calls = Arc::new(AtomicUsize::new(0));
    let outcome = prepare_rules(
        input.clone(),
        context(&input, FactoryBehavior::Refused, &calls),
    )
    .await;
    assert_eq!(outcome.exit_code(), 0, "{outcome:?}");
    let outcome = evaluate_rules(
        input.clone(),
        context(&input, FactoryBehavior::Refused, &calls),
    )
    .await;
    assert_eq!(outcome.exit_code(), 0, "{outcome:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.occurrences.len(), 3);
    assert_eq!(report.measurements.model_requests, 0);
    assert!(report.occurrences.iter().all(|result| matches!(
        result,
        RuleOccurrenceResult::Checked {
            outcome: LocalSettlement::Completed {
                result: MechanicalResult::Pass,
                ..
            },
            ..
        }
    )));
}

#[tokio::test]
async fn mixed_local_and_split_judgment_work_builds_one_real_backend() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(
        root.path(),
        &[
            local_rule(),
            fixture::rule("model.one"),
            fixture::rule("model.two"),
        ],
        vec![],
    );
    let mut server = controlled::ControlledServer::start().await;
    let mut input = fixture::input(root.path(), &server.endpoint);
    input.request_budget.max_questions = 1;
    let calls = Arc::new(AtomicUsize::new(0));
    let mut execution = tokio::spawn(evaluate_rules(
        input.clone(),
        context(&input, FactoryBehavior::Ready, &calls),
    ));
    let outcome=tokio::time::timeout(Duration::from_secs(3),async {
        loop {tokio::select! { result=&mut execution=>break result.unwrap(), request=server.receive()=>{assert_eq!(request.body["questions"].as_object().unwrap().len(),1);request.noul(0.95);} }}
    }).await.unwrap();
    assert_eq!(server.finish().await, 2);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(report.occurrences.len(), 3);
    assert_eq!(
        report
            .occurrences
            .iter()
            .filter(|result| matches!(result, RuleOccurrenceResult::Checked { .. }))
            .count(),
        1
    );
    assert_eq!(report.measurements.model_requests, 2);
}

#[tokio::test]
async fn reachable_model_work_keeps_factory_configuration_refusal_as_invalid_exit_two() {
    let root = tempfile::tempdir().unwrap();
    fixture::write_set(root.path(), &[fixture::rule("model.one")], vec![]);
    let input = fixture::input(root.path(), "http://127.0.0.1:9/v1/systemone");
    let calls = Arc::new(AtomicUsize::new(0));
    let outcome = evaluate_rules(
        input.clone(),
        context(&input, FactoryBehavior::Refused, &calls),
    )
    .await;
    assert!(matches!(outcome, RuleInvocationOutcome::Invalid { .. }));
    assert_eq!(outcome.exit_code(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    calls.store(0, Ordering::SeqCst);
    let outcome = prepare_rules(
        input.clone(),
        context(&input, FactoryBehavior::Refused, &calls),
    )
    .await;
    assert!(matches!(outcome, RulePreparationOutcome::Invalid { .. }));
    assert_eq!(outcome.exit_code(), 2);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_stopped_judgment_beside_a_completed_local_check_leaves_no_model_questions_to_budget() {
    let root = tempfile::tempdir().unwrap();
    let mut judged = fixture::rule("model.one");
    judged["applicability"] = json!([fixture::decision("other")]);
    fixture::write_set(root.path(), &[local_rule(), judged], vec![]);
    let mut server = controlled::ControlledServer::start().await;
    let mut input = fixture::input(root.path(), &server.endpoint);
    input.request_budget.max_requests = 1;
    let calls = Arc::new(AtomicUsize::new(0));
    let execution = tokio::spawn(evaluate_rules(
        input.clone(),
        context(&input, FactoryBehavior::Ready, &calls),
    ));
    server.next().await.noul(0.1);
    let outcome = tokio::time::timeout(Duration::from_secs(3), execution)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(server.finish().await, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    assert!(
        report
            .occurrences
            .iter()
            .any(|result| matches!(result, RuleOccurrenceResult::Checked { .. }))
    );
    assert!(
        report
            .occurrences
            .iter()
            .any(|result| matches!(result, RuleOccurrenceResult::NotApplicable { .. }))
    );
    let unsent = report
        .requests
        .iter()
        .find(|request| matches!(request.status, RequestStatus::NotSent { .. }))
        .unwrap();
    assert!(
        matches!(
            unsent.status,
            RequestStatus::NotSent {
                reason: NotSentReason::NoQuestions
            }
        ),
        "{:?}",
        unsent.status
    );
}

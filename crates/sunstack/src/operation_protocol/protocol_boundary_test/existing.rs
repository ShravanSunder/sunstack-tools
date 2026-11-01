use super::super::*;
use serde_json::json;
use sunstack_rules::{InvocationMeasurements, RuleDiagnosticCode};

#[test]
fn internal_check_failure_keeps_native_diagnostics_measurements_and_exit_three()
-> serde_json::Result<()> {
    // The owned blocking task failure has no public-input fixture; this is boundary proof only.
    let outcome = CheckRulesOutcome::CheckFailed {
        diagnostics: sunstack_rules::DiagnosticList::rule(
            RuleDiagnosticCode::UnexpectedExecutionError,
        ),
        measurements: InvocationMeasurements {
            files_read: 7,
            ..InvocationMeasurements::default()
        },
    };
    let result = OperationResult::CheckRules(Box::new(outcome));
    assert_eq!(result.exit_code(), 3);
    let wire = serde_json::to_value(ProgramResponse::operation_result(result))?;
    assert_eq!(wire["result"]["operation"], "check_rules");
    assert_eq!(wire["result"]["outcome"]["kind"], "check_failed");
    assert_eq!(wire["result"]["outcome"]["measurements"]["files_read"], 7);
    assert_eq!(
        wire["result"]["outcome"]["diagnostics"]["items"][0]["code"],
        "unexpected_execution_error"
    );
    Ok(())
}

#[test]
fn internal_preparation_failures_keep_native_diagnostics_measurements_and_exit_three() {
    // These native invariant/fatal outcomes have no public-input fixture.
    // This checks the serialization boundary, not process reachability.
    let diagnostics =
        sunstack_rules::DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError);
    let measurements = InvocationMeasurements {
        files_read: 7,
        ..InvocationMeasurements::default()
    };
    for (operation, result) in [
        (
            "prepare_rules",
            OperationResult::PrepareRules(Box::new(RulePreparationOutcome::PreparationFailed {
                diagnostics: diagnostics.clone(),
                measurements: measurements.clone(),
            })),
        ),
        (
            "evaluate_rules",
            OperationResult::EvaluateRules(Box::new(RuleInvocationOutcome::PreparationFailed {
                diagnostics: diagnostics.clone(),
                measurements: measurements.clone(),
            })),
        ),
    ] {
        assert_eq!(result.exit_code(), 3);
        let wire = serde_json::to_value(ProgramResponse::operation_result(result));
        assert!(wire.is_ok());
        let expected = json!({
            "kind":"operation_result", "protocol_version":1,
            "result":{"operation":operation,"outcome":{
                "kind":"preparation_failed", "diagnostics":diagnostics,
                "measurements":measurements
            }}
        });
        assert_eq!(wire.ok(), Some(expected));
    }
}

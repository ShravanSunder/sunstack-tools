#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/journey_fixtures.rs"]
mod journey_fixtures;
mod support;
use journey_fixtures::*;
use serde_json::{Value, json};
use support::*;
const ENDPOINT: &str = "https://openrouter.ai/api/v1/systemone";

async fn observe_hosted_pair(journey: &str, input: Value, is_handoff: bool) {
    let frozen = fingerprints(&input);
    record_observation(
        &format!("hosted-{journey}-frozen"),
        json!({"fixture_fingerprints":frozen,"input":input,"invocations":2,"attempts_per_request":1,"retries":0,"provider_and_os_cache":"uncontrolled","process_deadline_seconds":420}),
    );
    let mut rows = Vec::new();
    for condition in ["cold_session_first", "warm_path_repeat"] {
        let (output, elapsed) =
            run_hosted_timed(&workspace_root(), "evaluate_rules", input.clone()).await;
        record_observation(
            &format!("hosted-{journey}-{condition}"),
            json!({"wall_ms":elapsed.as_secs_f64()*1000.0,"exit":output.status.code(),"response":serde_json::from_slice::<Value>(&output.stdout).unwrap()}),
        );
        let wire = response(&output, 0);
        let report = complete_report(&wire);
        // A legitimate uncertain/non-route result stays in the raw output, but
        // cannot close this routed V29 journey. Never retry until favorable.
        check_journey(report, is_handoff);
        let receipts = receipt_summary(report);
        assert!(!receipts.is_empty());
        assert_eq!(fingerprints(&input), frozen);
        rows.push(json!({"condition":condition,"wall_ms":elapsed.as_secs_f64()*1000.0,"measurements":report["measurements"],"receipts":receipts,"report":report}));
    }
    record_observation(
        &format!("hosted-{journey}-pair"),
        json!({"rows":rows,"claim":"genuine process journey; semantic accuracy and cold storage/model state unmeasured"}),
    );
    println!("hosted {journey} measurements: {}",serde_json::to_string(&rows.iter().map(|row|json!({"condition":row["condition"],"wall_ms":row["wall_ms"],"measurements":row["measurements"],"receipts":row["receipts"]})).collect::<Vec<_>>()).unwrap());
}

#[tokio::test]
#[ignore = "genuine Jev V30 and fixed first/repeat measurements require one-command credential"]
async fn hosted_skill_lint_journey_through_real_binary() {
    observe_hosted_pair("skill", skill_input(ENDPOINT), false).await;
}
#[tokio::test]
#[ignore = "genuine Jev V29 and fixed first/repeat measurements require one-command credential"]
async fn hosted_handoff_monitor_journey_through_real_binary() {
    let fixture = HandoffFixture::new(ENDPOINT);
    observe_hosted_pair("handoff", fixture.input.clone(), true).await;
}

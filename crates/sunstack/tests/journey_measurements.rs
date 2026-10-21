#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/journey_fixtures.rs"]
mod journey_fixtures;
#[path = "support/rule_transport.rs"]
mod rule_transport;
mod support;
use journey_fixtures::*;
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use serde_json::{Value, json};
use support::*;

fn distribution(values: &[f64]) -> Value {
    assert_eq!(values.len(), 5);
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mean = values.iter().sum::<f64>() / 5.0;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / 4.0;
    json!({"count":5,"min":sorted[0],"median":sorted[2],"max":sorted[4],"sample_stddev":variance.sqrt()})
}

#[tokio::test]
#[ignore = "fixed V14 workload: five first/repeat pairs per journey and budget variant"]
async fn fixed_journey_measurements_preserve_scope_and_record_process_local_and_provider_work() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let handoff = HandoffFixture::new(&server.endpoint);
    let mut rows = Vec::<Value>::new();
    for (journey, input, is_handoff, split_limit) in [
        ("skill", skill_input(&server.endpoint), false, 2),
        ("handoff", handoff.input.clone(), true, 4),
    ] {
        let frozen = fingerprints(&input);
        record_observation(
            &format!("v14-frozen-{journey}"),
            json!({"fixture_fingerprints":frozen,"input":input,"pairs_per_variant":5,"question_limits":[32,split_limit],"conditions":["cold_session_first","warm_path_repeat"],"both_conditions_spawn_new_process_and_session":true,"os_cache":"uncontrolled; fingerprinting and preflight can warm files","wall_interval":"before process spawn through child/stdout/stderr join; excludes JSON encoding and artifact writes"}),
        );
        let mut expected_scope = None;
        for limit in [32, split_limit] {
            let mut variant = input.clone();
            variant["request_budget"]["max_questions"] = json!(limit);
            for pair in 0..5 {
                for condition in ["cold_session_first", "warm_path_repeat"] {
                    let before = server.count();
                    let (output, elapsed) =
                        run_timed(&workspace_root(), "evaluate_rules", variant.clone()).await;
                    let wire = response(&output, 0);
                    let report = complete_report(&wire);
                    check_journey(report, is_handoff);
                    let scope = coverage(report);
                    if let Some(expected) = &expected_scope {
                        assert_eq!(
                            &scope, expected,
                            "measurement changed selected owner/question/target coverage or judgments"
                        );
                    } else {
                        expected_scope = Some(scope);
                    }
                    let actual_requests = server.count() - before;
                    assert_eq!(report["measurements"]["model_requests"], actual_requests);
                    assert_eq!(
                        fingerprints(&input),
                        frozen,
                        "measured fixture bytes changed"
                    );
                    rows.push(json!({"journey":journey,"question_limit":limit,"pair":pair,"condition":condition,"wall_ms":elapsed.as_secs_f64()*1000.0,"native":report["measurements"],"http_requests":actual_requests,"receipts":receipt_summary(report)}));
                }
            }
        }
    }
    let mut summaries = Vec::new();
    for (journey, split_limit) in [("skill", 2), ("handoff", 4)] {
        for limit in [32, split_limit] {
            for condition in ["cold_session_first", "warm_path_repeat"] {
                let selected: Vec<&Value> = rows
                    .iter()
                    .filter(|row| {
                        row["journey"] == journey
                            && row["question_limit"] == limit
                            && row["condition"] == condition
                    })
                    .collect();
                summaries.push(json!({"journey":journey,"question_limit":limit,"condition":condition,"wall_ms":distribution(&selected.iter().map(|row|row["wall_ms"].as_f64().unwrap()).collect::<Vec<_>>()),"local_ms":distribution(&selected.iter().map(|row|row["native"]["local_elapsed_ms"].as_f64().unwrap()).collect::<Vec<_>>()),"provider_ms":distribution(&selected.iter().map(|row|row["native"]["provider_elapsed_ms"].as_f64().unwrap()).collect::<Vec<_>>()),"model_requests":selected.iter().map(|row|row["native"]["model_requests"].clone()).collect::<Vec<_>>()}));
            }
        }
    }
    let (requests, bodies) = server.finish().await;
    assert_eq!(rows.len(), 40);
    assert_eq!(requests, 70);
    record_observation(
        "v14-controlled",
        json!({"rows":rows,"summaries":summaries,"total_http_requests":requests,"parsed_http_bodies":bodies,"claim":"descriptive measurements only; no SLA or inferred cold storage/model cache"}),
    );
    println!(
        "V14 controlled summaries: {}",
        serde_json::to_string(&summaries).unwrap()
    );
}

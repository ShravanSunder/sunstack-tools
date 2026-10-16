#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/journey_fixtures.rs"]
mod journey_fixtures;
#[path = "support/rule_transport.rs"]
mod rule_transport;
mod support;
use journey_fixtures::*;
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use serde_json::json;
use support::*;

#[tokio::test]
async fn reviewed_skill_and_handoff_journeys_prepare_without_dispatch_then_judge_all_targets() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let handoff = HandoffFixture::new(&server.endpoint);
    for (input, is_handoff) in [
        (skill_input(&server.endpoint), false),
        (handoff.input.clone(), true),
    ] {
        record_observation(
            if is_handoff {
                "handoff-fixture-before-run"
            } else {
                "skill-fixture-before-run"
            },
            fingerprints(&input),
        );
        let before = server.count();
        let prepared = response(
            &run(&workspace_root(), "prepare_rules", input.clone()).await,
            0,
        );
        assert_eq!(server.count(), before);
        let preparation = &prepared["result"]["outcome"]["preparation"];
        assert_eq!(preparation["measurements"]["model_requests"], 0);
        if is_handoff {
            assert_eq!(preparation["routing"]["status"], "not_evaluated");
            assert_eq!(
                preparation["routing"]["candidate_rule_sets"]
                    .as_array()
                    .unwrap()
                    .len(),
                2
            );
        }
        let output = response(&run(&workspace_root(), "evaluate_rules", input).await, 0);
        let report = complete_report(&output);
        check_journey(report, is_handoff);
        receipt_summary(report);
        record_observation(
            if is_handoff {
                "controlled-handoff-report"
            } else {
                "controlled-skill-report"
            },
            output,
        );
    }
    let (count, bodies) = server.finish().await;
    record_observation(
        "controlled-journey-http",
        json!({"requests":count,"parsed_bodies":bodies}),
    );
    assert_eq!(count, 2);
}

#[tokio::test]
async fn real_router_uncertainty_and_no_route_are_complete_without_fabricated_occurrences() {
    for (router, expected) in [
        (RouterReply::Uncertain, "uncertain_intent"),
        (RouterReply::NoRoute, "no_route"),
    ] {
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.9,
            router,
        })
        .await;
        let fixture = HandoffFixture::new(&server.endpoint);
        let wire = response(
            &run(&workspace_root(), "evaluate_rules", fixture.input.clone()).await,
            0,
        );
        let report = complete_report(&wire);
        assert_eq!(report["routing"]["groups"][0]["outcome"]["kind"], expected);
        assert!(report["occurrences"].as_array().unwrap().is_empty());
        assert_eq!(server.finish().await.0, 1);
    }
}

#[tokio::test]
async fn real_router_provider_failure_stays_incomplete_without_rule_verdicts() {
    let server = AnswerServer::start(ReplyMode::ProviderFailure { status: 401 }).await;
    let fixture = HandoffFixture::new(&server.endpoint);
    let wire = response(
        &run(&workspace_root(), "evaluate_rules", fixture.input.clone()).await,
        3,
    );
    assert_eq!(wire["result"]["outcome"]["kind"], "incomplete_report");
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(
        report["routing"]["groups"][0]["outcome"]["kind"],
        "router_failed"
    );
    let occurrences = report["occurrences"].as_array().unwrap();
    assert_eq!(occurrences.len(), 4);
    assert!(occurrences.iter().all(|occurrence| {
        occurrence["kind"] == "evaluation_failed" && occurrence.get("judgment").is_none()
    }));
    assert_eq!(server.finish().await.0, 1);
}

#[tokio::test]
async fn declared_question_budget_comparators_preserve_all_journey_owners_before_measurement() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let handoff = HandoffFixture::new(&server.endpoint);
    for (input, is_handoff, split_limit) in [
        (skill_input(&server.endpoint), false, 2),
        (handoff.input.clone(), true, 4),
    ] {
        let mut baseline = None;
        for limit in [32, split_limit] {
            let mut variant = input.clone();
            variant["request_budget"]["max_questions"] = json!(limit);
            let prepared = response(
                &run(&workspace_root(), "prepare_rules", variant.clone()).await,
                0,
            );
            assert_eq!(prepared["result"]["outcome"]["kind"], "prepared");
            let evaluated = response(&run(&workspace_root(), "evaluate_rules", variant).await, 0);
            let report = complete_report(&evaluated);
            check_journey(report, is_handoff);
            let observed = coverage(report);
            if let Some(expected) = &baseline {
                assert_eq!(
                    &observed, expected,
                    "question budget changed selected scope"
                );
            } else {
                baseline = Some(observed);
            }
            record_observation(
                &format!(
                    "comparator-preflight-{}-{limit}",
                    if is_handoff { "handoff" } else { "skill" }
                ),
                json!({"measurements":report["measurements"],"coverage":coverage(report)}),
            );
        }
    }
    assert_eq!(server.finish().await.0, 7);
}

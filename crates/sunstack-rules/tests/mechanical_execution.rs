#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/rule_fixture.rs"]
mod fixture;
#[path = "support/decision_http.rs"]
mod transport;
use serde_json::{Value, json};
use std::sync::Arc;
use sunstack_decisions::CancellationToken;
use sunstack_evidence::AdapterRegistry;
use sunstack_rules::*;
#[path = "support/contract_schemas.rs"]
mod schema_support;

fn mechanical_rule(id: &str, select: Value, check: Value) -> Value {
    let mut rule = fixture::rule(id);
    rule["evidence"][0]["select"] = select;
    rule["verdict"] = json!({"kind":"mechanical","explanation":"A local standard","check":check});
    rule
}
async fn evaluate(root: &std::path::Path) -> Value {
    let endpoint = "http://127.0.0.1:9/v1/systemone";
    let outcome = evaluate_rules(
        fixture::input(root, endpoint),
        RuleExecutionContext {
            backend: Arc::new(transport::backend(endpoint)),
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    serde_json::to_value(outcome).unwrap()
}

#[tokio::test]
async fn each_expect_form_checks_good_and_bad_units_without_any_model_request() {
    for (expect, text, passed) in [
        (
            json!({"matches":"MUST"}),
            "You MUST load the standard.\n",
            true,
        ),
        (
            json!({"matches":"MUST"}),
            "You may load the standard.\n",
            false,
        ),
        (
            json!({"not_matches":"forbidden"}),
            "Use the standard.\n",
            true,
        ),
        (
            json!({"not_matches":"forbidden"}),
            "Use the forbidden standard.\n",
            false,
        ),
        (json!({"starts_with":"Use"}), "Use the standard.\n", true),
        (json!({"starts_with":"Use"}), "Avoid the standard.\n", false),
        (
            json!({"matches":"MUST"}),
            "Use `MUST` only as an example.\n",
            false,
        ),
        (json!({"matches":"MUST"}), "```text\nMUST\n```\n", true),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let fenced = text.starts_with("```");
        let rule = mechanical_rule(
            "local.one",
            json!({"kind":"markdown_units","mode":"unit","units":if fenced {vec!["section"]} else {vec!["paragraph"]}}),
            json!({"kind":"expect","expect":expect}),
        );
        fixture::write_set(directory.path(), &[rule], vec![]);
        std::fs::write(
            directory.path().join("source.md"),
            if fenced {
                format!("## Standard\n\n{text}")
            } else {
                text.to_owned()
            },
        )
        .unwrap();
        let wire = evaluate(directory.path()).await;
        assert_eq!(wire["kind"], "complete_report", "{wire}");
        let report = &wire["report"];
        assert_eq!(report["measurements"]["model_requests"], 0);
        assert_eq!(report["measurements"]["inference_attempts"], 0);
        assert_eq!(report["occurrences"].as_array().unwrap().len(), 1);
        let result = &report["occurrences"][0];
        assert_eq!(result["kind"], "checked");
        assert_eq!(result["outcome"]["kind"], "completed");
        assert_eq!(
            result["outcome"]["result"],
            if passed { "pass" } else { "fail" }
        );
        for field in ["judgment", "answers", "confidence", "receipt"] {
            assert!(result.get(field).is_none(), "{result}");
        }
        assert!(
            report["requests"]
                .as_array()
                .unwrap()
                .iter()
                .all(|request| request["questions"].as_array().unwrap().is_empty())
        );
    }
}

#[tokio::test]
async fn frontmatter_present_absent_and_unavailable_remain_distinct_and_keep_a_completed_peer() {
    let reviewed = schema_support::reviewed("rule-invocation-report.schema.json");
    let generated = schema_support::generated::<RuleInvocationReport>();
    for (text, expect, completed, passed) in [
        (
            "---\ndescription: Ready\n---\n",
            json!({"matches":"^Ready$"}),
            true,
            true,
        ),
        (
            "---\ndescription: 12\n---\n",
            json!({"starts_with":"12"}),
            true,
            true,
        ),
        (
            "---\ndescription: false\n---\n",
            json!({"matches":"^false$"}),
            true,
            true,
        ),
        (
            "---\ndescription: null\n---\n",
            json!({"present":true}),
            true,
            true,
        ),
        (
            "---\ndescription: null\n---\n",
            json!({"matches":"^null$"}),
            true,
            true,
        ),
        (
            "---\ndescription: [a, b]\n---\n",
            json!({"matches":"^\\[\\\"a\\\",\\\"b\\\"\\]$"}),
            true,
            true,
        ),
        (
            "---\ndescription: {z: 1, a: 2}\n---\n",
            json!({"starts_with":"{\"a\":2,\"z\":1}"}),
            true,
            true,
        ),
        (
            "---\nname: other\n---\n",
            json!({"present":true}),
            true,
            false,
        ),
        (
            "---\nname: other\n---\n",
            json!({"not_matches":"bad"}),
            true,
            false,
        ),
        (
            "---\nname: other\n---\n",
            json!({"matches":".*"}),
            true,
            false,
        ),
        (
            "---\nname: other\n---\n",
            json!({"starts_with":""}),
            true,
            false,
        ),
        ("## No header\n", json!({"present":true}), false, false),
        (
            "---\ndescription: [\n---\n",
            json!({"present":true}),
            false,
            false,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut field = mechanical_rule(
            "local.field",
            json!({"kind":"frontmatter_field","field":"description"}),
            json!({"kind":"expect","expect":expect}),
        );
        field["severity"] = json!("error");
        let mut peer = mechanical_rule(
            "local.peer",
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            json!({"kind":"expect","expect":{"starts_with":"U001:"}}),
        );
        peer["evidence"][0]["input"] = json!("other");
        fixture::write_set(directory.path(), &[field, peer], vec![]);
        std::fs::write(directory.path().join("source.md"), text).unwrap();
        let wire = evaluate(directory.path()).await;
        assert_eq!(
            wire["kind"],
            if completed {
                "complete_report"
            } else {
                "incomplete_report"
            },
            "{text}: {wire}"
        );
        let report = &wire["report"];
        assert!(reviewed.is_valid(report), "reviewed schema: {report}");
        assert!(generated.is_valid(report), "generated schema: {report}");
        let mut fabricated = report.clone();
        fabricated["occurrences"][0]["outcome"]["confidence"] = json!(0.9);
        assert!(!reviewed.is_valid(&fabricated));
        assert!(serde_json::from_value::<RuleInvocationReport>(fabricated).is_err());
        assert_eq!(report["measurements"]["model_requests"], 0);
        let results = report["occurrences"].as_array().unwrap();
        let field = results
            .iter()
            .find(|r| r["key"]["rule_id"] == "local.field")
            .unwrap();
        assert_eq!(field["kind"], "checked");
        assert_eq!(field["declared_severity"], "error");
        assert_eq!(
            field["outcome"]["kind"],
            if completed { "completed" } else { "unresolved" }
        );
        if completed {
            assert_eq!(
                field["outcome"]["result"],
                if passed { "pass" } else { "fail" }
            );
        }
        let peer = results
            .iter()
            .find(|r| r["key"]["rule_id"] == "local.peer")
            .unwrap();
        assert_eq!(peer["outcome"]["result"], "pass");
    }
}

#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/rule_source_fixture.rs"]
mod rule_source_fixture;
#[path = "support/rule_transport.rs"]
mod rule_transport;
mod support;
use rule_source_fixture::{runtime_input, source_fixture};
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
use support::*;

const ANSWERS: &[(&str, f64)] = &[
    ("Judge alpha", 0.9),
    ("Judge beta", 0.1),
    ("Judge gamma", 0.5),
];
const MARKDOWN: &str =
    "Judge alpha<!-- K014 -->\n\nJudge beta<!-- K015 -->\n\nJudge gamma<!-- K016 -->\n";

fn write_three(root: &Path) {
    std::fs::write(root.join("teaching.md"), MARKDOWN).unwrap();
    let mut rules = serde_json::Map::new();
    for (key, name, target) in [
        ("K014", "alpha", "U001"),
        ("K015", "beta", "U002"),
        ("K016", "gamma", "U003"),
    ] {
        rules.insert(key.into(),json!({"name":name,"target":{"input":"skill","select":{"kind":"paragraph","target_id":target}},"threshold":0.8}));
    }
    std::fs::write(
        root.join("teaching.rules.json"),
        serde_json::to_vec(&rules).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("missing-reviewed.md"),
        "U001: TARGET_ALPHA\n\nU002: TARGET_BETA\n\nU003: TARGET_GAMMA\n",
    )
    .unwrap();
}

fn semantic_verdicts(report: &Value) -> BTreeMap<String, Value> {
    report["occurrences"].as_array().unwrap().iter().map(|occurrence| {
        assert_eq!(occurrence["kind"],"judged");
        let verdict=&occurrence["judgment"]["verdict"];
        let reference=&verdict["basis"]["answer"];
        let request=usize::try_from(reference["request_index"].as_u64().unwrap()).unwrap();
        let question=reference["question_name"].as_str().unwrap();
        let answer=&report["requests"][request]["status"]["outcome"]["answers"][question];
        (occurrence["key"]["rule_id"].as_str().unwrap().into(),json!({"target":occurrence["judgment"]["target"],"answer":answer,"value":verdict["value"],"result":verdict["result"]}))
    }).collect()
}

#[tokio::test]
async fn partial_rule_sources_keep_each_surviving_target_answer_and_verdict() {
    let server = AnswerServer::start(ReplyMode::AnswersByQuestion {
        answers: ANSWERS,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let (directory, input) = source_fixture();
    let root = directory.path();
    write_three(root);
    let input = runtime_input(input, &server.endpoint);
    let baseline = response(&run(root, "evaluate_rules", input.clone()).await, 0);
    let report = &baseline["result"]["outcome"]["report"];
    assert_eq!(report["rule_coverage"], "full");
    assert_eq!(report["skipped_rules"], json!([]));
    let baseline = semantic_verdicts(report);
    assert_eq!(baseline.len(), 3);
    assert_eq!(baseline["alpha"]["value"], 0.9);
    assert_eq!(baseline["alpha"]["result"]["kind"], "pass");
    assert_eq!(baseline["beta"]["value"], 0.1);
    assert_eq!(baseline["beta"]["result"]["kind"], "fail");
    std::fs::write(
        root.join("teaching.md"),
        MARKDOWN.replace("gamma<!--", "gamma\\<!--"),
    )
    .unwrap();
    let before = server.count();
    let preparation = response(&run(root, "prepare_rules", input.clone()).await, 0);
    assert_eq!(server.count(), before);
    assert_eq!(
        preparation["result"]["outcome"]["preparation"]["rule_coverage"],
        "partial"
    );
    let partial = response(&run(root, "evaluate_rules", input.clone()).await, 0);
    let report = &partial["result"]["outcome"]["report"];
    assert_eq!(report["rule_coverage"], "partial");
    assert_eq!(report["skipped_rules"].as_array().unwrap().len(), 1);
    assert_eq!(report["skipped_rules"][0]["kind"], "entry");
    assert_eq!(report["skipped_rules"][0]["key"], "K016");
    assert_eq!(report["measurements"]["invalid_rule_entries"], 1);
    assert_eq!(report["measurements"]["skipped_rule_sources"], 0);
    assert_eq!(report["measurements"]["rules_skipped"], 0);
    let survivors = semantic_verdicts(report);
    assert_eq!(survivors.len(), 2);
    for name in ["alpha", "beta"] {
        assert_eq!(survivors[name], baseline[name]);
    }
    std::fs::write(root.join("teaching.md"), MARKDOWN.replace("<!--", "\\<!--")).unwrap();
    let none = response(&run(root, "evaluate_rules", input.clone()).await, 3);
    let report = &none["result"]["outcome"]["report"];
    assert_eq!(none["result"]["outcome"]["kind"], "incomplete_report");
    assert_eq!(report["rule_coverage"], "none");
    assert_eq!(report["occurrences"], json!([]));
    assert_eq!(report["measurements"]["invalid_rule_entries"], 3);
    assert_eq!(report["measurements"]["model_requests"], 0);
    let none_prepared = response(&run(root, "prepare_rules", input).await, 3);
    assert_eq!(
        none_prepared["result"]["outcome"]["preparation"]["rule_coverage"],
        "none"
    );
    let (count, bodies) = server.finish().await;
    assert_eq!(count, 2);
    record_observation(
        "rule-source-survivors",
        json!({"baseline":baseline,"survivors":survivors,"requests":bodies}),
    );
}

#[tokio::test]
async fn malformed_source_is_one_skip_and_valid_independent_source_still_runs() {
    for broken in ["rules_json", "markdown_utf8", "markdown_missing"] {
        let server = AnswerServer::start(ReplyMode::AnswersByQuestion {
            answers: ANSWERS,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let (directory, input) = source_fixture();
        let root = directory.path();
        write_three(root);
        std::fs::write(root.join("other.md"), "Judge alpha<!-- K014 -->\n").unwrap();
        std::fs::write(root.join("other.rules.json"),serde_json::to_vec(&json!({"K014":{"name":"independent","target":{"input":"skill","select":{"kind":"paragraph","target_id":"U001"}},"threshold":0.8}})).unwrap()).unwrap();
        let mut header: Value =
            serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
        header["rule_sources"]
            .as_array_mut()
            .unwrap()
            .push(json!({"markdown":"other.md","rules":"other.rules.json"}));
        std::fs::write(root.join("set.json"), serde_json::to_vec(&header).unwrap()).unwrap();
        let source_file = match broken {
            "rules_json" => {
                std::fs::write(root.join("teaching.rules.json"), "{").unwrap();
                "teaching.rules.json"
            }
            "markdown_utf8" => {
                std::fs::write(root.join("teaching.md"), [0xff]).unwrap();
                "teaching.md"
            }
            "markdown_missing" => {
                std::fs::remove_file(root.join("teaching.md")).unwrap();
                "teaching.md"
            }
            _ => panic!("case"),
        };
        let mut input = runtime_input(input, &server.endpoint);
        let group = input["definitions"]["rule_sets"][0]["binding_groups"][0].clone();
        input["definitions"]["rule_sets"][0]["binding_groups"]
            .as_array_mut()
            .unwrap()
            .push(group);
        let wire = response(&run(root, "evaluate_rules", input).await, 0);
        let report = &wire["result"]["outcome"]["report"];
        assert_eq!(report["rule_coverage"], "partial");
        assert_eq!(report["skipped_rules"].as_array().unwrap().len(), 1);
        assert_eq!(report["skipped_rules"][0]["kind"], "source");
        assert_eq!(
            report["skipped_rules"][0]["source_file"],
            json!(root.canonicalize().unwrap().join(source_file))
        );
        assert_eq!(report["measurements"]["invalid_rule_entries"], 0);
        assert_eq!(report["measurements"]["skipped_rule_sources"], 1);
        let occurrences = report["occurrences"].as_array().unwrap();
        assert_eq!(occurrences.len(), 2);
        assert!(
            occurrences
                .iter()
                .all(|occurrence| occurrence["key"]["rule_id"] == "independent"
                    && occurrence["judgment"]["verdict"]["value"] == 0.9)
        );
        assert_eq!(server.finish().await.0, 2);
    }
}

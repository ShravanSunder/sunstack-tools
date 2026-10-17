#![allow(clippy::unwrap_used)]
#[path = "../../sunstack-rules/tests/support/change_fixture.rs"]
mod changes;
#[path = "support/rule_source_fixture.rs"]
mod rule_source_fixture;
#[path = "support/rule_transport.rs"]
mod rule_transport;
mod support;
use rule_source_fixture::{runtime_input, write_source_fixture};
use rule_transport::{AnswerServer, ReplyMode, RouterReply};
use serde_json::{Value, json};
use support::*;

#[tokio::test]
async fn either_definition_file_selects_the_rule_while_an_unchanged_pair_is_pruned() {
    for variant in ["unchanged", "markdown_only", "rules_only"] {
        let fixture = changes::ChangeFixture::new();
        let root = fixture.root.path();
        let mut input = write_source_fixture(root);
        fixture.write("missing-reviewed.md", "U001: unchanged target\n");
        for path in [
            "set.json",
            "teaching.md",
            "teaching.rules.json",
            "missing-reviewed.md",
        ] {
            fixture.stage(path);
        }
        fixture.commit();
        if variant == "markdown_only" {
            fixture.write("teaching.md","## Standard<!-- K014 -->\n\nKeep *visible* words and `code`.\n\nAn additional authored sentence.\n");
        } else if variant == "rules_only" {
            let path = root.join("teaching.rules.json");
            let mut rules: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            rules["K014"]["threshold"] = json!(0.7);
            std::fs::write(path, serde_json::to_vec(&rules).unwrap()).unwrap();
        }
        input["repositories"][0]["change_scope"] =
            json!({"kind":"uncommitted","layers":["unstaged"]});
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.9,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let mut input = runtime_input(input, &server.endpoint);
        input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
        let wire = response(&run(root, "evaluate_rules", input).await, 0);
        let report = &wire["result"]["outcome"]["report"];
        assert_eq!(report["rule_coverage"], "full");
        assert_eq!(report["occurrences"].as_array().unwrap().len(), 1);
        let occurrence = &report["occurrences"][0];
        if variant == "unchanged" {
            assert_eq!(occurrence["kind"], "outside_change_scope");
            assert_eq!(server.finish().await.0, 0);
        } else {
            assert_eq!(occurrence["kind"], "judged", "{variant}: {wire}");
            assert_eq!(
                occurrence["selected_by"],
                json!(["rule_changed"]),
                "{variant}"
            );
            assert_eq!(server.finish().await.0, 1);
        }
    }
}

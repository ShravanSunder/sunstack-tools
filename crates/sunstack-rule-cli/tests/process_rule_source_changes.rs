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

#[tokio::test]
async fn committed_definition_changes_select_rules_and_missing_baselines_stay_conservative() {
    let fixture = changes::ChangeFixture::new();
    let root = fixture.root.path();
    let input = write_source_fixture(root);
    fixture.write("missing-reviewed.md", "U001: unchanged reviewed target\n");
    let tracked_paths = [
        "set.json",
        "teaching.md",
        "teaching.rules.json",
        "missing-reviewed.md",
    ];
    for path in tracked_paths {
        fixture.stage(path);
    }
    let base_revision = fixture.commit();
    let repository = git2::Repository::open(root).unwrap();
    let base_commit = repository
        .find_commit(git2::Oid::from_str(&base_revision).unwrap())
        .unwrap();
    let base_tree = base_commit.tree().unwrap();
    let reviewed_blob = base_tree
        .get_path(std::path::Path::new("missing-reviewed.md"))
        .unwrap()
        .id();
    let original_markdown = std::fs::read(root.join("teaching.md")).unwrap();
    let original_rules = std::fs::read(root.join("teaching.rules.json")).unwrap();
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let mut input = runtime_input(input, &server.endpoint);
    input["discovery"] = serde_json::to_value(changes::discovery()).unwrap();
    let mut observations = Vec::new();
    let mut last_revision = base_revision.clone();
    // Siblings isolate base..commit: the unrelated control must not inherit earlier definition edits.
    for (variant, changed_path) in [
        ("markdown_only", "teaching.md"),
        ("rules_only", "teaching.rules.json"),
        ("unrelated_only", "unrelated.txt"),
    ] {
        std::fs::write(root.join("teaching.md"), &original_markdown).unwrap();
        std::fs::write(root.join("teaching.rules.json"), &original_rules).unwrap();
        match variant {
            "markdown_only" => fixture.write(
                "teaching.md",
                "## Standard<!-- K014 -->\n\nAn edited teaching standard.\n",
            ),
            "rules_only" => {
                let mut rules: Value = serde_json::from_slice(&original_rules).unwrap();
                rules["K014"]["threshold"] = json!(0.7);
                std::fs::write(root.join(changed_path), serde_json::to_vec(&rules).unwrap())
                    .unwrap();
            }
            _ => fixture.write(changed_path, "unrelated change\n"),
        }
        let mut index = repository.index().unwrap();
        for path in tracked_paths
            .into_iter()
            .chain(std::iter::once(changed_path))
        {
            index.add_path(std::path::Path::new(path)).unwrap();
        }
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git2::Signature::new(
            "Fixture",
            "fixture@example.invalid",
            &git2::Time::new(1001, 0),
        )
        .unwrap();
        let reference = format!("refs/heads/{variant}");
        let revision = repository
            .commit(
                Some(&reference),
                &signature,
                &signature,
                variant,
                &tree,
                &[&base_commit],
            )
            .unwrap();
        let commit = repository.find_commit(revision).unwrap();
        assert_eq!(commit.parent_count(), 1);
        assert_eq!(commit.parent_id(0).unwrap(), base_commit.id());
        let changed = repository
            .diff_tree_to_tree(Some(&base_tree), Some(&tree), None)
            .unwrap();
        let paths = changed
            .deltas()
            .map(|delta| delta.new_file().path().unwrap().to_path_buf())
            .collect::<Vec<_>>();
        assert_eq!(paths, vec![std::path::PathBuf::from(changed_path)]);
        assert_eq!(
            tree.get_path(std::path::Path::new("missing-reviewed.md"))
                .unwrap()
                .id(),
            reviewed_blob
        );
        input["repositories"][0]["change_scope"] = json!({"kind":"revision_diff","base_revision":base_revision,"head":{"kind":"commit","revision":revision.to_string()}});
        let before = server.count();
        let wire = response(&run(root, "evaluate_rules", input.clone()).await, 0);
        let report = &wire["result"]["outcome"]["report"];
        assert_eq!(report["rule_coverage"], "full");
        assert_eq!(report["occurrences"].as_array().unwrap().len(), 1);
        let occurrence = &report["occurrences"][0];
        if variant == "unrelated_only" {
            assert_eq!(occurrence["kind"], "outside_change_scope");
            assert_eq!(server.count(), before);
        } else {
            assert_eq!(occurrence["kind"], "judged");
            assert_eq!(occurrence["selected_by"], json!(["rule_changed"]));
            assert_eq!(server.count() - before, 1);
        }
        observations.push(json!({"variant":variant,"base":base_revision,"commit":revision.to_string(),"changed_paths":paths,"reviewed_blob":reviewed_blob.to_string(),"http_requests":server.count()-before,"occurrence":occurrence}));
        last_revision = revision.to_string();
    }
    input["repositories"][0]["change_scope"] = json!({"kind":"revision_diff","base_revision":"absent-baseline","head":{"kind":"commit","revision":last_revision}});
    let before = server.count();
    let conservative = response(&run(root, "evaluate_rules", input).await, 0);
    let occurrence = &conservative["result"]["outcome"]["report"]["occurrences"][0];
    assert_eq!(occurrence["kind"], "judged");
    assert_eq!(occurrence["selected_by"], json!(["baseline_unavailable"]));
    assert_eq!(server.count() - before, 1);
    observations.push(json!({"variant":"unavailable_baseline","http_requests":server.count()-before,"occurrence":occurrence}));
    let (_, bodies) = server.finish().await;
    record_observation(
        "committed-rule-source-changes",
        json!({"cases":observations,"requests":bodies}),
    );
}

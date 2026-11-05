use serde_json::{Value, json};
use std::path::Path;
use sunstack_decisions::CancellationToken;
use sunstack_lint::test_support::lint_fixture as fixture;
use sunstack_lint::*;
use sunstack_rules::test_support::controlled_http::ControlledServer;

fn setup(root: &Path, endpoint: &str) -> Value {
    let mut config = fixture::write(root);
    std::fs::write(
        root.join("evidence/rules/member.yml"),
        "id: member\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    config["evidence"]["kinds"]["member"] =
        json!({"rule":"member","fields":{"description":{"from":"first_line"}}});
    config["check"]["files"] = json!(["packages/*/references/*.md"]);
    config["filters"] = json!({"purpose":{"per":"packages/{name}","reads":{"file":"about.md","kind":"skill","field":"description"},"question":{"type":"choice","instructions":{"task":"Classify purpose"},"criteria":{"a":"A","b":"B"}},"min_probability":0.7}});
    config["decision_model"]["endpoint"] = json!(endpoint);
    for folder in ["one", "two"] {
        std::fs::create_dir_all(root.join(format!("packages/{folder}/references"))).unwrap();
        std::fs::write(
            root.join(format!("packages/{folder}/about.md")),
            format!("---\ndescription: Purpose {folder}\n---\n"),
        )
        .unwrap();
        std::fs::write(
            root.join(format!("packages/{folder}/references/member.md")),
            "First.\n\nSecond.\n\nThird.\n",
        )
        .unwrap();
    }
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.join("rules.json")).unwrap()).unwrap();
    entries["K001"]["evidence"] = json!("member");
    entries["K001"]["only_when"] = json!({"purpose":["a"]});
    fixture::entry_file(root, &entries);
    fixture::config_file(root, &config);
    config
}

async fn execute(
    root: &Path,
    mut server: ControlledServer,
    probability: f64,
    selected: &str,
) -> (LintReport, Vec<Value>) {
    let request = fixture::request(root);
    let mut task = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC".into()),
    ));
    let mut bodies = Vec::new();
    let report = loop {
        tokio::select! {
            biased;
            result=&mut task=>break result.unwrap(),
            pending=server.receive()=>{
                let answers=pending.body["questions"].as_object().unwrap().iter().map(|(name,question)| {
                    (name.clone(), if question["type"]=="choice" { json!({"type":"choice","choice":selected,"confidence":0.99,"probabilities":{"a":probability,"b":1.0-probability}}) } else {json!({"type":"noul","noul":0.9})})
                }).collect::<serde_json::Map<_,_>>();
                bodies.push(pending.body.clone());
                pending.respond(200,json!({"model":"typesafe/controlled","answers":answers,"usage":{"input_tokens":20,"output_tokens":5}}));
            }
        }
    };
    assert_eq!(server.finish().await, bodies.len());
    let payload: Value = serde_json::from_str(&render(&report, OutputFormat::Json)).unwrap();
    let schema = serde_json::to_value(report_schema()).unwrap();
    let validator = jsonschema::options().offline().build(&schema).unwrap();
    assert!(validator.is_valid(&payload), "{payload}");
    let mut invalid = payload;
    invalid.as_object_mut().unwrap().remove("filter_skips");
    assert!(!validator.is_valid(&invalid));
    (report, bodies)
}

#[tokio::test]
async fn filter_selected_probability_controls_skips_exclusions_and_same_process_freshness() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut config = setup(root.path(), &server.endpoint);
    let (unsure, bodies) = execute(root.path(), server, 0.3, "a").await;
    assert_eq!(bodies.len(), 2, "{unsure:?}");
    assert!(
        bodies
            .iter()
            .all(|body| body["questions"]["filter"] == config["filters"]["purpose"]["question"])
    );
    assert_eq!(unsure.status, LintStatus::Uncertain);
    let unsure = serde_json::to_value(unsure).unwrap();
    assert_eq!(unsure["filter_skips"].as_array().unwrap().len(), 2);
    assert!(unsure["results"].as_array().unwrap().is_empty());
    assert_eq!(unsure["filter_answers"][0]["state"], "unsure");
    assert_eq!(unsure["filter_answers"][0]["answer"], "a");
    assert_eq!(unsure["filter_answers"][0]["probability"], 0.3);

    let server = ControlledServer::start().await;
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    std::fs::write(
        root.path().join("packages/one/about.md"),
        "---\ndescription: Changed purpose\n---\n",
    )
    .unwrap();
    let (allowed, bodies) = execute(root.path(), server, 0.7, "a").await;
    assert_eq!(bodies.len(), 2);
    assert!(
        bodies
            .iter()
            .any(|body| body["state"]["description"] == "Changed purpose")
    );
    assert_eq!(allowed.results.len(), 6);
    assert_eq!(allowed.status, LintStatus::Findings);

    let server = ControlledServer::start().await;
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    let (excluded, bodies) = execute(root.path(), server, 0.2, "b").await;
    assert_eq!(bodies.len(), 2);
    let excluded = serde_json::to_value(excluded).unwrap();
    assert_eq!(excluded["results"].as_array().unwrap().len(), 6);
    for result in excluded["results"].as_array().unwrap() {
        assert_eq!(result["outcome"], "not_applicable");
        assert_eq!(result["basis"], "judged");
        assert_eq!(result["reason"]["kind"], "filter");
        assert_eq!(result["reason"]["answer"], "b");
        assert_eq!(result["path"], json!([]));
    }
}

#[tokio::test]
async fn filters_refuse_unusable_extracted_reads_without_a_model_request() {
    for (text, reason) in [
        ("---\nname: Missing description\n---\n", "reads_absent"),
        ("---\ndescription: [one, two]\n---\n", "reads_unavailable"),
        ("---\ndescription: [broken\n---\n", "reads_unavailable"),
        ("# No frontmatter\n", "undescribed"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let server = ControlledServer::start().await;
        let mut config = setup(root.path(), &server.endpoint);
        config["check"]["files"] = json!(["packages/one/references/*.md"]);
        fixture::config_file(root.path(), &config);
        std::fs::write(root.path().join("packages/one/about.md"), text).unwrap();
        let (report, bodies) = execute(root.path(), server, 0.9, "a").await;
        assert!(bodies.is_empty());
        assert_eq!(report.status, LintStatus::Incomplete);
        assert!(report.results.is_empty());
        let report = serde_json::to_value(report).unwrap();
        assert_eq!(report["filter_answers"][0]["reason"]["kind"], reason);
        assert_eq!(report["filter_skips"].as_array().unwrap().len(), 1);
    }
    for scenario in ["unreadable", "ambiguous"] {
        let root = tempfile::tempdir().unwrap();
        let server = ControlledServer::start().await;
        let mut config = setup(root.path(), &server.endpoint);
        config["check"]["files"] = json!(["packages/one/references/*.md"]);
        if scenario == "unreadable" {
            std::fs::write(root.path().join("packages/one/about.md"), [0xff, 0xfe]).unwrap();
        } else {
            config["filters"]["purpose"]["reads"]["kind"] = json!("reader");
            config["evidence"]["kinds"]["reader"] =
                json!({"rule":"member","fields":{"description":{"from":"first_line"}}});
            std::fs::write(
                root.path().join("packages/one/about.md"),
                "First paragraph.\n\nSecond paragraph.\n",
            )
            .unwrap();
        }
        fixture::config_file(root.path(), &config);
        let (report, bodies) = execute(root.path(), server, 0.9, "a").await;
        assert!(bodies.is_empty());
        assert_eq!(report.status, LintStatus::Incomplete);
        assert!(report.results.is_empty());
        let report = serde_json::to_value(report).unwrap();
        assert_eq!(
            report["filter_answers"][0]["reason"]["kind"],
            if scenario == "unreadable" {
                "reads_unavailable"
            } else {
                "ambiguous"
            }
        );
    }
}

#[tokio::test]
async fn filters_send_only_the_projected_field_and_preserve_cut_metadata_and_encoded_cap() {
    for (description, cut) in [
        ("Authored [cut] marker".to_owned(), false),
        ("長い description ".repeat(180), true),
    ] {
        let root = tempfile::tempdir().unwrap();
        let server = ControlledServer::start().await;
        let mut config = setup(root.path(), &server.endpoint);
        config["check"]["files"] = json!(["packages/one/references/*.md"]);
        config["evidence"]["kinds"]["skill"]["fields"]["folder"] =
            json!({"from":"yaml","key":"description"});
        config["evidence"]["kinds"]["skill"]["fields"]["secret"] =
            json!({"from":"yaml","key":"secret"});
        config["evidence"]["kinds"]["skill"]["max_chars"] = json!(1000);
        config["filters"]["purpose"]["reads"]["field"] = json!("folder");
        config["filters"]["purpose"]["min_probability"] = json!(0.83);
        fixture::config_file(root.path(), &config);
        std::fs::write(root.path().join("packages/one/about.md"),format!("---\ndescription: {description}\nsecret: NOT_FOR_THE_MODEL\n---\n\nNOT_FOR_THE_MODEL\n")).unwrap();
        let (report, bodies) = execute(root.path(), server, 0.85, "a").await;
        assert_eq!(bodies.len(), 1);
        let state = &bodies[0]["state"];
        assert_eq!(state["where"]["folder"], "packages/one");
        assert!(
            state["folder"]
                .as_str()
                .unwrap()
                .contains(if cut { "[cut]" } else { "Authored" })
        );
        assert!(!state.to_string().contains("NOT_FOR_THE_MODEL"));
        assert_eq!(
            state.get("cut-fields").cloned(),
            if cut { Some(json!(["folder"])) } else { None }
        );
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["filter_answers"][0]["input_cut"], cut);
        assert_eq!(value["filter_answers"][0]["requested_model"], "jev-latest");
        assert_eq!(
            value["filter_answers"][0]["resolved_model"],
            "typesafe/controlled"
        );
        assert_eq!(
            value["filter_answers"][0]["probabilities"]["b"],
            json!(1.0 - 0.85)
        );
        let server = ControlledServer::start().await;
        config["decision_model"]["endpoint"] = json!(server.endpoint);
        fixture::config_file(root.path(), &config);
        let (below_custom_threshold, bodies) = execute(root.path(), server, 0.8, "a").await;
        assert_eq!(bodies.len(), 1);
        assert_eq!(below_custom_threshold.status, LintStatus::Uncertain);
        assert!(below_custom_threshold.results.is_empty());
    }
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut config = setup(root.path(), &server.endpoint);
    config["check"]["files"] = json!(["packages/one/references/*.md"]);
    config["limits"] = json!({"request_max_chars":100000});
    fixture::config_file(root.path(), &config);
    let (positive, bodies) = execute(root.path(), server, 0.9, "a").await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(positive.filter_answers.len(), 1);
    let chars = serde_json::to_string(&bodies[0]).unwrap().chars().count();
    for (cap, sends) in [(chars, true), (chars - 1, false)] {
        let server = ControlledServer::start().await;
        config["decision_model"]["endpoint"] = json!(server.endpoint);
        config["limits"]["request_max_chars"] = json!(cap);
        fixture::config_file(root.path(), &config);
        let (report, bodies) = execute(root.path(), server, 0.9, "a").await;
        assert_eq!(bodies.len(), usize::from(sends));
        if !sends {
            let value = serde_json::to_value(report).unwrap();
            assert_eq!(
                value["filter_answers"][0]["reason"]["kind"],
                "reads_too_large"
            );
            assert_eq!(value["status"]["kind"], "incomplete");
        }
    }
}

#[tokio::test]
async fn selection_filter_and_rule_share_the_request_allowance_and_usage() {
    for allowance in [1, 2, 3] {
        let root = tempfile::tempdir().unwrap();
        let server = ControlledServer::start().await;
        let mut config = setup(root.path(), &server.endpoint);
        config["check"]["files"] = json!(["packages/one/references/*.md"]);
        config["limits"] = json!({"max_model_requests":allowance});
        config["evidence"]["kinds"]["member"]["select"] = json!({"question":{"type":"noul","instructions":{"task":"Select member"}},"yes_at_or_above":0.7,"no_at_or_below":0.3});
        std::fs::write(
            root.path().join("packages/one/references/member.md"),
            "One candidate.\n",
        )
        .unwrap();
        let mut entries: Value =
            serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap())
                .unwrap();
        entries["K001"]["questions"] = json!({"complies":{"type":"noul","instructions":{"task":"Complies?","check":"description"}}});
        entries["K001"]["decision"] = json!({"start":"ask","steps":{"ask":{"ask":"complies","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"pass","no":"not_applicable"}}});
        fixture::entry_file(root.path(), &entries);
        fixture::config_file(root.path(), &config);
        let (report, bodies) = execute(root.path(), server, 0.9, "a").await;
        assert_eq!(bodies.len(), allowance);
        assert_eq!(report.usage.model_requests as usize, allowance);
        assert!(
            bodies[0]["questions"]
                .as_object()
                .unwrap()
                .keys()
                .all(|name| name.starts_with('c'))
        );
        if allowance >= 2 {
            assert!(bodies[1]["questions"].get("filter").is_some());
        }
        if allowance == 3 {
            assert!(bodies[2]["questions"].get("complies").is_some());
            assert_eq!(report.status, LintStatus::Clean);
        } else {
            assert_eq!(report.status, LintStatus::Incomplete);
        }
    }
}

#[tokio::test]
async fn ungated_choice_exclusion_is_identical_with_filters_absent_or_declared() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut config = setup(root.path(), &server.endpoint);
    let filters = config.as_object_mut().unwrap().remove("filters").unwrap();
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap()).unwrap();
    entries["K001"].as_object_mut().unwrap().remove("only_when");
    entries["K001"]["questions"] = json!({"verdict":{"type":"choice","instructions":{"task":"Judge member"},"criteria":{"a":"A","b":"B"}}});
    entries["K001"]["decision"] = json!({"start":"verdict","steps":{"verdict":{"ask":"verdict","min_probability":0.7,"routes":{"a":"pass","b":"not_applicable"}}}});
    fixture::entry_file(root.path(), &entries);
    fixture::config_file(root.path(), &config);
    let (baseline, requests) = execute(root.path(), server, 0.2, "b").await;
    assert_eq!(requests.len(), 6);
    for result in &baseline.results {
        assert!(matches!(result.outcome, PieceOutcome::NotApplicable));
        assert_eq!(result.basis, Some(sunstack_rules::Basis::Judged));
    }
    let server = ControlledServer::start().await;
    config["filters"] = filters;
    config["decision_model"]["endpoint"] = json!(server.endpoint);
    fixture::config_file(root.path(), &config);
    let (declared, declared_requests) = execute(root.path(), server, 0.2, "b").await;
    assert_eq!(declared_requests, requests);
    assert!(declared.filter_answers.is_empty());
    assert!(declared.filter_skips.is_empty());
    assert_eq!(
        serde_json::to_vec(&declared).unwrap(),
        serde_json::to_vec(&baseline).unwrap()
    );
}

#[tokio::test]
async fn folder_identity_uses_the_full_path_and_asks_only_for_demanded_rule_files() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    let mut config = setup(root.path(), &server.endpoint);
    for (old, parent) in [("one", "left"), ("two", "right")] {
        std::fs::create_dir_all(root.path().join(format!("groups/{parent}/packages"))).unwrap();
        std::fs::rename(
            root.path().join(format!("packages/{old}")),
            root.path().join(format!("groups/{parent}/packages/shared")),
        )
        .unwrap();
    }
    std::fs::create_dir_all(root.path().join("groups/unused/packages/shared/references")).unwrap();
    std::fs::write(
        root.path().join("groups/unused/packages/shared/about.md"),
        "---\ndescription: Not demanded\n---\n",
    )
    .unwrap();
    std::fs::write(
        root.path()
            .join("groups/unused/packages/shared/references/empty.md"),
        "# Heading only\n",
    )
    .unwrap();
    config["check"]["files"] = json!(["groups/*/packages/*/references/*.md"]);
    config["filters"]["purpose"]["per"] = json!("groups/*/packages/{name}");
    fixture::config_file(root.path(), &config);
    let (report, bodies) = execute(root.path(), server, 0.9, "a").await;
    assert_eq!(bodies.len(), 2);
    assert_eq!(report.filter_answers.len(), 2);
    let folders = report
        .filter_answers
        .iter()
        .map(|answer| answer.folder.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        folders,
        [
            "groups/left/packages/shared",
            "groups/right/packages/shared"
        ]
    );
    assert!(
        report
            .results
            .iter()
            .all(|result| !result.file.to_string_lossy().contains("unused"))
    );
}

#[tokio::test]
async fn two_gated_rules_share_answers_across_three_member_files_and_a_second_folder() {
    let root = tempfile::tempdir().unwrap();
    let server = ControlledServer::start().await;
    setup(root.path(), &server.endpoint);
    for folder in ["one", "two"] {
        std::fs::write(
            root.path()
                .join(format!("packages/{folder}/references/member.md")),
            "First.\n",
        )
        .unwrap();
    }
    for file in ["second", "third"] {
        std::fs::write(
            root.path()
                .join(format!("packages/one/references/{file}.md")),
            "First.\n",
        )
        .unwrap();
    }
    let mut entries: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap()).unwrap();
    entries["K002"] = entries["K001"].clone();
    entries["K002"]["name"] = json!("second_rule");
    fixture::entry_file(root.path(), &entries);
    std::fs::write(
        root.path().join("rulebook.md"),
        "Check each member. <!-- K001 -->\n\nCheck it again. <!-- K002 -->\n",
    )
    .unwrap();
    let (report, bodies) = execute(root.path(), server, 0.9, "a").await;
    assert_eq!(bodies.len(), 2);
    assert_eq!(report.filter_answers.len(), 2);
    assert_eq!(report.results.len(), 8);
    assert_eq!(report.counts.error, 8);
    assert!(report.filter_skips.is_empty());
    assert_eq!(
        bodies
            .iter()
            .map(|body| body["state"]["where"]["folder"].as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>(),
        ["packages/one", "packages/two"].into()
    );
}

#[tokio::test]
async fn cancelling_a_received_filter_request_settles_the_shared_dispatcher_without_a_rule_walk() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut config = setup(root.path(), &server.endpoint);
    config["check"]["files"] = json!(["packages/one/references/*.md"]);
    fixture::config_file(root.path(), &config);
    let cancellation = CancellationToken::new();
    let mut task = tokio::spawn(run(
        fixture::request(root.path()),
        cancellation.clone(),
        Some("SYNTHETIC".into()),
    ));
    let pending = tokio::select! {pending=server.next()=>pending,result=&mut task=>panic!("no filter request: {result:?}")};
    assert!(pending.body["questions"].get("filter").is_some());
    cancellation.cancel();
    let report = tokio::time::timeout(std::time::Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    drop(pending);
    assert_eq!(server.finish().await, 1);
    assert_eq!(report.status, LintStatus::Cancelled);
    assert!(report.results.is_empty());
    assert_eq!(report.filter_skips.len(), 1);
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["filter_answers"][0]["reason"]["reason"], "cancelled");
}

#[tokio::test]
async fn unusable_descriptions_in_the_pinned_real_skill_fixture_never_dispatch() {
    // Existing frozen skill fixture; unchanged from the E0 source pin ad0acfd.
    let source = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../sunstack/tests/fixtures/rulebook-lint/skills/practices-research/SKILL.md"),
    )
    .unwrap();
    let line = source
        .lines()
        .find(|line| line.starts_with("description:"))
        .unwrap();
    for replacement in [
        "summary: No description",
        "description: [a, b]",
        "description: [broken",
    ] {
        let root = tempfile::tempdir().unwrap();
        let server = ControlledServer::start().await;
        let mut config = setup(root.path(), &server.endpoint);
        config["check"]["files"] = json!(["packages/one/references/*.md"]);
        fixture::config_file(root.path(), &config);
        std::fs::write(
            root.path().join("packages/one/about.md"),
            source.replacen(line, replacement, 1),
        )
        .unwrap();
        let (report, bodies) = execute(root.path(), server, 0.9, "a").await;
        assert!(bodies.is_empty());
        assert_eq!(report.status, LintStatus::Incomplete);
        assert!(report.results.is_empty());
        assert_eq!(report.filter_skips.len(), 1);
    }
}

use super::*;

fn filter() -> serde_json::Value {
    json!({"per":"packages/{name}","reads":{"file":"about.md","kind":"skill","fields":["description"]},"labels":["general","role","domain","tool_manual","repo_specific"],"questions":{"filter":{"type":"choice","instructions":{"task":"What is this work for?","check":"description"},"criteria":{"general":"General work","role":"A role","domain":"A domain","tool_manual":"A tool","repo_specific":"One product"}}},"decision":{"start":"filter","steps":{"filter":{"ask":"filter","min_probability":0.7,"routes":{"general":{"label":"general"},"role":{"label":"role"},"domain":{"label":"domain"},"tool_manual":{"label":"tool_manual"},"repo_specific":{"label":"repo_specific"}},"uncertain":"unsure"}}}})
}

#[tokio::test]
async fn filters_admit_native_choice_and_refuse_each_invalid_declaration_at_its_path() {
    let root = tempfile::tempdir().unwrap();
    let mut baseline = fixture::write(root.path());
    baseline["filters"] = json!({"kind":filter()});
    fixture::config_file(root.path(), &baseline);
    let location = discover_config(root.path(), None).unwrap();
    let loaded = load_config(&location).unwrap();
    assert!(matches!(
        check(&loaded, CancellationToken::new()).await,
        CheckResult::Valid { .. }
    ));
    for (pointer, bad, path) in [
        ("/per", json!("packages/**/{name}"), "per"),
        ("/per", json!("packages/*"), "per"),
        ("/per", json!("{name}/{name}"), "per"),
        ("/reads/file", json!(""), "reads.file"),
        ("/reads/kind", json!("missing"), "reads.kind"),
        ("/reads/fields", json!(["missing"]), "reads.fields"),
        (
            "/questions/filter",
            json!({"type":"noul","instructions":{}}),
            "questions.filter",
        ),
        (
            "/questions/filter/criteria",
            json!({"one":"Only one"}),
            "questions.filter",
        ),
        (
            "/questions/filter",
            json!({"type":"choice","task":"Wrong envelope","criteria":{"one":"One","two":"Two"}}),
            "questions.filter",
        ),
        (
            "/decision/steps/filter/min_probability",
            json!(0),
            "decision.steps.filter",
        ),
        (
            "/decision/steps/filter/min_probability",
            json!(1.01),
            "decision.steps.filter",
        ),
    ] {
        let mut config = baseline.clone();
        *config["filters"]["kind"].pointer_mut(pointer).unwrap() = bad;
        fixture::config_file(root.path(), &config);
        let message = match load_config(&location) {
            Err(error) => format!("{}: {}", error.json_path.join("."), error.message),
            Ok(config) => match check(&config, CancellationToken::new()).await {
                CheckResult::Invalid { diagnostics } => diagnostics
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
                CheckResult::Valid { .. } => panic!("accepted {pointer}: {config:?}"),
            },
        };
        assert!(
            message.contains(&format!("filters.kind.{path}")),
            "{message}"
        );
    }
    for source in [
        json!({"from":"same_file","rule":"skill"}),
        json!({"from":"target","regex":"(.*)","resolve":"path"}),
    ] {
        let mut config = baseline.clone();
        config["evidence"]["kinds"]["skill"]["fields"]["description"] = source;
        fixture::config_file(root.path(), &config);
        let loaded = load_config(&location).unwrap();
        let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
        else {
            panic!()
        };
        assert!(diagnostics[0].message.contains("filters.kind.reads.fields"));
    }
    for field in ["select", "within"] {
        let mut config = baseline.clone();
        config["evidence"]["kinds"]["skill"][field] = if field == "within" {
            json!(["skill"])
        } else {
            json!({"question":{"type":"noul","instructions":{}},"yes_at_or_above":0.7,"no_at_or_below":0.3})
        };
        fixture::config_file(root.path(), &config);
        let loaded = load_config(&location).unwrap();
        let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
        else {
            panic!()
        };
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("filters.kind.reads.kind")),
            "{diagnostics:?}"
        );
    }
}

#[tokio::test]
async fn only_when_refusals_skip_the_entry_and_name_the_gate_field() {
    for gate in [
        json!(null),
        json!({}),
        json!({"kind":["general"],"other":["general"]}),
        json!({"missing":["general"]}),
        json!({"kind":[]}),
        json!({"kind":["general","general"]}),
        json!({"kind":["unknown"]}),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::write(root.path());
        config["filters"] = json!({"kind":filter()});
        fixture::config_file(root.path(), &config);
        let path = root.path().join("rules.json");
        let mut entries: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        entries["K001"]["only_when"] = gate;
        fixture::entry_file(root.path(), &entries);
        let loaded = load_config(&discover_config(root.path(), None).unwrap()).unwrap();
        let CheckResult::Invalid { diagnostics } = check(&loaded, CancellationToken::new()).await
        else {
            panic!()
        };
        assert!(
            diagnostics.iter().any(|d| d.message.contains("only_when")),
            "{diagnostics:?}"
        );
    }
}

#[tokio::test]
async fn filter_choice_admission_refuses_256_at_exact_criteria_path_with_count_and_limit() {
    for count in [255, 256] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::write(root.path());
        let mut declaration = filter();
        declaration["questions"]["filter"]["criteria"] = serde_json::to_value(
            (0..count)
                .map(|index| (format!("option{index}"), "Option"))
                .collect::<std::collections::BTreeMap<_, _>>(),
        )
        .unwrap();
        declaration["labels"] = serde_json::to_value(
            (0..count)
                .map(|index| format!("option{index}"))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        declaration["decision"]["steps"]["filter"]["routes"] = serde_json::to_value(
            (0..count)
                .map(|index| {
                    (
                        format!("option{index}"),
                        json!({"label":format!("option{index}")}),
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>(),
        )
        .unwrap();
        config["filters"] = json!({"kind":declaration});
        let schema = serde_json::to_value(config_schema()).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert_eq!(validator.is_valid(&config), count == 255);
        fixture::config_file(root.path(), &config);
        let loaded = load_config(&discover_config(root.path(), None).unwrap()).unwrap();
        let checked = check(&loaded, CancellationToken::new()).await;
        if count == 255 {
            assert!(matches!(checked, CheckResult::Valid { .. }));
        } else {
            let CheckResult::Invalid { diagnostics } = checked else {
                panic!("accepted256")
            };
            assert!(
                diagnostics.iter().any(|diagnostic| diagnostic
                    .message
                    .contains("filters.kind.questions.filter.criteria")
                    && diagnostic.message.contains("256")
                    && diagnostic.message.contains("255")),
                "{diagnostics:?}"
            );
            let report = run(
                fixture::request(root.path()),
                CancellationToken::new(),
                None,
            )
            .await;
            assert_eq!(report.status, LintStatus::Invalid);
            assert!(report.diagnostics.iter().any(|diagnostic| {
                diagnostic
                    .message
                    .contains("filters.kind.questions.filter.criteria")
                    && diagnostic.message.contains("256")
                    && diagnostic.message.contains("255")
            }));
        }
    }
}

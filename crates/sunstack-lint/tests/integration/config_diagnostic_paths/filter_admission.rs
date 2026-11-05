use super::*;

fn filter() -> serde_json::Value {
    json!({"per":"packages/{name}","reads":{"file":"about.md","kind":"skill","field":"description"},"question":{"type":"choice","instructions":{"task":"What is this work for?"},"criteria":{"general":"General work","role":"A role","domain":"A domain","tool_manual":"A tool","repo_specific":"One product"}},"min_probability":0.7})
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
        ("/reads/field", json!("missing"), "reads.field"),
        (
            "/question",
            json!({"type":"noul","instructions":{}}),
            "question",
        ),
        ("/question/criteria", json!({"one":"Only one"}), "question"),
        (
            "/question",
            json!({"type":"choice","task":"Wrong envelope","criteria":{"one":"One","two":"Two"}}),
            "question",
        ),
        ("/min_probability", json!(0), "min_probability"),
        ("/min_probability", json!(1.01), "min_probability"),
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
        assert!(diagnostics[0].message.contains("filters.kind.reads.field"));
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

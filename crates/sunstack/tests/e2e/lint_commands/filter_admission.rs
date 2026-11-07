use super::*;

#[tokio::test]
async fn filter_declarations_and_gates_refuse_through_real_check_and_run() {
    let root = tempfile::tempdir().unwrap();
    let mut baseline = fixture::write(root.path());
    baseline["filters"] = json!({"kind":{"per":"packages/{name}","reads":{"file":"about.md","kind":"skill","fields":["description"]},"labels":["general","role","domain","tool_manual","repo_specific"],"questions":{"filter":{"type":"choice","instructions":{"task":"Classify work","check":"description"},"criteria":{"general":"General","role":"Role","domain":"Domain","tool_manual":"Tool","repo_specific":"Product"}}},"decision":{"start":"filter","steps":{"filter":{"ask":"filter","min_probability":0.83,"routes":{"general":{"label":"general"},"role":{"label":"role"},"domain":{"label":"domain"},"tool_manual":{"label":"tool_manual"},"repo_specific":{"label":"repo_specific"}},"uncertain":"unsure"}}}}});
    fixture::config_file(root.path(), &baseline);
    invoke(root.path(), &["--check"], 0).await;
    for (pointer, value, path) in [
        ("/per", json!("packages/**/{name}"), "per"),
        ("/per", json!("packages/*"), "per"),
        ("/per", json!("{name}/{name}"), "per"),
        (
            "/reads",
            json!({"kind":"skill","fields":["description"]}),
            "reads",
        ),
        ("/reads/file", json!(""), "reads.file"),
        ("/reads/kind", json!("unknown"), "reads.kind"),
        ("/reads/fields", json!(["unknown"]), "reads.fields"),
        (
            "/questions/filter",
            json!({"type":"noul","instructions":{"check":"description"}}),
            "decision.steps.filter",
        ),
        (
            "/questions/filter/criteria",
            json!({"one":"One"}),
            "questions.filter",
        ),
        (
            "/questions/filter",
            json!({"type":"choice","task":"Wrong","criteria":{"one":"One","two":"Two"}}),
            "questions.filter",
        ),
        (
            "/decision/steps/filter/min_probability",
            json!(0),
            "decision.steps.filter",
        ),
        (
            "/decision/steps/filter/min_probability",
            json!(1.1),
            "decision.steps.filter",
        ),
    ] {
        let mut config = baseline.clone();
        *config["filters"]["kind"].pointer_mut(pointer).unwrap() = value;
        fixture::config_file(root.path(), &config);
        for args in [&["--check"][..], &["--all"][..]] {
            let output = invoke(root.path(), args, 2).await;
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(text.contains(&format!("filters.kind.{path}")), "{text}");
        }
    }
    for variant in ["same_file", "target", "select", "within"] {
        let mut config = baseline.clone();
        match variant {
            "same_file" => {
                config["evidence"]["kinds"]["skill"]["fields"]["description"] =
                    json!({"from":"same_file","rule":"skill"})
            }
            "target" => {
                config["evidence"]["kinds"]["skill"]["fields"]["description"] =
                    json!({"from":"target","regex":"(.*)","resolve":"path"})
            }
            "select" => {
                config["evidence"]["kinds"]["skill"]["select"] = json!({"question":{"type":"noul","instructions":{}},"yes_at_or_above":0.7,"no_at_or_below":0.3})
            }
            "within" => config["evidence"]["kinds"]["skill"]["within"] = json!(["skill"]),
            _ => unreachable!(),
        }
        fixture::config_file(root.path(), &config);
        let output = invoke(root.path(), &["--check"], 2).await;
        assert!(
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
            .contains("filters.kind.reads")
        );
    }
    fixture::config_file(root.path(), &baseline);
    let entries: Value =
        serde_json::from_slice(&std::fs::read(root.path().join("rules.json")).unwrap()).unwrap();
    for gate in [
        json!(null),
        json!({}),
        json!({"kind":["general"],"other":["general"]}),
        json!({"missing":["general"]}),
        json!({"kind":[]}),
        json!({"kind":["general","general"]}),
        json!({"kind":["unknown"]}),
    ] {
        let mut entries = entries.clone();
        entries["K001"]["only_when"] = gate;
        fixture::entry_file(root.path(), &entries);
        let output = invoke(root.path(), &["--check"], 2).await;
        assert!(
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
            .contains("only_when")
        );
        let result = report(root.path(), &["--all"], 3).await;
        assert_eq!(result["skipped_rules"].as_array().unwrap().len(), 1);
    }
}

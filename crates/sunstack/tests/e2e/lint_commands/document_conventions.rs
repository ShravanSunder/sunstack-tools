use super::*;

#[tokio::test]
async fn document_convention_config_admits_valid_settings_and_locates_bad_shapes() {
    let root = tempfile::tempdir().unwrap();
    let baseline = fixture::write(root.path());
    let mut config = baseline.clone();
    config["evidence"]["html"] = json!({
        "skip_tags":["custom-source"],"text_attributes":{"custom-diagram":["caption"]}
    });
    config["evidence"]["markdown"] = json!({"heading_anchors":"github"});
    fixture::config_file(root.path(), &config);
    invoke(root.path(), &["--check"], 0).await;

    for (field, value) in [
        ("html", json!({"skip_tags":[false]})),
        ("html", json!({"skip_tags":[""]})),
        ("html", json!({"text_attributes":{"custom-diagram":[3]}})),
        ("html", json!({"text_attributes":{"custom-diagram":[""]}})),
        ("html", json!({"text_attributes":{"":["caption"]}})),
        ("markdown", json!({"heading_anchors":"unknown"})),
    ] {
        let mut config = baseline.clone();
        config["evidence"][field] = value;
        fixture::config_file(root.path(), &config);
        let output = invoke(root.path(), &["--check"], 2).await;
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            diagnostic.contains(&format!("evidence.{field}")),
            "{diagnostic}"
        );
        assert!(diagnostic.contains("sunstack.lint.json"), "{diagnostic}");
    }
}

#[tokio::test]
async fn ordinary_cli_shares_project_settings_with_asking_and_outline_collectors() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::write(root.path());
    config["check"]["files"] = json!(["docs/*.html"]);
    std::fs::write(
        root.path().join("evidence/rules/html.yml"),
        "id: whole_html\nlanguage: Html\nrule: {kind: document}\n",
    )
    .unwrap();
    config["evidence"]["kinds"] = json!({
        "page":{"rule":"whole_html","fields":{"body":{"from":"remainder"}}},
        "section":{"html_units":{"mode":"unit","units":["section"]},
            "fields":{"body":{"from":"section","text":"full"}}}
    });
    fixture::entry_file(
        root.path(),
        &json!({"K001":{
            "name":"page_words","evidence":"page","severity":"warn",
            "questions":{"words":{"type":"noul","instructions":{"check":"body","task":"Read the page."}}},
            "decision":{"start":"words","steps":{"words":{"ask":"words","yes_at_or_above":0.8,
                "no_at_or_below":0.2,"yes":"pass","no":{"finding":"Missing words."},"uncertain":"uncertain"}}}
        }}),
    );
    let raw = "<h2>Visible</h2><custom-source><h3>Nested</h3></custom-source><custom-diagram caption='Flow' alt='Picture'>Tail</custom-diagram>";
    std::fs::write(root.path().join("docs/page.html"), raw).unwrap();

    for (configured, expected_text, expected_sections) in [
        (false, "Visible Nested [alt: Picture] Tail", 2),
        (
            true,
            "Visible Nested [caption: Flow] [alt: Picture] Tail",
            1,
        ),
        (false, "Visible Nested [alt: Picture] Tail", 2),
    ] {
        if configured {
            config["evidence"]["html"] = json!({
                "skip_tags":["custom-source"],"text_attributes":{"custom-diagram":["caption"]}
            });
        } else {
            config["evidence"].as_object_mut().unwrap().remove("html");
        }
        fixture::config_file(root.path(), &config);
        let listing: Value = serde_json::from_slice(
            &invoke(root.path(), &["--all", "--evidence", "section"], 0)
                .await
                .stdout,
        )
        .unwrap();
        assert_eq!(
            listing["pieces"].as_array().unwrap().len(),
            expected_sections
        );
        let saved = root.path().join("prepared.json");
        invoke(
            root.path(),
            &["--all", "--prepare", saved.to_str().unwrap()],
            6,
        )
        .await;
        let report: Value = serde_json::from_slice(&std::fs::read(saved).unwrap()).unwrap();
        let requests = report["requests"].as_array().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["request"]["state"]["body"], expected_text);
        assert_eq!(report["usage"]["model_requests"], 0);
        assert!(
            report["piece_evidence"]
                .as_object()
                .unwrap()
                .values()
                .any(|record| record["piece"]["fields"]["body"]["value"] == raw)
        );
    }
}

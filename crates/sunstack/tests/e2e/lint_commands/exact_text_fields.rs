use super::*;

fn text_fixture(root: &Path, quote: &str, source: &str) -> Value {
    let mut config = fixture::write(root);
    config["check"]["files"] = json!(["docs/quote.html"]);
    config["evidence"]["kinds"] = json!({"quote":{"rule":"skill","fields":{
        "quote":{"from":"html_text"},
        "source_name":{"from":"html_attribute","name":"data-source"},
        "source":{"from":"document","path":"source.html","rule":"record"}
    }}});
    std::fs::write(root.join("evidence/rules/skill.yml"), "id: skill\nlanguage: Html\nrule: {kind: element, regex: '^<blockquote'}\n---\nid: record\nlanguage: Html\nrule: {kind: element, regex: '^<p'}\n").unwrap();
    std::fs::write(root.join("docs/quote.html"), quote).unwrap();
    std::fs::write(root.join("docs/source.html"), source).unwrap();
    fixture::config_file(root, &config);
    fixture::entry_file(
        root,
        &json!({"K001":{
            "name":"exact_text","evidence":"quote","severity":"error",
            "decision":{"start":"match","steps":{"match":{
                "mechanical_check":{"tool":"exact_contains_field","field":"source","needle":"quote"},"yes":"pass","no":{"finding":"No exact span."}
            }}}
        }}),
    );
    config
}

#[tokio::test]
async fn decoded_html_fields_preserve_characters_and_source_identity_in_saved_show() {
    let root = tempfile::tempdir().unwrap();
    let raw =
        "<blockquote data-source='a&amp;lt;b'>config<b>uration</b> &amp;lt;\n  “text”</blockquote>";
    text_fixture(root.path(), raw, "<p>configuration &lt;\n  “text”</p>");

    let output = invoke(
        root.path(),
        &["--all", "--report", "report.json", "--format", "json"],
        0,
    )
    .await;

    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let piece = &report["piece_evidence"]["1"]["piece"];
    assert_eq!(
        piece["fields"]["quote"]["value"],
        "configuration &lt;\n  “text”"
    );
    assert_eq!(piece["fields"]["source_name"]["value"], "a&lt;b");
    assert_eq!(
        piece["model_view"]["fields"]["quote"],
        "configuration &lt;\n  “text”"
    );
    assert_eq!(piece["fields"]["quote"]["spans"][0], piece["where"]["span"]);
    assert_eq!(
        piece["fields"]["quote"]["spans"][0]["source"]["content_identity"],
        sunstack_evidence::content_digest(raw.as_bytes())
    );
    let show = invoke(root.path(), &["show", "report.json", "1"], 0).await;
    let detail: Value = serde_json::from_slice(&show.stdout).unwrap();
    assert!(detail.to_string().contains("source.html"));
    assert_eq!(report["usage"]["model_requests"], 0);
}

#[tokio::test]
async fn exact_membership_uses_only_the_named_file_and_never_stitches_or_normalizes() {
    for (source, matches) in [
        ("<p>configuration</p>", true),
        ("<p>Configuration</p>", false),
        ("<p>confi</p><p>guration</p>", false),
        ("<p>confi guration</p>", false),
        ("<p>confi\nguration</p>", false),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut config = text_fixture(root.path(), "<blockquote>config</blockquote>", source);
        // The same needle exists in another readable file; it cannot authorize this source.
        std::fs::write(root.path().join("docs/other.html"), "<p>configuration</p>").unwrap();
        let output = invoke(
            root.path(),
            &["--all", "--report", "report.json", "--format", "json"],
            if matches { 0 } else { 1 },
        )
        .await;
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            report["results"][0]["outcome"],
            if matches { "pass" } else { "finding" }
        );
        config["evidence"]["kinds"]["quote"]["fields"]["source"]["path"] = json!("other.html");
        fixture::config_file(root.path(), &config);
        invoke(root.path(), &["--all", "--format", "json"], 0).await;
    }
}

#[tokio::test]
async fn document_projection_decodes_selected_json_strings_without_merging_records() {
    let root = tempfile::tempdir().unwrap();
    let quote = "Say \"yes\"; it’s\n  clear.";
    let mut config = text_fixture(
        root.path(),
        "<blockquote>Say &quot;yes&quot;; it’s\n  clear.</blockquote>",
        "",
    );
    let records = format!(
        "{}\n{}\n",
        serde_json::to_string(&json!({"id":"first","text":quote})).unwrap(),
        serde_json::to_string(&json!({"id":"second","text":"Other record"})).unwrap()
    );
    std::fs::write(root.path().join("docs/messages.jsonl"), &records).unwrap();
    std::fs::write(root.path().join("evidence/rules/record.yml"), "id: message-text\nlanguage: Json\nrule:\n  kind: string\n  inside:\n    kind: pair\n    field: value\n    has: {field: key, regex: '^\"text\"$'}\n").unwrap();
    config["evidence"]["kinds"]["quote"]["fields"]["source"] = json!({"from":"document","path":"messages.jsonl","rule":"message-text","text":"json_string"});
    fixture::config_file(root.path(), &config);

    let output = invoke(
        root.path(),
        &["--all", "--report", "report.json", "--format", "json"],
        0,
    )
    .await;

    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = &report["piece_evidence"]["1"]["piece"]["fields"];
    assert_eq!(fields["quote"]["value"], quote);
    assert_eq!(fields["source"]["values"], json!([quote, "Other record"]));
    assert_eq!(fields["source"]["spans"].as_array().unwrap().len(), 2);
    for span in fields["source"]["spans"].as_array().unwrap() {
        assert_eq!(
            span["source"]["content_identity"],
            sunstack_evidence::content_digest(records.as_bytes())
        );
        let raw = &records[span["byte_start"].as_u64().unwrap() as usize
            ..span["byte_end"].as_u64().unwrap() as usize];
        assert!(serde_json::from_str::<String>(raw).is_ok());
    }
    let show = invoke(root.path(), &["show", "report.json", "1"], 0).await;
    assert!(
        String::from_utf8(show.stdout)
            .unwrap()
            .contains("messages.jsonl")
    );

    // A record selected by the caller cannot borrow text from its peer.
    let selector_path = root.path().join("evidence/rules/record.yml");
    let selector = std::fs::read_to_string(&selector_path).unwrap();
    std::fs::write(&selector_path,format!("{selector}    inside:\n      kind: object\n      has: {{kind: pair, regex: '\"id\":\"second\"'}}\n")).unwrap();
    invoke(root.path(), &["--all", "--format", "json"], 1).await;
    std::fs::write(
        root.path().join("docs/other.jsonl"),
        serde_json::to_string(&json!({"id":"second","text":quote})).unwrap(),
    )
    .unwrap();
    config["evidence"]["kinds"]["quote"]["fields"]["source"]["path"] = json!("other.jsonl");
    fixture::config_file(root.path(), &config);
    invoke(root.path(), &["--all", "--format", "json"], 0).await;
    config["evidence"]["kinds"]["quote"]["fields"]["source"]["path"] = json!("messages.jsonl");
    std::fs::write(&selector_path, selector).unwrap();

    // Raw is the unchanged default; escaped JSON is not decoded silently.
    config["evidence"]["kinds"]["quote"]["fields"]["source"]
        .as_object_mut()
        .unwrap()
        .remove("text");
    fixture::config_file(root.path(), &config);
    let output = invoke(root.path(), &["--all", "--format", "json"], 1).await;
    let raw: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        raw["piece_evidence"]["1"]["piece"]["fields"]["source"]["values"][0],
        serde_json::to_string(quote).unwrap()
    );
}

#[tokio::test]
async fn decoded_source_comparison_does_not_normalize_spaces_punctuation_or_unicode() {
    for source in [
        "<p>It’s  café.</p>",
        "<p>It's  café.</p>",
        "<p>It’s café.</p>",
        "<p>It’s  cafe\u{301}.</p>",
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut config = text_fixture(root.path(), "<blockquote>It’s  café.</blockquote>", source);
        config["evidence"]["kinds"]["quote"]["fields"]["source"]["text"] = json!("html_text");
        fixture::config_file(root.path(), &config);
        let matches = source == "<p>It’s  café.</p>";
        invoke(
            root.path(),
            &["--all", "--format", "json"],
            if matches { 0 } else { 1 },
        )
        .await;
    }
}

#[tokio::test]
async fn exact_membership_never_passes_empty_missing_cut_unreadable_or_outside_source() {
    for scenario in [
        "empty_needle",
        "missing_needle",
        "empty_source",
        "missing_source",
        "outside_source",
        "cut_source",
        "cut_needle",
    ] {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let needle = if scenario == "empty_needle" {
            String::new()
        } else if scenario == "cut_needle" {
            "config".repeat(500)
        } else {
            "config".into()
        };
        let source = if scenario == "empty_source" {
            String::new()
        } else if scenario == "cut_source" {
            format!("<p>config{}</p>", "x".repeat(3000))
        } else {
            format!("<p>{needle}</p>")
        };
        let mut config = text_fixture(
            root.path(),
            &format!("<blockquote>{needle}</blockquote>"),
            &source,
        );
        if scenario == "missing_needle" {
            config["evidence"]["kinds"]["quote"]["fields"]["quote"] =
                json!({"from":"html_attribute","name":"missing"});
        }
        if scenario == "missing_source" {
            std::fs::remove_file(root.path().join("docs/source.html")).unwrap();
        }
        if scenario == "outside_source" {
            let path = outside.path().join("source.html");
            std::fs::write(&path, &source).unwrap();
            config["evidence"]["kinds"]["quote"]["fields"]["source"]["path"] = json!(path);
        }
        if scenario.starts_with("cut_") {
            config["evidence"]["kinds"]["quote"]["max_chars"] = json!(1000);
        }
        fixture::config_file(root.path(), &config);
        let output = invoke(
            root.path(),
            &["--all", "--format", "json"],
            if scenario == "empty_source" { 1 } else { 3 },
        )
        .await;
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_ne!(report["results"][0]["outcome"], "pass", "{scenario}");
        assert_eq!(report["usage"]["model_requests"], 0);
        let piece = &report["piece_evidence"]["1"]["piece"];
        if scenario == "empty_needle" {
            assert_eq!(piece["fields"]["quote"]["value"], "");
        } else if scenario == "outside_source" {
            assert_eq!(piece["fields"]["source"]["reason"], "outside_root");
        } else if scenario == "cut_source" {
            assert!(
                piece["model_view"]["cut"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("source"))
            );
        }
        if scenario == "missing_source" {
            let listing = invoke(root.path(), &["--all", "--evidence", "quote"], 0).await;
            assert!(listing.stderr.is_empty());
        }
    }
}

use super::*;

#[tokio::test]
async fn section_levels_keep_a_sibling_subsection_picture_in_the_governing_prefix() {
    for surface in ["html", "markdown", "island"] {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture::write(root.path());
        let html = surface == "html";
        let extension = if surface == "markdown" { "md" } else { "html" };
        config["check"]["files"] = json!([format!("docs/*.{extension}")]);
        config["evidence"]["html"] =
            json!({"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]});
        let readme = include_str!("../../../README.md");
        let selection: Value = serde_json::from_str(
            readme
                .split_once("<!-- example: governing-section-levels -->\n```json\n")
                .unwrap()
                .1
                .split_once("\n```")
                .unwrap()
                .0,
        )
        .unwrap();
        let governing = if html {
            json!({"html_units":selection,"fields":{}})
        } else {
            json!({"markdown_units":selection,"fields":{}})
        };
        config["evidence"]["kinds"] = json!({
            "governing":governing,
            "picture":{"rule":"picture","fields":{}},
            "choice":{"rule":"choice","fields":{
                "prefix":{"from":"section_prefix","kind":"governing"},
                "pictures":{"from":"inside","kind":"picture","scope":{"section_prefix":"governing"}}
            }}
        });
        let rules = if html {
            "id: picture\nlanguage: Html\nrule: {kind: element, regex: '^<figure'}\n---\nid: choice\nlanguage: Html\nrule: {kind: element, regex: '^<aside'}\n"
        } else {
            "id: picture\nlanguage: Markdown\nrule: {kind: fenced_code_block}\n---\nid: choice\nlanguage: Markdown\nrule: {kind: paragraph, regex: '^Choose'}\n"
        };
        std::fs::write(root.path().join("evidence/rules/levels.yml"), rules).unwrap();
        let raw = if html {
            "<h1>Document</h1>\n<h2>Decision</h2>\n<h3>Drawing</h3>\n<figure>MODEL</figure>\n<h3>Options</h3>\n<aside>Choose owner.</aside>\n<h2>Peer</h2>\n<figure>LATER</figure>\n"
        } else {
            "# Document\n\n## Decision\n\n### Drawing\n\n```mermaid\ngraph LR\n A --> B\n```\n\n### Options\n\nChoose owner.\n\n## Peer\n\n```mermaid\nLATER\n```\n"
        };
        let page = if surface == "island" {
            format!("<script type='text/markdown'>\n{raw}</script>")
        } else {
            raw.into()
        };
        std::fs::write(root.path().join(format!("docs/page.{extension}")), &page).unwrap();
        fixture::entry_file(
            root.path(),
            &json!({"K001":{"name":"picture","evidence":"choice","severity":"warn","decision":{"start":"picture","steps":{"picture":{"mechanical_check":{"tool":"piece_count","field":"pictures","bound":{"greater_than":0}},"yes":"pass","no":{"finding":"Missing picture."}}}}}}),
        );
        fixture::config_file(root.path(), &config);

        invoke(root.path(), &["--check"], 0).await;
        let observed = report(root.path(), &["--all"], 0).await;

        assert_eq!(observed["counts"]["warn"], 0, "{surface}: {observed}");
        assert_eq!(observed["results"].as_array().unwrap().len(), 1);
        let piece = &observed["piece_evidence"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()["piece"];
        let start = page
            .find(if html { "<h2>Decision" } else { "## Decision" })
            .unwrap();
        let end = page.find(if html { "<aside>" } else { "Choose" }).unwrap();
        assert_eq!(piece["fields"]["prefix"]["value"], page[start..end]);
        assert_eq!(piece["fields"]["prefix"]["spans"][0]["byte_start"], start);
        assert_eq!(piece["fields"]["prefix"]["spans"][0]["byte_end"], end);
        assert_eq!(
            piece["fields"]["pictures"]["values"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(!piece["fields"]["pictures"].to_string().contains("LATER"));
        assert_eq!(observed["usage"]["model_requests"], 0);

        let key = if html { "html_units" } else { "markdown_units" };
        config["evidence"]["kinds"]["governing"][key]
            .as_object_mut()
            .unwrap()
            .remove("levels");
        fixture::config_file(root.path(), &config);
        let nearest = report(root.path(), &["--all"], 0).await;
        assert_eq!(
            nearest["counts"]["warn"], 1,
            "nearest h3 must not borrow its sibling's picture"
        );
    }
}

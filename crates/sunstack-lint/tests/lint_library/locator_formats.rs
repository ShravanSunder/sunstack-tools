use super::*;

#[tokio::test]
async fn default_and_github_render_the_primary_anchor_line_and_keep_the_default_control() {
    let root = tempfile::tempdir().unwrap();
    fixture::write(root.path());
    let report = run(fixture::request(root.path()), CancellationToken::new()).await;
    for (anchor, line) in [("resolved", 17), ("default", 7)] {
        let mut value = serde_json::to_value(&report).unwrap();
        value["findings"][0]["line"] = json!(line);
        value["findings"][0]["anchor"] = json!(anchor);
        value["findings"][0]["span"]["start_line"] = json!(7);
        value["findings"][0]["span"]["end_line"] = json!(20);
        let typed: LintReport = serde_json::from_value(value).unwrap();
        let human = render(&typed, OutputFormat::Default);
        assert!(
            human
                .lines()
                .nth(1)
                .unwrap()
                .starts_with(&format!("docs/one.md:{line}:")),
            "{human}"
        );
        let github = render(&typed, OutputFormat::Github);
        assert!(
            github
                .lines()
                .next()
                .unwrap()
                .contains(&format!("file=docs/one.md,line={line},")),
            "{github}"
        );
    }
}

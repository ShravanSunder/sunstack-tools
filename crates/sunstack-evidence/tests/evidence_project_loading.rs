#![allow(clippy::unwrap_used)]
use serde::Deserialize;
use std::{collections::BTreeSet, path::PathBuf};
use sunstack_evidence::load_evidence_project;

#[derive(Deserialize)]
struct Oracle {
    matches: Vec<ScanMatch>,
}
#[derive(Deserialize)]
struct ScanMatch {
    rule: String,
    file: PathBuf,
    start: usize,
    end: usize,
    text: String,
}
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sunstack/tests/fixtures/rulebook-lint")
}

#[test]
fn project_matches_equal_native_scan_in_both_directions() {
    let root = fixture();
    let project = load_evidence_project(&root.join("evidence/sgconfig.yml")).unwrap();
    let oracle: Oracle = serde_json::from_str(
        &std::fs::read_to_string(root.join("expected/scan-oracle.json")).unwrap(),
    )
    .unwrap();
    let files = oracle
        .matches
        .iter()
        .map(|m| m.file.clone())
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    for file in files {
        let source = std::fs::read_to_string(root.join(&file)).unwrap();
        let index = project.match_file(&file, &source, usize::MAX);
        assert!(!index.limited);
        for (rule, spans) in index.by_rule {
            for span in spans {
                actual.insert((
                    rule.as_str().to_owned(),
                    file.clone(),
                    span.start,
                    span.end,
                    source[span.start..span.end].to_owned(),
                ));
            }
        }
    }
    let expected = oracle
        .matches
        .into_iter()
        .map(|m| (m.rule, m.file, m.start, m.end, m.text))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn reserved_utils_and_other_evidence_languages_are_refused() {
    for util in [
        "id: quoted\nlanguage: Markdown\nrule: {kind: paragraph}\n",
        "id: example\nlanguage: Markdown\nutils: {quoted: {kind: paragraph}}\nrule: {matches: quoted}\n",
        "id: other\nlanguage: JavaScript\nrule: {kind: program}\n",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("sgconfig.yml"),
            "ruleDirs: []\nutilDirs: [utils]\n",
        )
        .unwrap();
        std::fs::create_dir(dir.path().join("utils")).unwrap();
        std::fs::write(dir.path().join("utils/invalid.yml"), util).unwrap();
        assert!(load_evidence_project(&dir.path().join("sgconfig.yml")).is_err());
    }
}

#[test]
fn json_projects_use_json_nodes_and_source_ranges() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/json.yml"),
        "id: pair\nlanguage: Json\nrule: {kind: pair}\n",
    )
    .unwrap();
    let project = load_evidence_project(&root.path().join("sgconfig.yml")).unwrap();
    let text = "{\n  \"first\": true,\n  \"second\": false\n}\n";
    let index = project.match_file(std::path::Path::new("rules.json"), text, 100);
    let spans = &index.by_rule["pair"];
    assert_eq!(
        spans
            .iter()
            .map(|range| &text[range.start..range.end])
            .collect::<Vec<_>>(),
        ["\"first\": true", "\"second\": false"]
    );
    assert_eq!(index.parents.len(), 2);
}

#[test]
fn json_utils_are_admitted_and_all_other_languages_remain_refused() {
    use ast_grep_language::{LanguageExt, SupportLang};
    for language in SupportLang::all_langs() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("utils")).unwrap();
        std::fs::write(
            root.path().join("sgconfig.yml"),
            "ruleDirs: []\nutilDirs: [utils]\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("utils/rule.yml"),
            format!("id: caller_util\nlanguage: {language}\nrule: {{regex: example}}\n"),
        )
        .unwrap();
        assert_eq!(
            load_evidence_project(&root.path().join("sgconfig.yml")).is_ok(),
            matches!(language, SupportLang::Markdown | SupportLang::Json),
            "{language}"
        );
        std::fs::create_dir(root.path().join("rules")).unwrap();
        std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
        let root_kind = language.ast_grep("").root().kind().into_owned();
        std::fs::write(
            root.path().join("rules/rule.yml"),
            format!("id: caller_rule\nlanguage: {language}\nrule: {{kind: {root_kind}, regex: example}}\n"),
        )
        .unwrap();
        let rule_result = load_evidence_project(&root.path().join("sgconfig.yml"));
        assert_eq!(
            rule_result.is_ok(),
            matches!(language, SupportLang::Markdown | SupportLang::Json),
            "rule {language}: {:?}",
            rule_result.err()
        );
    }
}

#[test]
fn standard_utils_each_match_positive_and_reject_negative_shapes() {
    let cases = [
        (
            "numbered-heading-section",
            "### 3. Inspect\n\nBody.\n",
            "### Inspect\n\nBody.\n",
        ),
        (
            "step-heading-section",
            "### Step 3: Inspect\n\nBody.\n",
            "### Step three: Inspect\n\nBody.\n",
        ),
        (
            "bold-numbered-item",
            "3. **Inspect** details.\n",
            "3. Inspect details.\n",
        ),
        (
            "numbered-item",
            "3) Inspect details.\n",
            "- Inspect details.\n",
        ),
        ("quoted", "> quoted text\n", "ordinary text\n"),
        (
            "frontmatter",
            "---\nname: example\n---\n",
            "name: example\n",
        ),
    ];
    for (id, positive, negative) in cases {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("rules")).unwrap();
        std::fs::write(dir.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
        std::fs::write(
            dir.path().join("rules/check.yml"),
            format!("id: example\nlanguage: Markdown\nrule: {{matches: {id}}}\n"),
        )
        .unwrap();
        let project = load_evidence_project(&dir.path().join("sgconfig.yml")).unwrap();
        assert_eq!(
            project
                .match_file(std::path::Path::new("example.md"), positive, 10)
                .by_rule["example"]
                .len(),
            1,
            "{id}"
        );
        assert!(
            project
                .match_file(std::path::Path::new("example.md"), negative, 10)
                .by_rule["example"]
                .is_empty(),
            "{id}"
        );
    }
}

#[test]
fn missing_util_bad_rule_glob_and_reserved_rule_are_rejected_at_load() {
    for rule in [
        "id: example\nlanguage: Markdown\nutils: {quoted: {kind: paragraph}}\nrule: {matches: quoted}\n",
        "id: example\nlanguage: Markdown\nrule: {matches: undefined}\n",
        "id: example\nlanguage: Markdown\nfiles: ['[']\nrule: {kind: paragraph}\n",
        "id: quoted\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("rules")).unwrap();
        std::fs::write(dir.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
        std::fs::write(dir.path().join("rules/invalid.yml"), rule).unwrap();
        assert!(
            load_evidence_project(&dir.path().join("sgconfig.yml")).is_err(),
            "{rule}"
        );
    }
}

#[test]
fn yaml_parser_refusals_preserve_the_vendors_line_and_column() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("rules")).unwrap();
    std::fs::write(directory.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        directory.path().join("rules/broken.yml"),
        "id: broken\nlanguage: Markdown\nrule:\n  kind: [\n",
    )
    .unwrap();
    let error = load_evidence_project(&directory.path().join("sgconfig.yml"))
        .err()
        .unwrap();
    assert!(error.path.ends_with("rules/broken.yml"));
    assert!(error.line >= 4);
    assert!(error.column.is_some());
}

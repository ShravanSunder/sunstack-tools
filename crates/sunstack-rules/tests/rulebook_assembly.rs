#![allow(clippy::unwrap_used)]
use sunstack_evidence::*;
use sunstack_rules::*;
mod fixture {
    use super::*;
    pub fn fixture() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../sunstack/tests/fixtures/rulebook-lint")
    }
    pub fn project() -> EvidenceProject {
        load_evidence_project(&fixture().join("evidence/sgconfig.yml")).unwrap()
    }
    pub fn kinds() -> std::collections::BTreeMap<EvidenceKindName, EvidenceKindDefinition> {
        let value: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(fixture().join("skill-rules.config.json")).unwrap(),
        )
        .unwrap();
        serde_json::from_value(value["evidence"]["kinds"].clone()).unwrap()
    }
    pub fn entry_json(marker: &str) -> serde_json::Value {
        let value: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(fixture().join("rulebook/steering.rules.json")).unwrap(),
        )
        .unwrap();
        value[marker].clone()
    }
}

#[test]
fn bad_entry_is_skipped_while_peers_keep_their_bound_visible_standards() {
    let directory = tempfile::tempdir().unwrap();
    let markdown = directory.path().join("rules.md");
    let rules = directory.path().join("rules.json");
    std::fs::copy(fixture::fixture().join("rulebook/steering.md"), &markdown).unwrap();
    let mut entries: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture::fixture().join("rulebook/steering.rules.json")).unwrap(),
    )
    .unwrap();
    entries["K002"]["evidence"] = serde_json::json!("undefined");
    std::fs::write(&rules, serde_json::to_string(&entries).unwrap()).unwrap();
    let result = assemble_rulebook(
        &[RulebookPair { markdown, rules }],
        &MarkerPrefix::new("K").unwrap(),
        400,
        &fixture::kinds(),
        &fixture::project(),
    )
    .unwrap();
    assert_eq!(result.rules.len(), 2);
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(result.skipped[0].key, "K002");
    assert!(
        result
            .rules
            .iter()
            .all(|r| !r.sentence.standard.contains("<!--") && !r.sentence.standard.is_empty())
    );
}

#[test]
fn marker_entry_pairing_is_checked_both_ways_and_duplicate_names_skip_both() {
    let directory = tempfile::tempdir().unwrap();
    let markdown = directory.path().join("rules.md");
    let rules = directory.path().join("rules.json");
    std::fs::write(
        &markdown,
        "First standard. <!-- K001 -->\n\nMissing entry. <!-- K004 -->\n",
    )
    .unwrap();
    let entries =
        serde_json::json!({"K001":fixture::entry_json("K001"),"K002":fixture::entry_json("K002")});
    std::fs::write(&rules, serde_json::to_string(&entries).unwrap()).unwrap();
    let pair = RulebookPair {
        markdown: markdown.clone(),
        rules: rules.clone(),
    };
    let result = assemble_rulebook(
        std::slice::from_ref(&pair),
        &MarkerPrefix::new("K").unwrap(),
        400,
        &fixture::kinds(),
        &fixture::project(),
    )
    .unwrap();
    assert_eq!(result.rules.len(), 1);
    assert_eq!(result.skipped.len(), 2);
    std::fs::write(
        &markdown,
        "First standard. <!-- K001 -->\n\nSecond standard. <!-- K002 -->\n",
    )
    .unwrap();
    let mut first = fixture::entry_json("K001");
    first["name"] = serde_json::json!("same");
    std::fs::write(
        &rules,
        serde_json::to_string(&serde_json::json!({"K001":first,"K002":first})).unwrap(),
    )
    .unwrap();
    let result = assemble_rulebook(
        &[pair],
        &MarkerPrefix::new("K").unwrap(),
        400,
        &fixture::kinds(),
        &fixture::project(),
    )
    .unwrap();
    assert!(result.rules.is_empty());
    assert_eq!(result.skipped.len(), 2);
}

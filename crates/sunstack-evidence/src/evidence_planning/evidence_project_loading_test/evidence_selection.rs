#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use std::path::PathBuf;
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../sunstack/tests/fixtures/rulebook-lint/selection")
}
#[test]
fn numbered_h2_stages_are_cut_by_the_embedded_util() {
    let root = fixture();
    let project = load_evidence_project(&root.join("evidence/sgconfig.yml")).unwrap();
    let text = std::fs::read_to_string(root.join("skills/spec-program-review/SKILL.md")).unwrap();
    let index = project.match_file(std::path::Path::new("SKILL.md"), &text, 10000);
    let stages = index.by_rule[&EvidenceRuleId::new("step").unwrap()]
        .iter()
        .filter(|range| text[range.start..range.end].starts_with("## "))
        .count();
    assert_eq!(stages, 9);
}

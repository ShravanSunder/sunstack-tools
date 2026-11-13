#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

#[path = "native_unit_fields_test/heading_text.rs"]
mod heading_text;

const MARKDOWN: &str = "# Outer\n\nbefore <mark>literal</mark>\n\n- lead\n\n  ### Inner\n\n  body\n\nAfter the list.\n\n# Peer\nend\n";

fn markdown_catalog(max_matches: usize) -> EvidenceCatalog {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/whole.yml"),
        "id: whole\nlanguage: Markdown\nrule: {kind: document}\n",
    )
    .unwrap();
    std::fs::write(root.path().join("page.md"), MARKDOWN).unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    // Native kind and AST rule deliberately share a name, as with HTML units.
    let kinds = serde_json::from_value(json!({
        "whole":{"markdown_units":{"mode":"unit","units":["section"]},"fields":{
            "full":{"from":"section","text":"full"},
            "own":{"from":"section","text":"own"},
            "parent":{"from":"section","text":"enclosing_own"}
        }},
        "ast":{"rule":"whole","fields":{"line":{"from":"first_line"}}}
    }))
    .unwrap();
    crate::build_pieces(
        &project,
        &kinds,
        &["page.md".into()],
        &mut crate::FilesystemCheckedSource::new(root.path()).unwrap(),
        PieceBuildLimits {
            max_files: 10,
            max_file_bytes: 10000,
            max_total_bytes: 100000,
            max_matches,
            evidence_max_chars: 6000,
        },
    )
}

#[test]
fn markdown_section_fields_preserve_discontiguous_own_and_parent_spans() {
    let catalog = markdown_catalog(100);
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    let pieces = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "whole")
        .collect::<Vec<_>>();
    assert_eq!(pieces.len(), 3);
    let inner = MARKDOWN.find("### Inner").unwrap();
    let tail = MARKDOWN.find("After the list.").unwrap();
    let peer = MARKDOWN.find("# Peer").unwrap();
    let expected_own = format!("{}{}", &MARKDOWN[..inner], &MARKDOWN[tail..peer]);
    let outer = serde_json::to_value(&pieces[0].fields).unwrap();
    assert_eq!(outer["full"]["value"], MARKDOWN[..peer]);
    assert_eq!(outer["own"]["value"], expected_own);
    assert_eq!(outer["own"]["spans"].as_array().unwrap().len(), 2);
    assert_eq!(outer["own"]["spans"][0]["byte_start"], 0);
    assert_eq!(outer["own"]["spans"][0]["byte_end"], inner);
    assert_eq!(outer["own"]["spans"][1]["byte_start"], tail);
    assert_eq!(outer["own"]["spans"][1]["byte_end"], peer);
    assert_eq!(outer["parent"]["state"], "absent");
    let child = serde_json::to_value(&pieces[1].fields).unwrap();
    assert_eq!(child["full"]["value"], MARKDOWN[inner..tail]);
    assert_eq!(child["parent"], outer["own"]);
    let view = serde_json::to_value(&pieces[0].model_view).unwrap();
    assert_eq!(
        view["fields"]["own"], expected_own,
        "Markdown asking text must retain literal HTML"
    );
    assert_eq!(
        catalog.matches[&PathBuf::from("page.md")].by_rule
            [&crate::EvidenceRuleId::new("whole").unwrap()]
            .len(),
        1
    );
}

#[test]
fn markdown_and_ast_unit_matches_share_one_allowance() {
    let limited = markdown_catalog(1);
    assert!(
        limited
            .failures
            .iter()
            .any(|failure| failure.reason == "max_matches")
    );
    assert_eq!(
        limited
            .pieces
            .iter()
            .filter(|piece| piece.kind.as_str() == "whole")
            .count(),
        0
    );
    let enough = markdown_catalog(4);
    assert!(enough.failures.is_empty(), "{:?}", enough.failures);
    assert_eq!(
        enough
            .pieces
            .iter()
            .filter(|piece| piece.kind.as_str() == "whole")
            .count(),
        3
    );
}

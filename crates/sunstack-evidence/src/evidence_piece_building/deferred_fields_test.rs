#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn mixed_deferrals_keep_range_reasons_and_withhold_unestablished_children() {
    // Arrange two unresolved selected parents and independent, already-known subjects.
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(
        root.path().join("rules/elements.yml"),
        "id: parent\nlanguage: Html\nrule: {kind: element, regex: '^<section'}\n---\nid: child\nlanguage: Html\nrule: {kind: element, regex: '^<p'}\n---\nid: observer\nlanguage: Html\nrule: {kind: element, regex: '^<aside'}\n",
    )
    .unwrap();
    let text =
        "<section><p>alpha</p></section><section><p>beta</p></section><aside>Standalone</aside>";
    std::fs::write(root.path().join("page.html"), text).unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let kinds = serde_json::from_value(json!({
        "parent": {"rule":"parent", "fields":{"body":{"from":"remainder"}},
            "select":{"question":{},"yes_at_or_above":0.7,"no_at_or_below":0.3}},
        "child": {"rule":"child", "within":["parent"], "fields":{"body":{"from":"remainder"}}},
        "holder": {"rule":"parent", "fields":{
            "children":{"from":"inside","kind":"child"},
            "tokens":{"from":"captures","of":"children","regex":"([a-z]+)"},
            "body":{"from":"remainder"}
        }},
        "observer": {"rule":"observer", "fields":{
            "children":{"from":"same_file","kind":"child"},
            "body":{"from":"remainder"}
        }}
    }))
    .unwrap();
    let mut source = crate::FilesystemCheckedSource::new(root.path()).unwrap();
    let mut build = EvidenceBuild::start(
        &project,
        &kinds,
        &["page.html".into()],
        &mut source,
        PieceBuildLimits {
            max_files: 10,
            max_file_bytes: 10_000,
            max_total_bytes: 100_000,
            max_matches: 100,
            evidence_max_chars: 6000,
        },
    )
    .unwrap();
    let BuildStep::NeedsSelection {
        kind, candidates, ..
    } = build.advance(ChosenCandidates::default(), &mut source)
    else {
        panic!("parent selection required")
    };
    assert_eq!(kind.as_str(), "parent");
    assert_eq!(candidates.len(), 2);
    let deferrals = BTreeMap::from([
        (candidates[0].id.clone(), SelectionDeferral::Pending),
        (candidates[1].id.clone(), SelectionDeferral::Unavailable),
    ]);

    // Act: withholding selected parents also withholds their unestablished children.
    let scopes = build.defer_selection(&deferrals);
    let BuildStep::Done(catalog) = build.advance(ChosenCandidates::default(), &mut source) else {
        panic!("all independent work must finish")
    };

    // Assert range-local reasons, transitive propagation, and independent raw content.
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    assert_eq!(
        scopes
            .iter()
            .map(|scope| (scope.kind.as_str(), scope.file.as_path()))
            .collect::<BTreeSet<_>>(),
        ["parent", "child", "holder", "observer"]
            .into_iter()
            .map(|kind| (kind, Path::new("page.html")))
            .collect()
    );
    assert_eq!(scopes.len(), 4);
    assert!(
        catalog
            .pieces
            .iter()
            .all(|piece| !["parent", "child"].contains(&piece.kind.as_str()))
    );
    let holders = catalog
        .pieces
        .iter()
        .filter(|piece| piece.kind.as_str() == "holder")
        .collect::<Vec<_>>();
    assert_eq!(holders.len(), 2);
    for (holder, reason, view) in [
        (
            holders[0],
            FieldUnavailable::SelectionPending,
            crate::ModelView::SelectionPending,
        ),
        (
            holders[1],
            FieldUnavailable::SelectionUnavailable,
            crate::ModelView::SelectionUnavailable,
        ),
    ] {
        for name in ["children", "tokens", "body"] {
            assert_eq!(
                holder.fields[&FieldName::new(name).unwrap()],
                FieldValue::Unavailable { reason }
            );
        }
        assert_eq!(holder.model_view, view);
    }
    let observer = catalog
        .pieces
        .iter()
        .find(|piece| piece.kind.as_str() == "observer")
        .unwrap();
    assert_eq!(
        observer.fields[&FieldName::new("children").unwrap()],
        FieldValue::Unavailable {
            reason: FieldUnavailable::SelectionUnavailable
        }
    );
    assert!(
        matches!(&observer.fields[&FieldName::new("body").unwrap()], FieldValue::Text { value, .. } if value == "<aside>Standalone</aside>")
    );
}

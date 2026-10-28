#![allow(clippy::unwrap_used)]
#[path = "support/context_page_assertions.rs"]
mod assertions;
#[path = "support/context_page_fixture.rs"]
mod fixture;
use fixture::*;
use sunstack_evidence::*;
use sunstack_rules::{CandidateContextView, ContextSettings, context_pages::project_context_pages};

#[test]
fn readable_pages_resolve_to_the_lossless_native_bundle_and_render_deterministically() {
    let input = unit(
        "input",
        "fn input() { target(); }".into(),
        SourceSide::After,
    );
    let target = unit("target", "fn target() {}".into(), SourceSide::After);
    let mut source = context(vec![input.clone(), target.clone()]);
    source.related.push(relation(
        "edge",
        &input,
        &target,
        vec![RelationshipUncertainty::SyntaxOnly],
    ));
    let original = serde_json::to_value(&source).unwrap();
    let projection =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    let again =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    assert_eq!(projection.pages.len(), 1);
    assert_eq!(projection.pages[0].text, again.pages[0].text);
    assert_eq!(
        projection.pages[0].content_identity,
        again.pages[0].content_identity
    );
    assert!(!projection.pages[0].text.contains("\"snapshot\""));
    assert!(!projection.pages[0].text.contains("\"content_identity\""));
    let view = CandidateContextView {
        context: source,
        labels: projection.labels,
        pages: projection.pages,
        coverage: projection.coverage,
    };
    let wire = serde_json::to_value(&view).unwrap();
    assert_eq!(wire["context"], original);
    let recovered: CandidateContextView = serde_json::from_value(wire).unwrap();
    assert_eq!(recovered.context.units[0], input);
    assert_eq!(recovered.context.units[1], target);
    assertions::check_bundle_pages(&recovered).unwrap();
}

#[test]
fn orphan_and_dangling_page_pieces_fail_the_bundle_integrity_check() {
    let source = context(vec![unit(
        "input",
        "fn input() {}".into(),
        SourceSide::After,
    )]);
    let projection =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    let view = CandidateContextView {
        context: source,
        labels: projection.labels,
        pages: projection.pages,
        coverage: projection.coverage,
    };
    assertions::check_bundle_pages(&view).unwrap();
    let mut orphan = view.clone();
    orphan.pages[0].holds.push(PieceId::new("orphan").unwrap());
    assert_eq!(
        assertions::check_bundle_pages(&orphan).unwrap_err(),
        "orphan or duplicate page piece"
    );
    let mut dangling = view;
    dangling.pages[0].text = dangling.pages[0]
        .text
        .replacen("piece [input]", "piece [unknown]", 1);
    assert_eq!(
        assertions::check_bundle_pages(&dangling).unwrap_err(),
        "dangling or ambiguous piece"
    );
}

#[test]
fn abbreviated_piece_ids_disambiguate_native_suffix_collisions() {
    let source = context(vec![
        unit(
            "aaaaaaaa00000001",
            "fn first() {}".into(),
            SourceSide::After,
        ),
        unit(
            "bbbbbbbb00000001",
            "fn second() {}".into(),
            SourceSide::After,
        ),
    ]);
    let projection =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    assert!(projection.pages[0].text.contains("piece […a00000001]"));
    assert!(projection.pages[0].text.contains("piece […b00000001]"));
    assertions::check_bundle_pages(&CandidateContextView {
        context: source,
        labels: projection.labels,
        pages: projection.pages,
        coverage: projection.coverage,
    })
    .unwrap();
}

#[test]
fn truncated_unit_repacks_with_its_small_related_context_on_one_page() {
    let large = large_function("large", "HEAD", "body");
    let target = unit(
        "target",
        "fn target() { SMALL_RELATED }".into(),
        SourceSide::After,
    );
    let mut source = context(vec![large.clone(), target.clone()]);
    source.related.push(relation(
        "related",
        &large,
        &target,
        vec![RelationshipUncertainty::SyntaxOnly],
    ));
    let projection = project_context_pages(&source, &truncation(3, 2), &[], scalar_cost).unwrap();
    assert_eq!(projection.pages.len(), 1);
    assert!(projection.pages[0].text.contains("SMALL_RELATED"));
    assert!(projection.pages[0].text.contains("… 99 lines truncated …"));
    assert!(scalar_cost(&projection.pages[0]).unwrap() <= 20_000);
}

fn single_piece_view() -> CandidateContextView {
    let source = context(vec![unit(
        "input",
        "fn input() {}".into(),
        SourceSide::After,
    )]);
    let projection =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    CandidateContextView {
        context: source,
        labels: projection.labels,
        pages: projection.pages,
        coverage: projection.coverage,
    }
}
#[test]
fn checker_rejects_missing_and_duplicate_piece_labels() {
    let view = single_piece_view();
    let label = view.pages[0]
        .text
        .lines()
        .find(|line| line.starts_with("label: "))
        .unwrap()
        .to_owned()
        + "\n";
    let mut missing = view.clone();
    missing.pages[0].text = missing.pages[0].text.replacen(&label, "", 1);
    assert!(
        assertions::check_bundle_pages(&missing).is_err(),
        "missing label accepted"
    );
    let mut duplicate = view;
    duplicate.pages[0].text =
        duplicate.pages[0]
            .text
            .replacen(&label, &format!("{label}{label}"), 1);
    assert!(
        assertions::check_bundle_pages(&duplicate).is_err(),
        "duplicate label accepted"
    );
}
#[test]
fn checker_rejects_a_wrong_basis_with_the_correct_related_prefix() {
    let mut view = single_piece_view();
    view.pages[0].text = view.pages[0].text.replacen(
        "related — input unit input.rs",
        "related — input unit wrong.rs",
        1,
    );
    assert!(
        assertions::check_bundle_pages(&view).is_err(),
        "wrong same-prefix basis accepted"
    );
}
#[test]
fn checker_rejects_wrong_path_side_and_line_range_and_missing_locator() {
    let view = single_piece_view();
    for (from, to) in [
        ("anchor: input.rs", "anchor: wrong.rs"),
        (" [after]:", " [before]:"),
        (":10:1-", ":99:1-"),
    ] {
        let mut wrong = view.clone();
        wrong.pages[0].text = wrong.pages[0].text.replacen(from, to, 1);
        assert!(
            assertions::check_bundle_pages(&wrong).is_err(),
            "wrong locator accepted: {from}"
        );
    }
    let locator = view.pages[0]
        .text
        .lines()
        .find(|line| line.starts_with("anchor: "))
        .unwrap()
        .to_owned()
        + "\n";
    let mut missing = view;
    missing.pages[0].text = missing.pages[0].text.replacen(&locator, "", 1);
    assert!(
        assertions::check_bundle_pages(&missing).is_err(),
        "missing locator accepted"
    );
}

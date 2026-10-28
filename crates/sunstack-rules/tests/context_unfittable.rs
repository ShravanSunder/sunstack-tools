#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/context_page_fixture.rs"]
mod fixture;
use fixture::*;
use serde_json::json;
use sunstack_evidence::*;
use sunstack_rules::{ContextSettings, context_pages::project_context_pages};

// Wrong builds propagate a run-wide error, clip the mandatory signature, or count only its text.
#[test]
fn an_unfittable_mandatory_unit_excludes_the_complete_context_with_its_measured_minimum() {
    let signature = format!("fn impossible({})", "argument: i64, ".repeat(1500));
    assert!(signature.chars().count() > 20_000);
    let text = format!("{signature} {{\n    MATCH();\n    unreachable_tail();\n}}\n");
    let mut oversized = unit("oversized", text, SourceSide::After);
    oversized.unit.kind = SourceUnitKind::Function {
        signature: signature.clone(),
    };
    let ordinary = unit(
        "ordinary",
        "fn ordinary() { MATCH(); }\n".into(),
        SourceSide::After,
    );
    let context = context(vec![ordinary, oversized.clone()]);
    let mut last_over_cap = None;
    let projected = project_context_pages(&context, &ContextSettings::default(), &[], |page| {
        let count = scalar_cost(page)?;
        if count > 20_000 {
            last_over_cap = Some((count, page.text.clone()));
        }
        Ok(count)
    })
    .unwrap();
    assert!(projected.pages.is_empty());
    let (required, minimum) = last_over_cap.unwrap();
    assert!(minimum.contains(&format!("signature: {signature}\n")));
    assert!(minimum.contains("… 2 lines truncated …"));
    assert!(minimum.contains(&format!("{signature} {{\n")));
    assert!(minimum.contains("}\n"));
    assert!(required > signature.chars().count());
    assert_eq!(projected.coverage.len(), context.units.len() + 1);
    assert_eq!(projected.labels.len(), projected.coverage.len());
    let expected_pieces =
        std::collections::BTreeSet::from(["ordinary", "oversized", "match:context"]);
    assert_eq!(
        projected
            .coverage
            .iter()
            .map(|piece| piece.piece_id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        expected_pieces
    );
    assert_eq!(
        projected
            .labels
            .keys()
            .map(PieceId::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        expected_pieces
    );
    for piece in projected.coverage {
        let wire = serde_json::to_value(piece).unwrap();
        assert_eq!(
            wire["disposition"],
            json!({"kind":"excluded","reason":{"kind":"over_page_budget","unit":oversized.piece_id,"required_chars":required}})
        );
        assert_eq!(wire["redacted"], 0);
    }
}

#[test]
fn over_page_budget_coverage_has_the_same_closed_native_and_published_shape() {
    use sunstack_rules::ContextExclusionReason;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/specs/2026-10-27-session-ingestion-and-trees/schemas/evaluate-candidates-outcome.schema.json");
    let published: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let published = jsonschema::options()
        .offline()
        .build(&json!({"$defs":published["$defs"],"$ref":"#/$defs/ContextExclusionReason"}))
        .unwrap();
    let native = jsonschema::options()
        .offline()
        .build(&serde_json::to_value(schemars::schema_for!(ContextExclusionReason)).unwrap())
        .unwrap();
    let valid = json!({"kind":"over_page_budget","unit":"blocking_unit","required_chars":24001});
    assert!(published.is_valid(&valid));
    assert!(native.is_valid(&valid));
    let decoded: ContextExclusionReason = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), valid);
    for invalid in [
        json!({"kind":"over_page_budget","required_chars":24001}),
        json!({"kind":"over_page_budget","unit":"blocking_unit","required_chars":-1}),
        json!({"kind":"over_page_budget","unit":"blocking_unit","required_chars":24001,"unexpected":true}),
    ] {
        assert!(!published.is_valid(&invalid));
        assert!(!native.is_valid(&invalid));
        assert!(serde_json::from_value::<ContextExclusionReason>(invalid).is_err());
    }
}

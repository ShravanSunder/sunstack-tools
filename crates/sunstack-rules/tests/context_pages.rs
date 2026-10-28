#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../src/context_pages.rs"]
mod context_pages;
use context_pages::*;
#[path = "support/context_page_fixture.rs"]
mod context_page_fixture;
use context_page_fixture::*;
use serde_json::json;
use std::collections::BTreeSet;
use sunstack_evidence::*;
use sunstack_rules::{
    ContextExclusionReason, ContextPage, ContextPieceCoverage, ContextPieceDisposition,
    ContextProjection, ContextSettings, DiagnosticList, OmittedSourceLines, RelatedBasis,
    RelatednessLabel, RelationshipFactRef, RuleDiagnosticCode, UncertaintyReason,
};

#[test]
fn native_syntax_only_is_probable_but_qualified_and_unresolved_edges_are_uncertain() {
    let input = unit("input", "fn input() { MATCH }".into(), SourceSide::After);
    let probable = unit("probable", "fn probable() {}".into(), SourceSide::After);
    let ambiguous = unit("ambiguous", "fn ambiguous() {}".into(), SourceSide::After);
    let mut source = context(vec![input.clone(), probable.clone(), ambiguous.clone()]);
    source.related = vec![
        relation(
            "edge-probable",
            &input,
            &probable,
            vec![RelationshipUncertainty::SyntaxOnly],
        ),
        relation(
            "edge-ambiguous",
            &input,
            &ambiguous,
            vec![
                RelationshipUncertainty::SyntaxOnly,
                RelationshipUncertainty::AmbiguousDeclaration,
            ],
        ),
        RelatedPiece::Unresolved {
            piece_id: PieceId::new("edge-missing").unwrap(),
            seed: input.unit.key.clone(),
            connection: RelationDirection::Callers,
            name_as_written: "unknown_caller".into(),
            call_site: input.unit.span.clone(),
            reason: RelationshipUncertainty::MissingSource,
        },
    ];
    let projected =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    assert!(matches!(
        projected.labels[&input.piece_id],
        RelatednessLabel::Related { .. }
    ));
    assert!(matches!(
        projected.labels[&PieceId::new("edge-probable").unwrap()],
        RelatednessLabel::ProbablyRelated { .. }
    ));
    for id in ["edge-ambiguous", "edge-missing"] {
        assert!(matches!(
            projected.labels[&PieceId::new(id).unwrap()],
            RelatednessLabel::Uncertain { .. }
        ));
    }
    let missing = projected
        .coverage
        .iter()
        .find(|c| c.piece_id.as_str() == "edge-missing")
        .unwrap();
    assert!(matches!(
        missing.disposition,
        ContextPieceDisposition::NeverFound {
            reason: RelationshipUncertainty::MissingSource
        }
    ));
    let text = projected
        .pages
        .iter()
        .map(|page| page.text.as_str())
        .collect::<String>();
    assert!(text.contains("unknown_caller"));
    assert!(text.contains("call_site"));
    assert!(
        !projected
            .labels
            .values()
            .any(|label| matches!(label, RelatednessLabel::Unrelated { .. }))
    );
}
#[test]
fn both_revisions_hunks_imports_and_relationship_facts_survive_with_total_coverage() {
    let before = unit(
        "before",
        "fn before() { OLD_BODY }".into(),
        SourceSide::Before,
    );
    let after = unit("after", "fn after() { NEW_BODY }".into(), SourceSide::After);
    let caller = unit(
        "caller",
        "fn caller() { CALLER_BODY }".into(),
        SourceSide::After,
    );
    let mut source = context(vec![after.clone(), before.clone(), caller.clone()]);
    source.changes.push(ChangeHunk {
        hunk_id: HunkId::new("hunk-one").unwrap(),
        path: "after.rs".into(),
        before: Some(CodeExcerpt {
            span: before.unit.span.clone(),
            text: "OLD_DIFF".into(),
        }),
        after: Some(CodeExcerpt {
            span: after.unit.span.clone(),
            text: "NEW_DIFF".into(),
        }),
    });
    source.related.push(relation(
        "caller-edge",
        &after,
        &caller,
        vec![RelationshipUncertainty::SyntaxOnly],
    ));
    source.imports.push(CodeExcerpt {
        span: after.unit.span.clone(),
        text: "use import::MARKER;".into(),
    });
    let projection =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    let text = projection
        .pages
        .iter()
        .map(|page| page.text.as_str())
        .collect::<String>();
    for literal in [
        "OLD_DIFF",
        "NEW_DIFF",
        "OLD_BODY",
        "NEW_BODY",
        "CALLER_BODY",
        "use import::MARKER;",
        "static_call",
        "before",
        "after",
        &"a".repeat(40),
        &"b".repeat(40),
        "coverage",
    ] {
        assert!(text.contains(literal), "missing {literal}");
    }
    assert_eq!(projection.coverage.len(), 8);
    assert_eq!(projection.labels.len(), 8);
    assert_eq!(
        projection
            .coverage
            .iter()
            .map(|c| &c.piece_id)
            .collect::<BTreeSet<_>>()
            .len(),
        8
    );
}
#[test]
fn whole_functions_pack_once_with_exact_escaping_and_unicode_scalar_cost() {
    let first = unit(
        "first",
        format!("fn first() {{\n{}\n}}", "🦀\\\"".repeat(2400)),
        SourceSide::After,
    );
    let second = unit(
        "second",
        format!("fn second() {{\n{}\n}}", "🦀\\\"".repeat(2400)),
        SourceSide::After,
    );
    let source = context(vec![first.clone(), second.clone()]);
    let projected =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    assert!(projected.pages.len() >= 2);
    for page in &projected.pages {
        assert!(page.rendered_chars <= 20_000);
        assert_eq!(page.rendered_chars, scalar_cost(page).unwrap());
        assert!(
            page.text
                .contains(&format!("page {} of {}", page.number, page.of))
        );
        assert!(page.text.contains("other pages"));
    }
    for unit in [&first, &second] {
        assert_eq!(
            projected
                .pages
                .iter()
                .filter(|page| page.text.contains(&unit.text))
                .count(),
            1
        );
        assert_eq!(
            projected
                .pages
                .iter()
                .filter(|page| page.holds.contains(&unit.piece_id))
                .count(),
            1
        );
    }
}
#[test]
fn oversized_unit_keeps_signature_head_tail_exact_gap_and_delivered_redaction_count() {
    let big = large_function("large", "HEAD [redacted credential]", "line");
    let source = context(vec![big.clone()]);
    let settings = truncation(3, 2);
    let start = big.text.find("[redacted credential]").unwrap();
    let ranges = [CodeRedactionRange {
        part: CodeTextPart::UnitText {
            piece_id: big.piece_id.clone(),
        },
        byte_start: start,
        byte_end: start + "[redacted credential]".len(),
    }];
    let projected = project_context_pages(&source, &settings, &ranges, scalar_cost).unwrap();
    let page = projected
        .pages
        .iter()
        .find(|page| page.holds.contains(&big.piece_id))
        .unwrap();
    for literal in [
        "fn large() {",
        "HEAD [redacted credential]",
        "line001 ",
        "TAIL\n}",
        "… 99 lines truncated …",
    ] {
        assert!(page.text.contains(literal), "missing {literal}");
    }
    assert_eq!(page.text.matches("lines truncated").count(), 1);
    let coverage = projected
        .coverage
        .iter()
        .find(|item| item.piece_id == big.piece_id)
        .unwrap();
    assert!(matches!(
        coverage.disposition,
        ContextPieceDisposition::Truncated {
            omitted_lines: OmittedSourceLines {
                first: 13,
                last: 111
            },
            ..
        }
    ));
    assert_eq!(coverage.redacted, 1);
    assert!(page.rendered_chars <= 20_000);
}
#[test]
fn final_page_count_and_other_page_inventory_are_measured_before_returning() {
    let source = context(
        (0..12)
            .map(|n| {
                unit(
                    &format!("unit{n}"),
                    format!("fn unit{n}() {{\n{}\n}}", "z".repeat(9000)),
                    SourceSide::After,
                )
            })
            .collect(),
    );
    let projection = project_context_pages(&source, &ContextSettings::default(), &[], |page| {
        Ok(scalar_cost(page)? + if page.of >= 10 { 9000 } else { 0 })
    })
    .unwrap();
    assert!(projection.pages.len() >= 10);
    for page in &projection.pages {
        assert_eq!(page.of, projection.pages.len());
        assert!(page.rendered_chars <= 20_000);
    }
    assert_eq!(projection.coverage.len(), 13);
}
#[test]
fn impossible_signature_and_callback_errors_are_returned() {
    let mut big = unit("huge", "x".repeat(25_000), SourceSide::After);
    big.unit.kind = SourceUnitKind::Function {
        signature: "S".repeat(21_000),
    };
    assert!(
        project_context_pages(
            &context(vec![big]),
            &ContextSettings::default(),
            &[],
            scalar_cost
        )
        .is_err()
    );
    let small = context(vec![unit(
        "small",
        "fn small() {}".into(),
        SourceSide::After,
    )]);
    let result = project_context_pages(&small, &ContextSettings::default(), &[], |_| {
        Err(DiagnosticList::rule(
            sunstack_rules::RuleDiagnosticCode::CallerAborted,
        ))
    });
    assert!(
        matches!(&result.unwrap_err().items[0],sunstack_rules::ReportDiagnostic::Rule(d) if d.code==sunstack_rules::RuleDiagnosticCode::CallerAborted)
    );
}
#[test]
fn settings_and_coverage_reject_undeclared_fields() {
    assert_eq!(ContextSettings::default().truncation.head_lines, 40);
    assert_eq!(ContextSettings::default().truncation.tail_lines, 20);
    assert!(
        serde_json::from_value::<ContextSettings>(
            json!({"truncation":{"head_lines":1,"tail_lines":2,"typo":3}})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ContextPieceDisposition>(json!({"kind":"sent","page":1,"typo":0}))
            .is_err()
    );
}

#[test]
fn actual_out_of_unit_input_gets_span_basis_without_an_invented_unit() {
    let mut source = context(vec![unit(
        "placeholder",
        "fn placeholder() {}".into(),
        SourceSide::After,
    )]);
    source.units.clear();
    let projected =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    assert_eq!(projected.labels.len(), 1);
    assert!(
        matches!(projected.labels.values().next().unwrap(),RelatednessLabel::Related {basis:RelatedBasis::InputSpan {span}} if span==&source.matched.span)
    );
    assert_eq!(projected.coverage.len(), 1);
}
#[test]
fn recovered_contradicted_and_zero_fact_edges_remain_uncertain_with_source() {
    let seed = unit("seed", "fn seed() {}".into(), SourceSide::After);
    let mut target = unit(
        "recovered",
        "fn recovered() { PRESENT_SOURCE }".into(),
        SourceSide::After,
    );
    target.unit.syntax_quality = SyntaxQuality::RecoveredWithErrors;
    let mut source = context(vec![seed.clone(), target.clone()]);
    source.related = vec![
        relation(
            "recovered-edge",
            &seed,
            &target,
            vec![RelationshipUncertainty::SyntaxOnly],
        ),
        relation(
            "contradicted-edge",
            &seed,
            &target,
            vec![RelationshipUncertainty::SyntaxOnly],
        ),
        relation("zero-fact-edge", &seed, &target, vec![]),
    ];
    if let RelatedPiece::Resolved { contradictions, .. } = &mut source.related[1] {
        contradictions.push(RelationshipContradiction::LexicalShadow);
    }
    if let RelatedPiece::Resolved { facts, .. } = &mut source.related[2] {
        facts.clear();
    }
    let projected =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    for id in [
        "recovered-edge",
        "contradicted-edge",
        "zero-fact-edge",
        "recovered",
    ] {
        assert!(matches!(
            projected.labels[&PieceId::new(id).unwrap()],
            RelatednessLabel::Uncertain { .. }
        ));
    }
    assert_eq!(
        projected
            .pages
            .iter()
            .filter(|page| page.text.contains("PRESENT_SOURCE"))
            .count(),
        1
    );
    let text = projected
        .pages
        .iter()
        .map(|page| page.text.as_str())
        .collect::<String>();
    for reason in ["recovered_syntax", "lexical_shadow", "missing_source"] {
        assert!(text.contains(reason));
    }
}
#[test]
fn duplicate_unit_source_is_delivered_once_and_exclusion_names_the_retained_piece() {
    let original = unit(
        "original",
        "fn original() { SOURCE_ONCE }".into(),
        SourceSide::After,
    );
    let mut duplicate = original.clone();
    duplicate.piece_id = PieceId::new("zduplicate").unwrap();
    let projected = project_context_pages(
        &context(vec![original, duplicate]),
        &ContextSettings::default(),
        &[],
        scalar_cost,
    )
    .unwrap();
    assert_eq!(
        projected
            .pages
            .iter()
            .map(|page| page.text.matches("SOURCE_ONCE").count())
            .sum::<usize>(),
        1
    );
    assert!(
        matches!(&projected.coverage.iter().find(|item|item.piece_id.as_str()=="zduplicate").unwrap().disposition,
        ContextPieceDisposition::Excluded {reason:ContextExclusionReason::DuplicateUnit {retained}} if retained.as_str()=="original")
    );
}
#[test]
fn unicode_scalars_do_not_spend_the_utf8_byte_count() {
    let unicode = unit(
        "unicode",
        format!("fn unicode() {{\n{}\n}}", "🦀".repeat(12_000)),
        SourceSide::After,
    );
    let projection = project_context_pages(
        &context(vec![unicode.clone()]),
        &ContextSettings::default(),
        &[],
        scalar_cost,
    )
    .unwrap();
    assert_eq!(
        projection
            .pages
            .iter()
            .filter(|page| page.text.contains(&unicode.text))
            .count(),
        1
    );
    assert!(
        !projection
            .pages
            .iter()
            .any(|page| page.text.contains("lines truncated"))
    );
}
#[test]
fn head_and_tail_are_reduced_against_the_callback_without_dropping_the_signature() {
    let large = large_function("shrinking", "HEAD", "line");
    let settings = truncation(3, 2);
    let projection = project_context_pages(&context(vec![large]), &settings, &[], |page| {
        Ok(scalar_cost(page)?
            + if page.text.contains("line001") {
                20_000
            } else {
                0
            })
    })
    .unwrap();
    let coverage = projection
        .coverage
        .iter()
        .find(|item| item.piece_id.as_str() == "shrinking")
        .unwrap();
    assert!(matches!(
        coverage.disposition,
        ContextPieceDisposition::Truncated {
            omitted_lines: OmittedSourceLines {
                first: 12,
                last: 111
            },
            ..
        }
    ));
    let text = projection
        .pages
        .iter()
        .map(|page| page.text.as_str())
        .collect::<String>();
    assert!(text.contains("fn shrinking() {\nHEAD\n… 100 lines truncated …\nTAIL\n}"));
}
#[test]
fn preexisting_redaction_marker_without_a_replacement_counts_zero() {
    let literal = unit(
        "literal",
        "fn literal() { \"[redacted credential]\"; }".into(),
        SourceSide::After,
    );
    let projection = project_context_pages(
        &context(vec![literal]),
        &ContextSettings::default(),
        &[],
        scalar_cost,
    )
    .unwrap();
    assert_eq!(
        projection
            .coverage
            .iter()
            .find(|item| item.piece_id.as_str() == "literal")
            .unwrap()
            .redacted,
        0
    );
}

#[test]
fn a_real_replacement_omitted_by_truncation_does_not_count_a_kept_literal_marker() {
    let body = (1..=100)
        .map(|n| {
            format!(
                "line{n:03} {} {}",
                "x".repeat(400),
                if n == 50 { "[redacted credential]" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let large = unit(
        "omitted",
        format!("fn omitted() {{\nLITERAL [redacted credential]\n{body}\nTAIL\n}}"),
        SourceSide::After,
    );
    let start = large.text.rfind("[redacted credential]").unwrap();
    let ranges = [CodeRedactionRange {
        part: CodeTextPart::UnitText {
            piece_id: large.piece_id.clone(),
        },
        byte_start: start,
        byte_end: start + "[redacted credential]".len(),
    }];
    let settings = truncation(2, 2);
    let projection =
        project_context_pages(&context(vec![large]), &settings, &ranges, scalar_cost).unwrap();
    let piece = projection
        .coverage
        .iter()
        .find(|piece| piece.piece_id.as_str() == "omitted")
        .unwrap();
    assert_eq!(piece.redacted, 0);
    assert!(matches!(
        piece.disposition,
        ContextPieceDisposition::Truncated { .. }
    ));
    assert!(
        projection
            .pages
            .iter()
            .any(|page| page.text.contains("LITERAL [redacted credential]"))
    );
}
#[test]
fn signature_header_and_body_count_the_same_actual_replacement_once() {
    let signature = "fn signature(value: &str = \"[redacted credential]\") {";
    let mut source_unit = unit(
        "signature",
        format!("{signature}\nBODY\n}}"),
        SourceSide::After,
    );
    source_unit.unit.kind = SourceUnitKind::Function {
        signature: signature.into(),
    };
    let start = signature.find("[redacted credential]").unwrap();
    let ranges = [
        CodeRedactionRange {
            part: CodeTextPart::UnitText {
                piece_id: source_unit.piece_id.clone(),
            },
            byte_start: start,
            byte_end: start + "[redacted credential]".len(),
        },
        CodeRedactionRange {
            part: CodeTextPart::UnitSignature {
                piece_id: source_unit.piece_id.clone(),
            },
            byte_start: start,
            byte_end: start + "[redacted credential]".len(),
        },
    ];
    let projection = project_context_pages(
        &context(vec![source_unit]),
        &ContextSettings::default(),
        &ranges,
        scalar_cost,
    )
    .unwrap();
    assert_eq!(
        projection
            .coverage
            .iter()
            .find(|piece| piece.piece_id.as_str() == "signature")
            .unwrap()
            .redacted,
        1
    );
    assert!(
        projection
            .pages
            .iter()
            .map(|page| page.text.matches("[redacted credential]").count())
            .sum::<usize>()
            >= 2
    );
}
#[test]
fn native_ranges_attribute_excerpts_and_hint_by_part_even_with_utf8_prefixes() {
    let mut source = context(vec![unit("seed", "fn seed() {}".into(), SourceSide::After)]);
    let marker = "🦀[redacted credential]";
    source.matched.text = marker.into();
    if let SourceRef::RepositoryFile { path, .. } = &mut source.matched.span.source {
        *path = "outside.rs".into();
    }
    source.changes.push(ChangeHunk {
        hunk_id: HunkId::new("hunk-redacted").unwrap(),
        path: "seed.rs".into(),
        before: None,
        after: Some(CodeExcerpt {
            span: source.matched.span.clone(),
            text: marker.into(),
        }),
    });
    source.imports.push(CodeExcerpt {
        span: source.matched.span.clone(),
        text: marker.into(),
    });
    source.related.push(RelatedPiece::Unresolved {
        piece_id: PieceId::new("redacted-hint").unwrap(),
        seed: source.units[0].unit.key.clone(),
        connection: RelationDirection::Callees,
        name_as_written: marker.into(),
        call_site: source.matched.span.clone(),
        reason: RelationshipUncertainty::MissingSource,
    });
    let ranges = [
        CodeTextPart::Match,
        CodeTextPart::Hunk {
            hunk_id: HunkId::new("hunk-redacted").unwrap(),
            side: SourceSide::After,
        },
        CodeTextPart::Import { position: 0 },
        CodeTextPart::Hint {
            piece_id: PieceId::new("redacted-hint").unwrap(),
        },
    ]
    .map(|part| CodeRedactionRange {
        part,
        byte_start: 4,
        byte_end: marker.len(),
    });
    let projection =
        project_context_pages(&source, &ContextSettings::default(), &ranges, scalar_cost).unwrap();
    assert_eq!(
        projection
            .coverage
            .iter()
            .filter(|piece| piece.redacted == 1)
            .count(),
        4
    );
    assert_eq!(
        projection
            .coverage
            .iter()
            .find(|piece| piece.piece_id.as_str() == "seed")
            .unwrap()
            .redacted,
        0
    );
}

#[test]
fn a_whole_function_match_is_an_anchor_into_one_source_rendering() {
    let function = unit(
        "whole_match",
        "fn whole_match() {\nBODY_MATCHED_ONCE\n}".into(),
        SourceSide::After,
    );
    let mut source = context(vec![function.clone()]);
    source.matched.text = function.text.clone();
    let projection =
        project_context_pages(&source, &ContextSettings::default(), &[], scalar_cost).unwrap();
    assert_eq!(
        projection
            .pages
            .iter()
            .map(|page| page.text.matches("BODY_MATCHED_ONCE").count())
            .sum::<usize>(),
        1
    );
    let match_id = PieceId::new("match:context").unwrap();
    let page = projection
        .pages
        .iter()
        .find(|page| page.holds.contains(&match_id))
        .unwrap();
    assert!(page.holds.contains(&function.piece_id));
    assert!(page.text.contains("match anchor"));
    assert!(
        matches!(projection.coverage.iter().find(|piece|piece.piece_id==match_id).unwrap().disposition,ContextPieceDisposition::Sent {page:number} if number==page.number)
    );
}
#[test]
fn a_match_wholly_inside_the_omitted_source_range_has_exact_truncated_coverage() {
    let function = large_function("omitted_match", "HEAD", "body");
    let mut source = context(vec![function]);
    positioned_match(&mut source, 20, 22); // source lines30..32, all omitted
    let settings = truncation(2, 2);
    let projection = project_context_pages(&source, &settings, &[], scalar_cost).unwrap();
    let match_piece = projection
        .coverage
        .iter()
        .find(|piece| piece.piece_id.as_str() == "match:context")
        .unwrap();
    assert!(matches!(
        match_piece.disposition,
        ContextPieceDisposition::Truncated {
            omitted_lines: OmittedSourceLines {
                first: 30,
                last: 32
            },
            ..
        }
    ));
    assert!(
        !projection
            .pages
            .iter()
            .any(|page| page.text.contains("body019"))
    );
}
#[test]
fn a_match_in_the_delivered_head_remains_sent_when_its_unit_is_truncated() {
    let function = large_function("head_match", "HEAD_MATCH", "body");
    let mut source = context(vec![function]);
    positioned_match(&mut source, 1, 1);
    let settings = truncation(2, 2);
    let projection = project_context_pages(&source, &settings, &[], scalar_cost).unwrap();
    let match_piece = projection
        .coverage
        .iter()
        .find(|piece| piece.piece_id.as_str() == "match:context")
        .unwrap();
    assert!(matches!(
        match_piece.disposition,
        ContextPieceDisposition::Sent { .. }
    ));
    assert_eq!(
        projection
            .pages
            .iter()
            .map(|page| page.text.matches("HEAD_MATCH").count())
            .sum::<usize>(),
        1
    );
}

#[test]
fn source_line_truncation_composes_with_actual_code_privacy_for_lf_and_crlf() {
    for newline in ["\n", "\r\n"] {
        let mut lines = vec![
            "fn composed() {".to_string(),
            "  let password =".into(),
            "    \"abcdefghijklmnop\";".into(),
            "HEAD literal [redacted credential]".into(),
        ];
        lines.extend((1..=100).map(|line| format!("body{line:03} {}", "x".repeat(400))));
        lines.extend(["TAIL".into(), "}".into()]);
        assert_eq!(lines.len(), 106); // source lines10..115
        let mut source = context(vec![unit(
            "composed",
            lines.join(newline),
            SourceSide::After,
        )]);
        positioned_match(&mut source, 3, 3); // source line13 remains in the delivered head
        let projected = project_code_context(&source, &BTreeSet::new()).unwrap();
        assert_eq!(projected.context.units[0].text.lines().count(), 106);
        assert!(
            projected.context.units[0]
                .text
                .contains(&format!("[redacted credential]{newline}"))
        );
        assert!(!projected.context.units[0].text.contains("abcdefghijklmnop"));
        let projection = project_context_pages(
            &projected.context,
            &truncation(4, 2),
            &projected.redaction_ranges,
            scalar_cost,
        )
        .unwrap();
        let unit_coverage = projection
            .coverage
            .iter()
            .find(|piece| piece.piece_id.as_str() == "composed")
            .unwrap();
        assert!(matches!(
            unit_coverage.disposition,
            ContextPieceDisposition::Truncated {
                omitted_lines: OmittedSourceLines {
                    first: 14,
                    last: 113
                },
                ..
            }
        ));
        assert_eq!(unit_coverage.redacted, 1);
        let match_coverage = projection
            .coverage
            .iter()
            .find(|piece| piece.piece_id.as_str() == "match:context")
            .unwrap();
        assert!(matches!(
            match_coverage.disposition,
            ContextPieceDisposition::Sent { .. }
        ));
        assert_eq!(match_coverage.redacted, 0);
        assert!(
            projection
                .pages
                .iter()
                .any(|page| page.text.contains("… 100 lines truncated …"))
        );
    }
}

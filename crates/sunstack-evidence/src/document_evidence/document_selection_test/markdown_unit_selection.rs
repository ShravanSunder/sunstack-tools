#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use serde_json::json;

fn document(text: &str) -> CapturedDocument {
    let now = chrono::Utc::now();
    CapturedDocument {
        interval: CaptureInterval {
            started_at: now,
            completed_at: now,
        },
        source: SourceRef::Document {
            repository: RepositoryId::new("fixture").unwrap(),
            path: "skill.md".into(),
            content_identity: content_digest(text.as_bytes()),
        },
        text: text.into(),
    }
}

fn selection(value: serde_json::Value) -> EvidenceSelection {
    serde_json::from_value(value).unwrap()
}

fn units(doc: &CapturedDocument, value: serde_json::Value) -> Vec<SelectedMarkdownUnit> {
    match select_document(doc, &selection(value), 100).unwrap() {
        SelectionContent::MarkdownUnits { units } => units,
        other => panic!("expected markdown units: {other:?}"),
    }
}

#[test]
fn unit_mode_keeps_overlapping_raw_blocks_and_terminal_newlines() {
    let doc = document("## Heading\n\nUse alpha.\n\n- Use beta.\n");
    let found = units(
        &doc,
        json!({"kind":"markdown_units","mode":"unit","units":["section","paragraph","list_item"]}),
    );
    assert_eq!(found.len(), 4);
    assert_eq!(found[0].kind, MarkdownUnitKind::Section);
    assert_eq!(found[0].text, doc.text.as_ref());
    assert!(found.iter().all(|unit| unit.text.ends_with('\n')));
    assert_eq!(
        found
            .iter()
            .filter(|unit| unit.kind == MarkdownUnitKind::Paragraph)
            .count(),
        2
    );
}

#[test]
fn three_calls_in_one_item_are_distinct_and_owned_once() {
    let doc = document("## Heading\n\n- Use alpha; Use beta; Use gamma.\n");
    let found = units(
        &doc,
        json!({"kind":"markdown_units","mode":"call_site","units":["section","paragraph","list_item"],"pattern":"Use"}),
    );
    assert_eq!(found.len(), 3);
    let mut starts = Vec::new();
    for unit in found {
        assert_eq!(unit.kind, MarkdownUnitKind::ListItem);
        let MarkdownOccurrence::CallSite {
            unit_span,
            match_span,
            visible_range,
        } = unit.occurrence
        else {
            panic!("expected call site")
        };
        assert_eq!(unit_span, unit.span);
        assert_eq!(&doc.text[match_span.byte_start..match_span.byte_end], "Use");
        assert_eq!(
            &unit.visible_text[visible_range.start..visible_range.end],
            "Use"
        );
        starts.push(match_span.byte_start);
    }
    assert_eq!(starts, vec![14, 25, 35]);
}

#[test]
fn call_offsets_cover_entities_escapes_and_crlf_with_character_coordinates() {
    let doc = document("Élan &NotEqualTilde; \\*\r\nnext\n");
    let found = units(
        &doc,
        json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"],"pattern":"[≂̸*\\n]"}),
    );
    assert_eq!(found.len(), 4);
    let calls = found
        .iter()
        .map(|unit| match &unit.occurrence {
            MarkdownOccurrence::CallSite {
                match_span,
                visible_range,
                ..
            } => (match_span, visible_range),
            _ => panic!("expected call site"),
        })
        .collect::<Vec<_>>();
    assert_eq!(calls[0].0, calls[1].0);
    assert_ne!(calls[0].1, calls[1].1);
    assert_eq!(calls[0].0.start_column, 6);
    assert_eq!(
        &doc.text[calls[0].0.byte_start..calls[0].0.byte_end],
        "&NotEqualTilde;"
    );
    assert_eq!(&doc.text[calls[2].0.byte_start..calls[2].0.byte_end], "\\*");
    assert_eq!(
        &doc.text[calls[3].0.byte_start..calls[3].0.byte_end],
        "\r\n"
    );
}

#[test]
fn zero_matches_and_occurrence_limits_are_real_selection_results() {
    let doc = document("- Use one; Use two; Use three.\n");
    let select = selection(
        json!({"kind":"markdown_units","mode":"call_site","units":["list_item"],"pattern":"Use"}),
    );
    assert_eq!(
        select_document(&doc, &select, 2).unwrap_err().code,
        EvidenceCode::SelectorLimit
    );
    assert!(units(&doc,json!({"kind":"markdown_units","mode":"call_site","units":["list_item"],"pattern":"Missing"})).is_empty());
}

#[test]
fn inline_code_is_hidden_and_fences_remain_in_a_selected_section() {
    let doc = document("## Heading\n\n`Use hidden` Use visible.\n\n```text\nUse fenced\n```\n");
    let found = units(
        &doc,
        json!({"kind":"markdown_units","mode":"call_site","units":["section"],"pattern":"Use"}),
    );
    assert_eq!(found.len(), 2);
    let starts = found
        .iter()
        .map(|unit| match &unit.occurrence {
            MarkdownOccurrence::CallSite { match_span, .. } => match_span.byte_start,
            _ => panic!("expected call site"),
        })
        .collect::<Vec<_>>();
    assert_eq!(starts, vec![25, 47]);
}

#[test]
fn generated_only_matches_use_boundary_anchors_without_collapsing_visible_identity() {
    let doc = document("## Heading\n\nFirst\n\nSecond\n\n| a |\n| --- |\n| b |\n");
    let found = units(
        &doc,
        json!({"kind":"markdown_units","mode":"call_site","units":["section"],"pattern":"\\n|\\|"}),
    );
    assert!(found.len() > 5);
    let mut visible = std::collections::BTreeSet::new();
    for unit in found {
        let MarkdownOccurrence::CallSite {
            match_span,
            visible_range,
            ..
        } = unit.occurrence
        else {
            panic!("expected call site")
        };
        assert_eq!(match_span.byte_start, match_span.byte_end);
        assert!(visible.insert((visible_range.start, visible_range.end)));
    }
}

#[test]
fn generated_separators_stay_in_the_section_when_requested_paragraphs_have_no_visible_match() {
    let doc = document("## Heading\n\nFirst\n\nSecond\n");
    let sections = units(
        &doc,
        json!({"kind":"markdown_units","mode":"call_site","units":["section"],"pattern":"\\n"}),
    );
    let overlapping = units(
        &doc,
        json!({"kind":"markdown_units","mode":"call_site","units":["section","paragraph"],"pattern":"\\n"}),
    );
    assert_eq!(sections.len(), 4);
    assert_eq!(
        overlapping, sections,
        "adding paragraphs cannot erase enclosing visible separators"
    );
    assert!(
        overlapping
            .iter()
            .all(|unit| unit.kind == MarkdownUnitKind::Section)
    );
    let ranges = overlapping
        .iter()
        .map(|unit| match &unit.occurrence {
            MarkdownOccurrence::CallSite { visible_range, .. } => {
                (visible_range.start, visible_range.end)
            }
            _ => panic!("expected call site"),
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ranges.len(), 4);
    let authored = units(
        &doc,
        json!({"kind":"markdown_units","mode":"call_site","units":["section","paragraph"],"pattern":"First|Second"}),
    );
    assert_eq!(authored.len(), 2);
    assert!(
        authored
            .iter()
            .all(|unit| unit.kind == MarkdownUnitKind::Paragraph)
    );
}

#[test]
fn a_paragraph_prefix_match_transfers_to_its_list_owner_at_the_owners_visible_range() {
    let doc = document("- Use alpha.\n");
    let found = units(
        &doc,
        json!({"kind":"markdown_units","mode":"call_site","units":["paragraph","list_item"],"starts_with":"Use"}),
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, MarkdownUnitKind::ListItem);
    let MarkdownOccurrence::CallSite {
        match_span,
        visible_range,
        ..
    } = &found[0].occurrence
    else {
        panic!("expected call site")
    };
    assert_eq!(&doc.text[match_span.byte_start..match_span.byte_end], "Use");
    assert_eq!((visible_range.start, visible_range.end), (2, 5));
    assert_eq!(
        &found[0].visible_text[visible_range.start..visible_range.end],
        "Use"
    );
}

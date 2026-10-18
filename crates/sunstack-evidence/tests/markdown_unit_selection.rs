#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::json;
use sunstack_evidence::*;

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
fn markdown_selector_wire_is_flat_strict_and_keeps_modes_distinct() {
    let schema = serde_json::to_value(schemars::schema_for!(EvidenceSelection)).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for value in [
        json!({"kind":"markdown_units", "mode":"unit", "units":["paragraph"]}),
        json!({"kind":"markdown_units", "mode":"unit", "units":["section"], "starts_with":"Use"}),
        json!({"kind":"markdown_units", "mode":"call_site", "units":["paragraph","list_item"], "pattern":"Use"}),
    ] {
        let parsed = serde_json::from_value::<EvidenceSelection>(value.clone()).unwrap();
        assert!(validator.is_valid(&value), "{value}");
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }
    for value in [
        json!({"kind":"markdown_units", "mode":"call_site", "units":["paragraph"]}),
        json!({"kind":"markdown_units", "mode":"unit", "units":["paragraph"], "pattern":"Use", "starts_with":"Use"}),
        json!({"kind":"markdown_units", "mode":"unit", "units":["paragraph"], "typo":true}),
        json!({"kind":"markdown_units", "mode":"call_site", "units":["paragraph"], "pattern":"Use", "typo":true}),
    ] {
        assert!(
            serde_json::from_value::<EvidenceSelection>(value.clone()).is_err(),
            "{value}"
        );
        assert!(!validator.is_valid(&value), "{value}\n{schema}");
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
fn empty_matching_filters_and_empty_kind_sets_are_refused_before_reads() {
    let sources = std::collections::BTreeMap::from([(
        SourceName::new("document").unwrap(),
        SourceSpec::Document {
            repository: RepositoryId::new("fixture").unwrap(),
            path: "missing.md".into(),
        },
    )]);
    for value in [
        json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"],"pattern":"\\b"}),
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"pattern":"a*"}),
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"],"pattern":"["}),
        json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"],"starts_with":""}),
        json!({"kind":"markdown_units","mode":"unit","units":[]}),
    ] {
        let declaration = EvidenceDeclaration {
            id: DeclarationId::new("target").unwrap(),
            input: SourceName::new("document").unwrap(),
            requirement: EvidenceRequirement::Required,
            select: selection(value),
            max_items: None,
            max_bytes: None,
        };
        assert_eq!(
            admit_evidence_declaration(&declaration, &sources)
                .unwrap_err()
                .code,
            EvidenceCode::InvalidSelector
        );
    }
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

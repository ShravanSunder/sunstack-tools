#![allow(clippy::unwrap_used)]
use super::*;

fn matched_file() -> MatchedFile {
    let text = "猫\nα section\nβ subject tail".to_owned();
    MatchedFile {
        source: SourceRef::Document {
            repository: RepositoryId::new("checked_repository").unwrap(),
            path: "page.html".into(),
            content_identity: content_digest(text.as_bytes()),
        },
        text,
        index: FileMatchIndex::default(),
        native_units: BTreeMap::new(),
    }
}

fn section_piece(file: &MatchedFile, start: usize, end: usize) -> EvidencePiece {
    let span = source_span(file, &MatchRange { start, end });
    EvidencePiece {
        kind: EvidenceKindName::new("configured_section").unwrap(),
        location: PieceLocation {
            file: "page.html".into(),
            first_line: span.start_line,
            last_line: span.end_line,
            span,
        },
        text: file.text[start..end].into(),
        fields: BTreeMap::new(),
        model_view: crate::ModelView::Bounded {
            fields: BTreeMap::new(),
            cut: vec![],
        },
    }
}

#[test]
fn section_prefix_uses_nearest_configured_same_version_and_each_subject_start() {
    let file = matched_file();
    let kind = EvidenceKindName::new("configured_section").unwrap();
    let mut pieces = vec![
        section_piece(&file, 0, file.text.len()),
        section_piece(&file, 4, file.text.len()),
    ];
    let mut unconfigured = section_piece(&file, 15, file.text.len());
    unconfigured.kind = EvidenceKindName::new("another_section").unwrap();
    pieces.push(unconfigured);
    let first = section_prefix_field(&file, &kind, &MatchRange { start: 15, end: 17 }, &pieces);
    let second = section_prefix_field(&file, &kind, &MatchRange { start: 18, end: 25 }, &pieces);

    assert_eq!(
        first,
        FieldValue::Text {
            value: "α section\n".into(),
            spans: vec![SourceSpan {
                source: file.source.clone(),
                byte_start: 4,
                byte_end: 15,
                start_line: 2,
                start_column: 1,
                end_line: 3,
                end_column: 1
            }],
        }
    );
    assert_eq!(
        second,
        FieldValue::Text {
            value: "α section\nβ ".into(),
            spans: vec![SourceSpan {
                source: file.source.clone(),
                byte_start: 4,
                byte_end: 18,
                start_line: 2,
                start_column: 1,
                end_line: 3,
                end_column: 3
            }],
        }
    );
}

#[test]
fn section_prefix_distinguishes_empty_missing_ambiguous_stale_and_limited() {
    let mut file = matched_file();
    let kind = EvidenceKindName::new("configured_section").unwrap();
    let subject = MatchRange { start: 4, end: 6 };
    let section = section_piece(&file, 4, file.text.len());
    let empty = section_prefix_field(&file, &kind, &subject, std::slice::from_ref(&section));
    assert!(
        matches!(empty, FieldValue::Text { value, spans } if value.is_empty() && spans.len() == 1 && spans[0].byte_start == 4 && spans[0].byte_end == 4)
    );
    let unavailable = FieldValue::Unavailable {
        reason: FieldUnavailable::Unreadable,
    };
    assert_eq!(
        section_prefix_field(&file, &kind, &subject, &[]),
        unavailable
    );
    let mut stale = section.clone();
    if let SourceRef::Document {
        content_identity, ..
    } = &mut stale.location.span.source
    {
        *content_identity = "old_version".into();
    }
    assert_eq!(
        section_prefix_field(&file, &kind, &subject, &[stale]),
        unavailable
    );
    let crossing = vec![section_piece(&file, 0, 17), section_piece(&file, 4, 25)];
    assert_eq!(
        section_prefix_field(&file, &kind, &subject, &crossing),
        unavailable
    );
    file.index.limited = true;
    assert_eq!(
        section_prefix_field(&file, &kind, &subject, &[section]),
        unavailable
    );
}

#[test]
fn section_prefix_schema_and_decoder_require_only_a_configured_kind() {
    let schema = schemars::schema_for!(FieldSource);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for (value, valid) in [
        (
            serde_json::json!({"from":"section_prefix","kind":"outline"}),
            true,
        ),
        (
            serde_json::json!({"from":"section_prefix","kind":""}),
            false,
        ),
        (
            serde_json::json!({"from":"section_prefix","kind":null}),
            false,
        ),
        (serde_json::json!({"from":"section_prefix"}), false),
        (
            serde_json::json!({"from":"section_prefix","rule":"section"}),
            false,
        ),
        (
            serde_json::json!({"from":"section_prefix","kind":"outline","text":"full"}),
            false,
        ),
    ] {
        // Identifier validation is owned by its constructor, beyond the generated string schema.
        if value["kind"] != "" {
            assert_eq!(validator.is_valid(&value), valid, "{value}");
        }
        assert_eq!(serde_json::from_value::<FieldSource>(value).is_ok(), valid);
    }
    let source = FieldSource::SectionPrefix {
        kind: EvidenceKindName::new("outline").unwrap(),
    };
    assert_eq!(source.shape(), crate::FieldShape::Text);
    assert_eq!(
        serde_json::to_value(&source).unwrap(),
        serde_json::json!({"from":"section_prefix","kind":"outline"})
    );
}

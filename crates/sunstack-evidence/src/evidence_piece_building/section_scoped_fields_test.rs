#![allow(clippy::unwrap_used)]
use super::*;

fn file() -> MatchedFile {
    MatchedFile {
        text: "x".repeat(100),
        source: SourceRef::Document {
            repository: RepositoryId::new("repo").unwrap(),
            path: "page.html".into(),
            content_identity: "current".into(),
        },
        index: FileMatchIndex::default(),
        native_units: BTreeMap::new(),
    }
}
fn piece(file: &MatchedFile, kind: &str, start: usize, end: usize) -> EvidencePiece {
    let span = source_span(file, &MatchRange { start, end });
    EvidencePiece {
        kind: EvidenceKindName::new(kind).unwrap(),
        location: PieceLocation {
            file: "page.html".into(),
            first_line: 1,
            last_line: 1,
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
fn scoped_inside_uses_nearest_same_source_section_and_orders_final_pieces() {
    let file = file();
    let mut stale = piece(&file, "picture", 22, 24);
    if let SourceRef::Document {
        content_identity, ..
    } = &mut stale.location.span.source
    {
        *content_identity = "old".into();
    }
    let pieces = vec![
        piece(&file, "outline", 0, 100),
        piece(&file, "outline", 10, 80),
        piece(&file, "picture", 45, 50),
        piece(&file, "picture", 15, 20),
        piece(&file, "picture", 2, 5),
        stale,
    ];
    let kind = EvidenceKindName::new("picture").unwrap();
    let outline = EvidenceKindName::new("outline").unwrap();
    let subject = MatchRange { start: 30, end: 35 };
    let before = scoped_inside_field(
        &file,
        &kind,
        &crate::SectionScope::Prefix {
            section_prefix: outline.clone(),
        },
        &subject,
        &pieces,
    );
    assert!(
        matches!(before,FieldValue::Texts{values,spans} if values==["xxxxx"] && spans[0].byte_start==15)
    );
    let whole = scoped_inside_field(
        &file,
        &kind,
        &crate::SectionScope::Section { section: outline },
        &subject,
        &pieces,
    );
    assert!(
        matches!(whole,FieldValue::Texts{values,spans} if values==["xxxxx","xxxxx"] && spans.iter().map(|s|s.byte_start).collect::<Vec<_>>()==[15,45])
    );
}

#[test]
fn scoped_inside_distinguishes_empty_from_missing_ambiguous_and_scan_limited() {
    let mut file = file();
    let kind = EvidenceKindName::new("picture").unwrap();
    let scope = crate::SectionScope::Prefix {
        section_prefix: EvidenceKindName::new("outline").unwrap(),
    };
    let subject = MatchRange { start: 30, end: 35 };
    let empty = scoped_inside_field(
        &file,
        &kind,
        &scope,
        &subject,
        &[piece(&file, "outline", 10, 80)],
    );
    assert!(
        matches!(empty,FieldValue::Texts{values,spans} if values.is_empty() && spans.is_empty())
    );
    let unavailable = FieldValue::Unavailable {
        reason: FieldUnavailable::Unreadable,
    };
    assert_eq!(
        scoped_inside_field(&file, &kind, &scope, &subject, &[]),
        unavailable
    );
    let crossing = vec![
        piece(&file, "outline", 0, 60),
        piece(&file, "outline", 10, 80),
    ];
    assert_eq!(
        scoped_inside_field(&file, &kind, &scope, &subject, &crossing),
        unavailable
    );
    let pieces = vec![piece(&file, "outline", 10, 80)];
    file.index.limited = true;
    assert_eq!(
        scoped_inside_field(&file, &kind, &scope, &subject, &pieces),
        unavailable
    );
}

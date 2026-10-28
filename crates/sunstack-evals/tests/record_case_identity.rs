#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use sunstack_evals::*;
use sunstack_evidence::*;
use support::*;

#[test]
fn kept_record_matches_frozen_record_case_despite_different_match_attribution() {
    let root = tempfile::tempdir().unwrap();
    let mut specification = input(root.path(), vec![case(1, Label::Yes, Split::HeldOut)]);
    let input_id = specification.sources[0].source.source_identity().unwrap();
    let events = vec![
        TranscriptEvent::MessagePart {
            message_id: RecordId::new("message_1").unwrap(),
            author: RecordAuthor::Assistant,
            position: SourcePosition {
                line: 1,
                byte_offset: 0,
            },
            blocks: vec![MessageBlock::Text {
                text: "preface".into(),
            }],
            injection: InjectionOrigin::NotInjected,
            private_kind: None,
        },
        TranscriptEvent::MessagePart {
            message_id: RecordId::new("message_1").unwrap(),
            author: RecordAuthor::Assistant,
            position: SourcePosition {
                line: 2,
                byte_offset: 100,
            },
            blocks: vec![MessageBlock::Text {
                text: "signal MATCH here".into(),
            }],
            injection: InjectionOrigin::NotInjected,
            private_kind: None,
        },
    ];
    let lines = events
        .into_iter()
        .map(|event| {
            let TranscriptEvent::MessagePart { position, .. } = &event else {
                unreachable!()
            };
            SourceLine {
                position: *position,
                legacy: LegacyDisposition::Folded,
                events: vec![event],
                parent: None,
            }
        })
        .collect::<Vec<_>>();
    let projected = project_positioned(&input_id, &lines).unwrap();
    assert_eq!(projected.records.len(), 1);
    let whole_record = projected.records.as_slice().discover(None, 0).unwrap();
    let pass = MechanicalPass {
        predicate: RecordPredicate::Literal {
            field: FieldPath::new("/body/blocks/1/block/text").unwrap(),
            text: "MATCH".into(),
        },
    };
    let kept = projected
        .records
        .as_slice()
        .discover(Some(&pass), 0)
        .unwrap();
    assert_eq!(whole_record.matches.len(), 1);
    assert_eq!(kept.matches.len(), 1);
    let frozen_anchor = whole_record.matches[0].occurrence.clone();
    let matched_anchor = kept.matches[0].occurrence.clone();
    match (&frozen_anchor, &matched_anchor) {
        (
            SourceOccurrence::Record {
                record: frozen_record,
                author: frozen_author,
                positions: frozen_positions,
                field: frozen_field,
                span: frozen_span,
                matched_at: frozen_at,
            },
            SourceOccurrence::Record {
                record: matched_record,
                author: matched_author,
                positions: matched_positions,
                field: matched_field,
                span: matched_span,
                matched_at,
            },
        ) => {
            assert_eq!(frozen_record, matched_record);
            assert_eq!(frozen_author, matched_author);
            assert_eq!(frozen_positions, matched_positions);
            assert_ne!(frozen_field, matched_field);
            assert_ne!(frozen_span, matched_span);
            assert_ne!(frozen_at, matched_at);
        }
    }
    assert_eq!(
        frozen_anchor.occurrence_identity().unwrap(),
        matched_anchor.occurrence_identity().unwrap()
    );

    let mut candidate = entry(
        1,
        json!({"kind":"finding","path":[]}),
        Some("passed"),
        Some(10),
    );
    candidate.occurrence = matched_anchor.clone();
    candidate.candidate_id.occurrence = candidate.occurrence.occurrence_identity().unwrap();
    specification.labels[0].source_anchor = frozen_anchor;
    let frozen = freeze_cases(&root.path().join("record-case"), &specification).unwrap();
    let mut observed = bundle(vec![candidate], Some(0.1), 20);
    observed.inputs.push(located_input(&frozen));

    // Exact-attribution control: the honest produced CandidateId is accepted and scores a finding.
    let mut exact_specification = specification.clone();
    exact_specification.labels[0].source_anchor = matched_anchor;
    let exact = freeze_cases(&root.path().join("exact-case"), &exact_specification).unwrap();
    let exact_report = score(&[observed.clone()], &exact).unwrap();
    assert_eq!(exact_report.filter.kept, 1);
    assert_eq!(
        exact_report.end_result.true_findings,
        vec![CaseId::new("case_1").unwrap()]
    );

    // E3/E6 identify this session occurrence by the record; E18 labels its source anchor.
    // Selector field/span/line attribution changed, while the kept physical record did not.
    let report = score(&[observed], &frozen).unwrap();
    assert_eq!(
        json!({"kept": report.filter.kept, "missed": report.filter.missed,
            "unknown": report.filter.unknown, "true_findings": report.end_result.true_findings,
            "inconclusive": report.end_result.inconclusive}),
        json!({"kept": 1, "missed": [], "unknown": [],
            "true_findings": ["case_1"], "inconclusive": []}),
        "the same kept record must not become a filter miss solely because match attribution changed",
    );
}

#![allow(dead_code, clippy::unwrap_used)]
use crate::*;
use std::{collections::BTreeMap, path::PathBuf};
use sunstack_evidence::*;

pub fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sunstack/tests/fixtures/rulebook-lint")
}
pub fn project() -> EvidenceProject {
    load_evidence_project(&fixture().join("evidence/sgconfig.yml")).unwrap()
}
pub fn kinds() -> BTreeMap<EvidenceKindName, EvidenceKindDefinition> {
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture().join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap()
}
pub fn entry(marker: &str) -> RuleEntryWire {
    serde_json::from_value(entry_json(marker)).unwrap()
}
pub fn entry_json(marker: &str) -> serde_json::Value {
    let entries: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture().join("rulebook/steering.rules.json")).unwrap(),
    )
    .unwrap();
    entries[marker].clone()
}
pub fn rule(marker: &str) -> RulebookRule {
    let entry = entry(marker);
    let graph = admit_rule_entry(&entry, &kinds(), &project()).unwrap();
    RulebookRule {
        name: entry.name,
        sentence: RulebookSentence {
            marker: MarkerId::new(marker).unwrap(),
            rulebook: "rulebook.md".into(),
            line: 1,
            standard: "A checkable completion covers its obligations.".into(),
        },
        kind: entry.evidence,
        questions: entry.questions,
        decision: graph,
        severity: entry.severity,
        related: entry.related,
    }
}
pub fn piece() -> EvidencePiece {
    let source = SourceRef::Document {
        repository: RepositoryId::new("fixture").unwrap(),
        path: "SKILL.md".into(),
        content_identity: "fixture".into(),
    };
    let span = |start, end, line| SourceSpan {
        source: source.clone(),
        byte_start: start,
        byte_end: end,
        start_line: line,
        start_column: 1,
        end_line: line,
        end_column: 1,
    };
    let fields = BTreeMap::from([
        (
            FieldName::new("heading").unwrap(),
            FieldValue::Text {
                value: "3. Choose each observation".into(),
                spans: vec![span(6114, 6144, 49)],
            },
        ),
        (
            FieldName::new("completion").unwrap(),
            FieldValue::Texts {
                values: vec!["Completion: ready.".into()],
                spans: vec![span(7801, 7908, 59)],
            },
        ),
        (
            FieldName::new("body").unwrap(),
            FieldValue::Text {
                value: "Choose an observation for every obligation.".into(),
                spans: vec![span(6146, 7800, 51)],
            },
        ),
    ]);
    EvidencePiece {
        kind: EvidenceKindName::new("step").unwrap(),
        location: PieceLocation {
            file: "SKILL.md".into(),
            first_line: 49,
            last_line: 60,
            span: span(6114, 7909, 49),
        },
        text: "raw step".into(),
        model_view: project_model_view(&fields, 6000),
        fields,
    }
}
pub fn index() -> FileMatchIndex {
    FileMatchIndex {
        by_rule: BTreeMap::from([(
            EvidenceRuleId::new("completion").unwrap(),
            vec![MatchRange {
                start: 7801,
                end: 7908,
            }],
        )]),
        limited: false,
        ..FileMatchIndex::default()
    }
}

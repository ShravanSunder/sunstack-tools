#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use serde_json::json;

fn selection(value: serde_json::Value) -> EvidenceSelection {
    serde_json::from_value(value).unwrap()
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

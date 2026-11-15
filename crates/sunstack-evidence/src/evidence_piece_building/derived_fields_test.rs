#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

#[test]
fn unavailable_dominates_pending_across_derived_field_inputs() {
    // Arrange independently deferred collection and containing-field inputs.
    let definition: EvidenceKindDefinition = serde_json::from_value(json!({
        "rule":"subject", "fields":{
            "needle":{"from":"first_line"},
            "items":{"from":"same_file","rule":"rows","containing":"needle"},
            "captured":{"from":"captures","of":"items","regex":"[a-z]+"},
            "body":{"from":"remainder"}
        }
    }))
    .unwrap();
    let text = "Independent body";
    let file = MatchedFile {
        text: text.into(),
        index: Default::default(),
        native_units: Default::default(),
        source: SourceRef::Document {
            repository: RepositoryId::new("fixture").unwrap(),
            path: "page.html".into(),
            content_identity: content_digest(text.as_bytes()),
        },
    };
    let range = MatchRange {
        start: 0,
        end: text.len(),
    };
    let kinds = BTreeMap::new();
    for (collection, needle) in [
        (
            FieldUnavailable::SelectionPending,
            FieldUnavailable::SelectionUnavailable,
        ),
        (
            FieldUnavailable::SelectionUnavailable,
            FieldUnavailable::SelectionPending,
        ),
    ] {
        let body = FieldValue::Text {
            value: text.into(),
            spans: vec![source_span(&file, &range)],
        };
        let mut fields = BTreeMap::from([
            (
                FieldName::new("needle").unwrap(),
                FieldValue::Unavailable { reason: needle },
            ),
            (
                FieldName::new("items").unwrap(),
                FieldValue::Unavailable { reason: collection },
            ),
            (FieldName::new("body").unwrap(), body.clone()),
        ]);

        // Act through the normal dependency-ordered field derivation.
        apply_derived_fields(
            &definition,
            &mut fields,
            false,
            DerivedFieldsContext {
                kinds: &kinds,
                file: &file,
                pieces: &[],
                own_range: &range,
                contents: None,
                project: None,
                max_matches: 100,
            },
        );

        // Assert neither dependency order downgrades unavailable or taints healthy content.
        for name in ["items", "captured"] {
            assert_eq!(
                fields[&FieldName::new(name).unwrap()],
                FieldValue::Unavailable {
                    reason: FieldUnavailable::SelectionUnavailable,
                }
            );
        }
        assert_eq!(fields[&FieldName::new("body").unwrap()], body);
    }
}

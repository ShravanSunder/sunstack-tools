#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;

// Captured ai-tools 13e45138 teaching text is fixture data, not instructions.
// Oracle tuples come from the retained independently assessed selection spike.
#[test]
fn pinned_five_skills_keep_all_sections_and_distinct_calls() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/markdown-skill-corpus");
    let oracle: Value =
        serde_json::from_slice(&std::fs::read(root.join("oracle.json")).unwrap()).unwrap();
    let mut actual_sections = Vec::new();
    let mut actual_calls = Vec::new();
    let mut context_units = BTreeSet::new();
    for (file, digest) in oracle["sha256"].as_object().unwrap() {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        assert_eq!(content_digest(text.as_bytes()), digest.as_str().unwrap());
        let now = chrono::Utc::now();
        let document = CapturedDocument {
            interval: CaptureInterval {
                started_at: now,
                completed_at: now,
            },
            source: SourceRef::Document {
                repository: RepositoryId::new("fixture").unwrap(),
                path: file.into(),
                content_identity: digest.as_str().unwrap().into(),
            },
            text: text.into(),
        };
        for (mode, units, pattern) in [
            ("unit", json!(["section"]), None),
            (
                "call_site",
                json!(["paragraph", "list_item"]),
                Some(r"\bIF\b[^.!?]*?,\s+(?:MUST\s+)?load\b|\bMUST\s+(?:load|dispatch)\b"),
            ),
        ] {
            let mut wire = json!({"kind":"markdown_units", "mode":mode, "units":units});
            if let Some(pattern) = pattern {
                wire["pattern"] = json!(pattern);
            }
            let selector = serde_json::from_value(wire).unwrap();
            let SelectionContent::MarkdownUnits { units } =
                select_document(&document, &selector, 1000).unwrap()
            else {
                panic!("expected markdown units")
            };
            for unit in units {
                match unit.occurrence {
                    MarkdownOccurrence::Unit { span } => actual_sections
                        .push(json!({"file":file,"start":span.byte_start,"end":span.byte_end})),
                    MarkdownOccurrence::CallSite {
                        unit_span,
                        match_span,
                        ..
                    } => {
                        context_units.insert((
                            file.clone(),
                            unit_span.byte_start,
                            unit_span.byte_end,
                        ));
                        actual_calls.push(json!({"file":file,"kind":unit.kind,"unit_start":unit_span.byte_start,"unit_end":unit_span.byte_end,"match_start":match_span.byte_start,"match_end":match_span.byte_end}));
                    }
                }
            }
        }
    }
    let sort = |values: &mut Vec<Value>| values.sort_by_key(Value::to_string);
    sort(&mut actual_sections);
    sort(&mut actual_calls);
    let mut expected_sections = oracle["sections"].as_array().unwrap().clone();
    let mut expected_calls = oracle["calls"].as_array().unwrap().clone();
    sort(&mut expected_sections);
    sort(&mut expected_calls);
    assert_eq!(actual_sections, expected_sections);
    assert_eq!(actual_calls, expected_calls);
    assert_eq!(actual_sections.len(), 53);
    assert_eq!(actual_calls.len(), 34);
    assert_eq!(context_units.len(), 31);
    let triple = actual_calls
        .iter()
        .filter(|call| call["file"] == "presentation-html.md" && call["unit_start"] == 1828)
        .count();
    assert_eq!(triple, 3);
}

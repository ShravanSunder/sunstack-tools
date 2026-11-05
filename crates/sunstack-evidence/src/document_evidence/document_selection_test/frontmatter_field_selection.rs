#![allow(clippy::unwrap_used, clippy::panic)]
use crate::*;
use serde_json::{Value, json};

fn select(text: &str, field: &str) -> Value {
    let now = chrono::Utc::now();
    let document = CapturedDocument {
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
    };
    let selector: EvidenceSelection =
        serde_json::from_value(json!({"kind":"frontmatter_field", "field":field})).unwrap();
    serde_json::to_value(select_document(&document, &selector, 10).unwrap()).unwrap()
}

#[test]
fn maintained_yaml_decoder_preserves_typed_values_and_raw_value_locations() {
    let text = "---\nname: alpha\nenabled: true\ncount: 4\nnothing: null\nitems: [one, two]\nnested: {key: value}\n---\nBody\n";
    for (field, expected) in [
        ("name", json!("alpha")),
        ("enabled", json!(true)),
        ("count", json!(4)),
        ("nothing", Value::Null),
        ("items", json!(["one", "two"])),
        ("nested", json!({"key":"value"})),
    ] {
        let found = select(text, field);
        assert_eq!(found["result"]["kind"], "present");
        assert_eq!(found["result"]["field"], field);
        assert_eq!(found["result"]["value"], expected);
        assert_eq!(found["result"]["span"]["source"]["kind"], "document");
    }
    let found = select(text, "name");
    let span = &found["result"]["span"];
    assert_eq!(
        &text[span["byte_start"].as_u64().unwrap() as usize
            ..span["byte_end"].as_u64().unwrap() as usize],
        "alpha"
    );
}

#[test]
fn absent_field_remains_one_selected_result_with_the_header_span() {
    for text in ["---\nname: alpha\n---\nBody\n", "---\n---\nBody\n"] {
        let found = select(text, "missing");
        assert_eq!(found["result"]["kind"], "absent");
        let span = &found["result"]["frontmatter_span"];
        assert_eq!(span["byte_start"], 0);
        assert_eq!(span["byte_end"], text.find("Body").unwrap());
    }
    let found = select("---\nname: {child: value}\n---\n", "name.child");
    assert_eq!(found["result"]["kind"], "absent");
}

#[test]
fn missing_or_malformed_block_is_selected_unavailable_without_invented_spans() {
    for (text, reason) in [
        ("Body\n", "missing"),
        ("\n---\nname: value\n---\n", "missing"),
        ("+++\nname = 'value'\n+++\n", "missing"),
        ("---\nname: [\n---\n", "malformed"),
        ("---\nname: value\n", "malformed"),
        ("---\nname: one\nname: two\n---\n", "malformed"),
        ("---\n- one\n- two\n---\n", "malformed"),
    ] {
        let found = select(text, "name");
        assert_eq!(found["result"]["kind"], "block_unavailable", "{text}");
        assert_eq!(found["result"]["reason"], reason, "{text}");
        assert!(found["result"].get("span").is_none());
        assert!(found["result"].get("frontmatter_span").is_none());
        assert!(
            !found["result"]["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn yaml_unicode_and_crlf_spans_stay_in_original_bytes_and_character_columns() {
    let text = "---\r\nclé: café\r\n---\r\nBody\r\n";
    let found = select(text, "clé");
    assert_eq!(found["result"]["value"], "café");
    let span = &found["result"]["span"];
    assert_eq!(span["start_line"], 2);
    assert_eq!(span["start_column"], 6);
    assert_eq!(span["end_column"], 10);
    assert_eq!(
        &text[span["byte_start"].as_u64().unwrap() as usize
            ..span["byte_end"].as_u64().unwrap() as usize],
        "café"
    );
}

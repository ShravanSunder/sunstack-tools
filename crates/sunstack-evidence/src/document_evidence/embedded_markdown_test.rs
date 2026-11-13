#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use serde_json::json;

#[test]
fn selectors_use_parsed_names_and_exact_decoded_attributes_without_duplicates() {
    let settings = serde_json::from_value(json!({"embedded_markdown":[
        {"tag":"SCRIPT","attributes":{"TYPE":"text/markdown"}},
        {"tag":"script","attributes":{"type":"text/markdown"}}
    ]}))
    .unwrap();
    let text = "<!-- <script type='text/markdown'>fake</script> --><SCRIPT TYPE='text&#47;markdown'>文\r\n- a\r\n</SCRIPT><script type='TEXT/MARKDOWN'>decoy</script><script>decoy</script>";
    let ranges = island_ranges(text, &settings);
    assert_eq!(ranges.len(), 1);
    assert_eq!(&text[ranges[0].clone()], "文\r\n- a\r\n");
    assert_eq!(ranges[0].start, text.find("文").unwrap());
}

#[test]
fn outermost_selected_elements_win_and_unclosed_or_self_closing_elements_are_ignored() {
    let settings =
        serde_json::from_value(json!({"embedded_markdown":[{"tag":"custom-md"},{"tag":"script"}]}))
            .unwrap();
    let text =
        "<custom-md>before<script>nested</script>after</custom-md><custom-md/><custom-md>unclosed";
    let ranges = island_ranges(text, &settings);
    assert_eq!(ranges.len(), 1);
    assert_eq!(
        &text[ranges[0].clone()],
        "before<script>nested</script>after"
    );
    assert!(island_ranges(text, &Default::default()).is_empty());
}

#[test]
fn direct_unit_selection_maps_calls_across_islands_and_enforces_the_shared_limit() {
    use crate::*;
    let settings: DocumentEvidenceSettings = serde_json::from_value(json!({"html":{"embedded_markdown":[{"tag":"script","attributes":{"type":"text/markdown"}}]}})).unwrap();
    let text = "<p>decoy Use</p>文<script type='text/markdown'>Use café &amp;\r\n</script>\r\n<script type='text/markdown'>Use second\r\n</script>";
    let now = chrono::Utc::now();
    let document = CapturedDocument {
        interval: CaptureInterval {
            started_at: now,
            completed_at: now,
        },
        source: SourceRef::Document {
            repository: RepositoryId::new("fixture").unwrap(),
            path: "page.html".into(),
            content_identity: content_digest(text.as_bytes()),
        },
        text: text.into(),
    };
    let selection = serde_json::from_value(
        json!({"kind":"markdown_units","mode":"call_site","units":["paragraph"],"pattern":"Use"}),
    )
    .unwrap();
    let SelectionContent::MarkdownUnits { units } =
        select_document(&document, &selection, &settings, 2).unwrap()
    else {
        panic!("units required")
    };
    assert_eq!(units.len(), 2);
    for (unit, expected) in units.iter().zip([
        text.find("Use café").unwrap(),
        text.find("Use second").unwrap(),
    ]) {
        assert_eq!(
            unit.span,
            source_span(&document.source, text, expected..expected + unit.text.len())
        );
        let MarkdownOccurrence::CallSite {
            unit_span,
            match_span,
            visible_range,
        } = &unit.occurrence
        else {
            panic!("call required")
        };
        assert_eq!(*unit_span, unit.span);
        assert_eq!(
            *match_span,
            source_span(&document.source, text, expected..expected + 3)
        );
        assert_eq!(
            &unit.visible_text[visible_range.start..visible_range.end],
            "Use"
        );
    }
    assert_eq!(
        select_document(&document, &selection, &settings, 1)
            .unwrap_err()
            .code,
        EvidenceCode::SelectorLimit
    );
    let absent: DocumentEvidenceSettings =
        serde_json::from_value(json!({"html":{"embedded_markdown":[{"tag":"missing"}]}})).unwrap();
    assert!(
        matches!(select_document(&document, &selection, &absent, 2).unwrap(), SelectionContent::MarkdownUnits {units} if units.is_empty())
    );
    let ast = EvidenceSelection::AstPattern {
        language: "markdown".into(),
        pattern: "Use second".into(),
    };
    let SelectionContent::Matches { matches } =
        select_document(&document, &ast, &settings, 2).unwrap()
    else {
        panic!("AST matches required")
    };
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].span.byte_start, text.find("Use second").unwrap());
    assert_eq!(
        matches[0].text,
        text[matches[0].span.byte_start..matches[0].span.byte_end]
    );
}

#[test]
fn island_settings_are_strict_and_empty_defaults_preserve_serialization() {
    use crate::HtmlEvidenceSettings;
    for invalid in [
        json!({"embedded_markdown":[{"tag":" "}]}),
        json!({"embedded_markdown":[{"tag":"script","attributes":{"":"x"}}]}),
        json!({"embedded_markdown":[{"tag":"script","typo":true}]}),
        json!({"embedded_markdown":[{}]}),
    ] {
        assert!(serde_json::from_value::<HtmlEvidenceSettings>(invalid).is_err());
    }
    assert_eq!(
        serde_json::to_value(HtmlEvidenceSettings::default()).unwrap(),
        json!({"skip_tags":[],"text_attributes":{}})
    );
}

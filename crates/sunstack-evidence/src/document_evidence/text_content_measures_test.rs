#![allow(clippy::unwrap_used)]
use super::count_text_content_words;

#[test]
fn counts_content_without_tags_attributes_urls_or_markdown_delimiters() {
    for (text, expected) in [
        (
            "<p class='many attribute words'><b>One</b> <a href='a b c'>two</a> three.</p>",
            3,
        ),
        ("**One** [two](https://example.com/many-words) `three`.", 3),
        ("- One two\n- three four\n", 4),
        ("| One | two |\n| --- | --- |\n| three | four |", 4),
        ("猫 café\u{2003}three\n", 3),
        ("<!-- words not content -->\n", 0),
        ("", 0),
    ] {
        // Act: use the shared parser-backed content measurement.
        let count = count_text_content_words(text, &Default::default()).unwrap();
        // Assert: literals are an independent oracle, including empty and Unicode text.
        assert_eq!(count, expected, "{text}");
    }
}

#[test]
fn inline_html_keeps_standard_and_configured_attribute_text() {
    let settings =
        serde_json::from_value(serde_json::json!({"text_attributes":{"span":["data-note"]}}))
            .unwrap();
    for (text, expected) in [
        ("A<img alt='red blue'>B", 5),
        ("A<svg aria-label='red blue'></svg>B", 5),
        (
            "before <span data-note='extra words'>inside</span> after",
            6,
        ),
    ] {
        assert_eq!(
            count_text_content_words(text, &settings).unwrap(),
            expected,
            "{text}"
        );
    }
}

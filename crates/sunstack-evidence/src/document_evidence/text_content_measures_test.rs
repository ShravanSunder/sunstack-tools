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

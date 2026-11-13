#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn mechanical_html_text_keeps_exact_spaces_and_decodes_entities_once() {
    assert_eq!(
        decoded_html_text("<blockquote>Say <b>&quot;yes&quot;</b>; it’s\n  &amp;lt;.</blockquote>")
            .unwrap(),
        "Say \"yes\"; it’s\n  &lt;."
    );
    assert_eq!(decoded_html_text("<p>A<b>B</b>C</p>").unwrap(), "ABC");
    assert_eq!(decoded_html_text("<p><!-- hidden --></p>").unwrap(), "");
    assert!(decoded_html_text("<p>Unclosed <").is_err());
}

#[test]
fn html_attribute_keeps_raw_value_range_and_distinguishes_missing_from_empty() {
    let raw = "<p data-name='a&amp;lt;b' empty=''>body</p>";
    let found = decoded_html_attribute(raw, &crate::HtmlAttributeName::new("DATA-NAME").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(found.value, "a&lt;b");
    assert_eq!(&raw[found.range], "a&amp;lt;b");
    assert!(
        decoded_html_attribute(raw, &crate::HtmlAttributeName::new("missing").unwrap())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        decoded_html_attribute(raw, &crate::HtmlAttributeName::new("empty").unwrap())
            .unwrap()
            .unwrap()
            .value,
        ""
    );
}

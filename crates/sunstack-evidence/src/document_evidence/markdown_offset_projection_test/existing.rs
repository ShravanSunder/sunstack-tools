#![allow(clippy::unwrap_used)]

use super::super::*;

#[test]
fn rendered_matches_keep_original_lexemes_and_unicode_offsets() {
    let raw = "Élan **Use** &amp; \\* now\r\nnext\n";
    let normalized = NormalizedMarkdown::new(raw);
    let projection = OffsetProjection::new(&normalized);
    let mapped = projection.mapped_prose(0..raw.len(), false, false);
    assert_eq!(mapped.text, "Élan Use & * now\nnext");
    for (visible, source) in [("Use", "Use"), ("&", "&amp;"), ("*", "\\*"), ("\n", "\r\n")] {
        let start = mapped.text.find(visible).unwrap();
        let span = mapped.raw_range(start..start + visible.len()).unwrap();
        assert_eq!(&raw[span], source);
    }
}

#[test]
fn distinct_visible_entity_scalars_share_one_atomic_raw_lexeme() {
    let raw = "&NotEqualTilde;\n";
    let normalized = NormalizedMarkdown::new(raw);
    let projection = OffsetProjection::new(&normalized);
    let mapped = projection.mapped_prose(0..raw.len(), false, false);
    let matches = regex::Regex::new(".")
        .unwrap()
        .find_iter(&mapped.text)
        .map(|found| (found.range(), mapped.raw_range(found.range()).unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 2);
    assert_ne!(matches[0].0, matches[1].0);
    assert_eq!(matches[0].1, 0..15);
    assert_eq!(matches[1].1, 0..15);
}

#[test]
fn inline_code_is_excluded_but_fenced_text_keeps_raw_mapping() {
    let raw = "Use `hidden` visible\n\n```text\nUse fenced\n```\n";
    let normalized = NormalizedMarkdown::new(raw);
    let projection = OffsetProjection::new(&normalized);
    let mapped = projection.mapped_prose(0..raw.len(), false, false);
    assert_eq!(mapped.text, "Use  visibleUse fenced\n");
    let start = mapped.text.find("Use fenced").unwrap();
    let span = mapped.raw_range(start..start + 10).unwrap();
    assert_eq!(&raw[span], "Use fenced");
}

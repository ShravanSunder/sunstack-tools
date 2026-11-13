#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn json_projection_decodes_one_string_and_preserves_raw_default() {
    let source = "\"Say \\\"yes\\\"; it’s\\n  clear.\"";
    assert_eq!(DocumentText::Raw.project(source).unwrap(), source);
    assert_eq!(
        DocumentText::JsonString.project(source).unwrap(),
        "Say \"yes\"; it’s\n  clear."
    );
    assert_eq!(DocumentText::JsonString.project("\"\"").unwrap(), "");
    for invalid in ["null", "42", "{}", "[]", "\"unfinished", "\"a\"\n\"b\""] {
        assert_eq!(
            DocumentText::JsonString.project(invalid),
            Err(FieldUnavailable::NotAString)
        );
    }
}

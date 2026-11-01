use super::super::*;

#[test]
fn display_preserves_public_spelling_without_collapsing_structured_identity() {
    let literal = ConfigPath::keys(["evidence", "kinds", "a.fields.body"]);
    let nested = ConfigPath::keys(["evidence", "kinds", "a", "fields", "body"]);
    assert_eq!(literal.to_string(), "evidence.kinds.a.fields.body");
    assert_eq!(literal.to_string(), nested.to_string());
    assert_ne!(literal, nested);
    let indexed = literal.with_key("within").with_index(1);
    assert_eq!(indexed.to_string(), "evidence.kinds.a.fields.body.within.1");
    assert_eq!(
        indexed.segments().last(),
        Some(&ConfigPathSegment::Index(1))
    );
}

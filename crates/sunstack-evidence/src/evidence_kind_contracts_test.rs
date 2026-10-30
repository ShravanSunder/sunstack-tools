#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn field_source_is_closed_and_field_names_exclude_reserved_names() {
    for value in [
        "text",
        "where",
        "standard",
        "Upper",
        "bad__name",
        "",
        "with2",
    ] {
        assert!(FieldName::new(value).is_err(), "{value}");
    }
    assert!(FieldName::new("completion_line").is_ok());
    for value in [
        r#"{"from":"first_line","extra":true}"#,
        r#"{"from":"target","regex":"x"}"#,
        r#"{"from":"unknown"}"#,
    ] {
        assert!(serde_json::from_str::<FieldSource>(value).is_err());
    }
    let source: FieldSource =
        serde_json::from_str(r#"{"from":"target","regex":"x","resolve":"path"}"#).unwrap();
    assert_eq!(source.shape(), FieldShape::Targets);
}

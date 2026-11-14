#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

#[test]
fn heading_level_sets_have_the_same_nonempty_unique_bounds_in_decode_and_schema() {
    let schema = schemars::schema_for!(HeadingLevels);
    let validator = jsonschema::validator_for(schema.as_value()).unwrap();
    for (wire, valid) in [
        (json!([1, 2]), true),
        (json!([6, 1]), true),
        (json!([]), false),
        (json!([1, 1]), false),
        (json!([0]), false),
        (json!([7]), false),
        (json!([1.5]), false),
        (json!(["2"]), false),
        (json!(null), false),
    ] {
        assert_eq!(validator.is_valid(&wire), valid, "{wire}");
        assert_eq!(
            serde_json::from_value::<HeadingLevels>(wire.clone()).is_ok(),
            valid,
            "{wire}"
        );
    }
    let levels = HeadingLevels::new(vec![6, 1]).unwrap();
    assert!(levels.contains(1));
    assert!(!levels.contains(2));
    assert_eq!(serde_json::to_value(levels).unwrap(), json!([6, 1]));
}

#[test]
fn level_predicates_are_optional_nonnull_and_only_apply_to_section_units() {
    for (html, schema) in [
        (true, schemars::schema_for!(crate::HtmlUnitSelection)),
        (false, schemars::schema_for!(crate::MarkdownUnitSelection)),
    ] {
        let validator = jsonschema::validator_for(schema.as_value()).unwrap();
        for (wire, valid) in [
            (json!({"mode":"unit","units":["section"]}), true),
            (
                json!({"mode":"unit","units":["section"],"levels":[1,2]}),
                true,
            ),
            (
                json!({"mode":"unit","units":["section"],"levels":[2],"starts_with":"Keep"}),
                true,
            ),
            (
                json!({"mode":"unit","units":["section"],"levels":[6,1],"pattern":"Keep"}),
                true,
            ),
            (
                json!({"mode":"unit","units":["section"],"levels":null}),
                false,
            ),
            (
                json!({"mode":"unit","units":["section"],"levels":[]}),
                false,
            ),
            (
                json!({"mode":"unit","units":["section"],"levels":[0]}),
                false,
            ),
            (
                json!({"mode":"unit","units":["section"],"levels":[7]}),
                false,
            ),
            (
                json!({"mode":"unit","units":["section"],"levels":[2,2]}),
                false,
            ),
            (
                json!({"mode":"unit","units":["section","section"],"levels":[2]}),
                false,
            ),
            (
                json!({"mode":"unit","units":["section","paragraph"],"levels":[2]}),
                false,
            ),
            (
                json!({"mode":"unit","units":["paragraph"],"levels":[2]}),
                false,
            ),
            (
                json!({"mode":"call_site","units":["section"],"levels":[2],"pattern":"Keep"}),
                false,
            ),
        ] {
            let decoded = if html {
                serde_json::from_value::<crate::HtmlUnitSelection>(wire.clone()).is_ok()
            } else {
                serde_json::from_value::<crate::MarkdownUnitSelection>(wire.clone()).is_ok()
            };
            assert_eq!(decoded, valid, "html={html}: {wire}");
            assert_eq!(validator.is_valid(&wire), valid, "html={html}: {wire}");
        }
    }
}

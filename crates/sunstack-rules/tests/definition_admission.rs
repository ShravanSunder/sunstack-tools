#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use sunstack_rules::*;
#[test]
fn reviewed_rule_set_and_bare_rule_lines_admit_native_questions() {
    let source = include_str!(
        "../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-quality.ruleset.json"
    );
    let rule_set =
        decode_rule_set(source.as_bytes(), "skill-quality.ruleset.json".as_ref()).unwrap();
    assert_eq!(rule_set.inputs.len(), 1);
    for line in
        include_str!("../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-rules.jsonl")
            .lines()
    {
        let rule =
            decode_rule_definition(line.as_bytes(), "skill-rules.jsonl".as_ref(), 1).unwrap();
        validate_rule_definition(&rule, &rule_set.inputs).unwrap();
    }
}
#[test]
fn raw_duplicate_fields_refuse_before_typed_maps_erase_them() {
    let source = include_str!(
        "../../../docs/specs/2026-10-12-rust-rule-engine/examples/skill-quality.ruleset.json"
    );
    let duplicate = source.replace(
        "\"schema_version\": 1",
        "\"schema_version\": 1, \"schema_version\": 1",
    );
    let diagnostics = decode_rule_set(duplicate.as_bytes(), "rules.json".as_ref()).unwrap_err();
    assert!(format!("{diagnostics:?}").contains("DuplicateField"));
}

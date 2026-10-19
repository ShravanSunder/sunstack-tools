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

#[test]
fn severity_cutover_accepts_current_words_and_refuses_old_words_with_replacements() {
    // R14 forbids aliases: old severities must fail at the typed definition boundary.
    let baseline = serde_json::json!({"rule_id":"current-severity","evidence":[],"target":"target","applicability":[],"questions":{},"verdict":{"kind":"noul","question":{"type":"noul","instructions":"judge"},"threshold":0.8}});
    for current in ["error", "warn"] {
        let mut rule = baseline.clone();
        rule["severity"] = serde_json::json!(current);
        let decoded = decode_rule_definition(
            &serde_json::to_vec(&rule).unwrap(),
            "rules.jsonl".as_ref(),
            7,
        )
        .unwrap();
        assert_eq!(serde_json::to_value(decoded.severity).unwrap(), current);
    }
    for (retired, replacement) in [("block", "error"), ("nudge", "warn")] {
        let mut rule = baseline.clone();
        rule["severity"] = serde_json::json!(retired);
        let diagnostics = decode_rule_definition(
            &serde_json::to_vec(&rule).unwrap(),
            "rules.jsonl".as_ref(),
            7,
        )
        .unwrap_err();
        assert!(diagnostics.items.iter().any(|item| matches!(item,
            ReportDiagnostic::Rule(diagnostic)
                if diagnostic.guidance.contains(replacement)
                && matches!(&diagnostic.location, RuleDiagnosticLocation::Definition { file, line: 7, field_path, .. }
                    if file == std::path::Path::new("rules.jsonl") && field_path == &["severity"])
        )), "{diagnostics:?}");
    }
}

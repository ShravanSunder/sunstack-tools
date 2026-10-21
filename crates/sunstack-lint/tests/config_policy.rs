#![allow(clippy::unwrap_used, clippy::panic)]
use serde_json::json;
use std::path::Path;
use sunstack_lint::*;
use sunstack_rules::{RuleId, Severity};

#[test]
fn selectors_have_one_string_wire_in_map_keys_and_report_fields_and_reject_malformed_names() {
    for written in ["skills/*", "skills/skill.description"] {
        let selector: RuleSelector = serde_json::from_value(json!(written)).unwrap();
        assert_eq!(serde_json::to_value(&selector).unwrap(), written);
        let map = std::collections::BTreeMap::from([(selector, SeveritySetting::Error)]);
        assert_eq!(serde_json::to_value(map).unwrap(), json!({written:"error"}));
    }
    for invalid in [
        "",
        "skills",
        "/rule",
        "skills/",
        "skills/a/b",
        "skills/INVALID",
    ] {
        assert!(
            serde_json::from_value::<RuleSelector>(json!(invalid)).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn policy_keeps_authored_wildcard_exact_and_ordered_override_precedence_and_origin() {
    // R10 is the oracle; an implementation choosing map order instead of precedence fails.
    let base = json!({"rule_sets":{"skills":{"path":"set.json","files":["**/*.md"]}}});
    let cases = [
        (
            json!({}),
            Severity::Warn,
            "guide.md",
            SeveritySetting::Warn,
            SettingOrigin::Authored,
        ),
        (
            json!({"rules":{"skills/*":"error"}}),
            Severity::Warn,
            "guide.md",
            SeveritySetting::Error,
            SettingOrigin::EntryWildcard,
        ),
        (
            json!({"rules":{"skills/*":"error","skills/one":"warn"}}),
            Severity::Error,
            "guide.md",
            SeveritySetting::Warn,
            SettingOrigin::ExactRule,
        ),
        (
            json!({"rules":{"skills/one":"error"},"overrides":[{"files":["**"],"rules":{"skills/*":"off"}},{"files":["guide.md"],"rules":{"skills/*":"warn","skills/one":"error"}}]}),
            Severity::Warn,
            "guide.md",
            SeveritySetting::Error,
            SettingOrigin::Override { index: 1 },
        ),
        (
            json!({"overrides":[{"files":["retired/**"],"rules":{"skills/*":"off"}}]}),
            Severity::Error,
            "retired/guide.md",
            SeveritySetting::Off,
            SettingOrigin::Override { index: 0 },
        ),
        (
            json!({"rules":{"skills/one":"error"}}),
            Severity::Warn,
            "guide.md",
            SeveritySetting::Error,
            SettingOrigin::ExactRule,
        ),
        (
            json!({"rules":{"skills/one":"warn"}}),
            Severity::Error,
            "guide.md",
            SeveritySetting::Warn,
            SettingOrigin::ExactRule,
        ),
    ];
    for (patch, authored, file, setting, origin) in cases {
        let mut raw = base.clone();
        raw.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let config: LintConfig = serde_json::from_value(raw).unwrap();
        let actual = effective_setting(
            &config,
            &EntryName::new("skills").unwrap(),
            &RuleId::new("one").unwrap(),
            authored,
            Some(Path::new(file)),
        );
        assert_eq!(actual.setting, setting);
        assert_eq!(actual.decided_by, origin);
    }
}

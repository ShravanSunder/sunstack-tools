#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::configuration_loading::config_validation::load_checked_rulebook;
use crate::{discover_config, load_config, test_support::lint_fixture as fixture};
use serde_json::json;

#[test]
fn recorded_bars_require_their_own_development_cases() {
    let root = tempfile::tempdir().unwrap();
    let mut config = fixture::judged(root.path(), "http://127.0.0.1:9/v1/systemone");
    config["filters"] = json!({"purpose":{"per":"packages/{name}","reads":{"file":"about.md","kind":"skill","fields":["description"]},"labels":["eligible","blocked"],"questions":{"q":{"type":"noul","instructions":{"check":"description"}}},"decision":{"start":"purpose","steps":{"purpose":{"ask":"q","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":{"label":"eligible"},"no":{"label":"blocked"},"uncertain":"unsure"}}}}});
    fixture::config_file(root.path(), &config);
    let loaded = load_config(&discover_config(root.path(), None).unwrap()).unwrap();
    let checked = load_checked_rulebook(&loaded).unwrap();
    let rule_case = |owner: &str, set: &str| {
        serde_json::from_value::<LintCase>(json!({"id":"known","revision":"fixture","set":set,"label_from":"blind-agents","kind":"rule","rule":owner,"file":"docs/one.md","lines":[1,1],"text_sha256":"0".repeat(64),"expected":"pass"})).unwrap()
    };
    let filter_case = |set: &str| {
        serde_json::from_value::<LintCase>(json!({"id":"known","revision":"fixture","set":set,"label_from":"blind-agents","kind":"filter","filter":"purpose","folder":"packages/one","reads_sha256":"0".repeat(64),"expected_label":"eligible"})).unwrap()
    };
    for (owner, step, value, case, reference, valid, reason) in [
        (
            "description",
            "complies",
            0.8,
            rule_case("description", "development"),
            "known",
            true,
            "",
        ),
        (
            "description",
            "complies",
            0.8,
            rule_case("description", "held-out"),
            "known",
            false,
            "development",
        ),
        (
            "description",
            "complies",
            0.8,
            filter_case("development"),
            "known",
            false,
            "owner",
        ),
        (
            "description",
            "complies",
            0.8,
            rule_case("another_rule", "development"),
            "known",
            false,
            "owner",
        ),
        (
            "description",
            "complies",
            0.8,
            rule_case("description", "development"),
            "unknown",
            false,
            "unknown",
        ),
        (
            "purpose",
            "purpose",
            0.7,
            filter_case("development"),
            "known",
            true,
            "",
        ),
        (
            "purpose",
            "purpose",
            0.7,
            filter_case("held-out"),
            "known",
            false,
            "development",
        ),
        (
            "purpose",
            "purpose",
            0.7,
            rule_case("purpose", "development"),
            "known",
            false,
            "owner",
        ),
    ] {
        let file = CaseFile {
            repository: None,
            cases: vec![case],
            bars: vec![RecordedBar {
                owner: owner.into(),
                step: StepName::new(step).unwrap(),
                bound: BarBound::YesAtOrAbove,
                value,
                set_from: vec![reference.into()],
            }],
        };
        let assessments = assess(&file, &checked);
        let bar = assessments
            .iter()
            .find(|bar| {
                bar.owner == owner
                    && bar.step.as_str() == step
                    && bar.bound == BarBound::YesAtOrAbove
            })
            .unwrap();
        assert_eq!(
            matches!(bar.state, BarState::Assessed),
            valid,
            "{owner} {reference}: {bar:?}"
        );
        if !valid {
            assert!(bar.reason.as_ref().unwrap().contains(reason), "{bar:?}");
        }
    }
}

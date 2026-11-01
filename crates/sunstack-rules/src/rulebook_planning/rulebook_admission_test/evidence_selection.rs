#![allow(clippy::unwrap_used, clippy::panic)]
use crate::rulebook_planning::rulebook_test_support as rule_fixture;
use crate::*;
use serde_json::Value;
use std::{collections::BTreeSet, path::PathBuf};
use sunstack_evidence::*;
fn root() -> PathBuf {
    rule_fixture::fixture().join("selection")
}
#[test]
fn selection_admission_refuses_each_crafted_cause_and_accepts_the_canonical_config() {
    let root = root();
    let config: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("skill-rules.config.json")).unwrap(),
    )
    .unwrap();
    let kinds = serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
    let project = load_evidence_project(&root.join("evidence/sgconfig.yml")).unwrap();
    assert!(admit_evidence_kinds(&kinds, &project, 6000).is_empty());
    for (name, expected) in [
        ("choice-question", "noul"),
        ("invalid-question", "schema"),
        ("high-threshold", "threshold"),
        ("equal-threshold", "threshold"),
        ("shared-undefined", "same_file"),
        ("shared-first-line", "same_file"),
        ("question-undefined", "defined fields"),
        ("question-target", "target field"),
        ("within-undefined", "undefined kind"),
        ("within-cycle", "cycle"),
        ("kind-cycle", "cycle"),
        ("kind-undefined", "undefined"),
        ("exclude-undefined", "undefined"),
        ("as-kind-selected", "selected"),
        ("as-kind-dependent", "selection-dependent"),
    ] {
        let config: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(format!("refusals/{name}.json"))).unwrap(),
        )
        .unwrap();
        let kinds = serde_json::from_value(config["evidence"]["kinds"].clone()).unwrap();
        let errors = admit_evidence_kinds(&kinds, &project, 6000);
        let public_paths = errors
            .iter()
            .map(|error| error.field_path.to_string())
            .collect::<BTreeSet<_>>();
        let expected_paths: &[&str] = match name {
            "choice-question" | "invalid-question" => &[
                "evidence.kinds.selected_targets.select.question",
                "evidence.kinds.workflow_section.select.question",
            ],
            "high-threshold" | "equal-threshold" => &[
                "evidence.kinds.selected_targets.select",
                "evidence.kinds.workflow_section.select",
            ],
            "shared-undefined" | "shared-first-line" => &[
                "evidence.kinds.selected_targets.select.shared",
                "evidence.kinds.workflow_section.select.shared",
            ],
            "question-undefined" => &[
                "evidence.kinds.selected_targets.select.question.instructions.check",
                "evidence.kinds.workflow_section.select.question.instructions.check",
            ],
            "question-target" => &["evidence.kinds.selected_targets.select.question"],
            "within-undefined" | "within-cycle" | "kind-cycle" => &["evidence.kinds.step"],
            "kind-undefined" => &["evidence.kinds.load", "evidence.kinds.load.fields.step"],
            "exclude-undefined" => &[
                "evidence.kinds.workflow_section",
                "evidence.kinds.workflow_section.fields.body",
            ],
            "as-kind-selected" => &["evidence.kinds.a_load"],
            "as-kind-dependent" => &["evidence.kinds.a_load", "evidence.kinds.selected_targets"],
            _ => panic!(),
        };
        assert_eq!(
            public_paths,
            expected_paths
                .iter()
                .map(|path| (*path).to_owned())
                .collect(),
            "public field spelling changed for {name}"
        );
        assert!(
            errors.iter().any(|error| error.message.contains(expected)),
            "{name}: {errors:?}"
        );
    }
}

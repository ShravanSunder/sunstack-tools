#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

#[test]
fn tree_filter_requires_declared_label_endings_and_refuses_the_old_shape() {
    let tree = json!({
        "per":"skills/{name}", "reads":{"file":"SKILL.md","kind":"skill","fields":["description"]},
        "labels":["eligible","blocked"], "reference":{"examples":{"eligible":"Portable guidance"}},
        "questions":{"purpose":{"type":"choice","instructions":{"check":"description","against":"reference.examples"},"criteria":{"portable":"Portable guidance","local":"Local procedure"}}},
        "decision":{"start":"purpose","steps":{"purpose":{"ask":"purpose","min_probability":0.6,"routes":{"portable":{"label":"eligible"},"local":{"label":"blocked"}},"uncertain":"unsure"}}}
    });
    let wire: FilterWire = serde_json::from_value(tree).unwrap();
    let kinds = crate::rulebook_planning::rulebook_test_support::kinds();
    let registry = admit_filters(
        &BTreeMap::from([(FilterName::new("purpose").unwrap(), wire)]),
        &kinds,
        &crate::rulebook_planning::rulebook_test_support::project(),
    )
    .unwrap();
    assert_eq!(registry.len(), 1);
    assert!(serde_json::from_value::<FilterWire>(json!({"per":"skills/{name}","reads":{"file":"SKILL.md","kind":"skill","field":"description"},"question":{"type":"choice","instructions":"Kind?","criteria":{"a":"A","b":"B"}},"min_probability":0.7})).is_err());
}

#[test]
fn undeclared_label_or_question_option_is_not_a_filter_ending() {
    for ending in [
        json!({"label":"portable"}),
        json!("portable"),
        json!("pass"),
        json!("uncertain"),
    ] {
        let wire: FilterWire = serde_json::from_value(json!({
            "per":"skills/{name}","reads":{"file":"SKILL.md","kind":"skill","fields":["description"]},"labels":["eligible"],
            "questions":{"purpose":{"type":"choice","instructions":{"check":"description"},"criteria":{"portable":"Reusable guidance","local":"One repository"}}},
            "decision":{"start":"purpose","steps":{"purpose":{"ask":"purpose","min_probability":0.6,"routes":{"portable":ending,"local":{"label":"eligible"}},"uncertain":"unsure"}}}
        })).unwrap();
        assert!(
            admit_filters(
                &BTreeMap::from([(FilterName::new("purpose").unwrap(), wire)]),
                &crate::rulebook_planning::rulebook_test_support::kinds(),
                &crate::rulebook_planning::rulebook_test_support::project()
            )
            .is_err()
        );
    }
}

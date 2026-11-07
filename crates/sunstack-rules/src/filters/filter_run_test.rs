#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use serde_json::json;

fn plan() -> FilterPlan {
    let wire=serde_json::from_value(json!({
        "per":"skills/{name}","reads":{"file":"SKILL.md","kind":"skill","fields":["description"]},"labels":["general","role","repo_specific"],
        "reference":{"examples":{"quiet":"Portable guidance"}},
        "questions":{"reusable":{"type":"noul","instructions":{"check":"description","context":["reference.examples"]}},"role":{"type":"noul","instructions":{"check":"description"}}},
        "decision":{"start":"reusable","steps":{
            "reusable":{"ask":"reusable","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":"role","no":{"label":"repo_specific"},"uncertain":"unsure"},
            "role":{"ask":"role","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":{"label":"role"},"no":{"label":"general"},"uncertain":"unsure"}
        }}
    })).unwrap();
    let name = FilterName::new("purpose").unwrap();
    crate::admit_filters(
        &BTreeMap::from([(name.clone(), wire)]),
        &crate::rulebook_planning::rulebook_test_support::kinds(),
        &crate::rulebook_planning::rulebook_test_support::project(),
    )
    .unwrap()
    .remove(&name)
    .unwrap()
}
fn piece() -> EvidencePiece {
    let mut piece = crate::rulebook_planning::rulebook_test_support::piece();
    let body = piece.fields[&sunstack_evidence::FieldName::new("body").unwrap()].clone();
    piece.kind = EvidenceKindName::new("skill").unwrap();
    piece.fields = BTreeMap::from([(
        sunstack_evidence::FieldName::new("description").unwrap(),
        body,
    )]);
    piece.model_view = project_model_view(&piece.fields, 6000);
    piece
}

#[test]
fn tree_filter_requests_only_declared_text_and_reference_without_folder_identity() {
    let plan = plan();
    let folder = FilterFolder {
        filter: plan.name.clone(),
        folder: "skills/private-folder-sentinel".into(),
        members: vec![],
        reads: ReadsPiece::One(0),
    };
    let prepared = request_for(
        &plan,
        &folder,
        &[piece()],
        &crate::rulebook_planning::rulebook_test_support::kinds(),
        6000,
    )
    .ok()
    .unwrap();
    let request = serde_json::to_value(prepared.request).unwrap();
    assert_eq!(request["questions"].as_object().unwrap().len(), 2);
    assert_eq!(
        request["state"]["reference"],
        json!({"examples":{"quiet":"Portable guidance"}})
    );
    assert!(request["state"].get("where").is_none());
    assert!(!request.to_string().contains("private-folder-sentinel"));
    assert!(request["state"].get("body").is_none());
    assert!(request["state"]["description"].as_str().is_some());
}

#[test]
fn filter_tree_has_distinct_root_and_second_level_unsure_routes_and_declared_labels() {
    for (first, second, label, path) in [
        (0.1, 0.9, Some("repo_specific"), vec!["reusable"]),
        (0.5, 0.9, None, vec!["reusable"]),
        (0.7, 0.7, Some("role"), vec!["reusable", "role"]),
        (0.9, 0.3, Some("general"), vec!["reusable", "role"]),
        (0.9, 0.5, None, vec!["reusable", "role"]),
    ] {
        let answers=serde_json::from_value(json!({"reusable":{"type":"noul","yes_probability":first},"role":{"type":"noul","yes_probability":second}})).unwrap();
        let result = walk_filter(&plan(), &piece(), &Default::default(), &answers);
        match (result, label) {
            (
                FilterAnswerState::Judged {
                    label,
                    path: observed,
                    answers: read,
                },
                Some(expected),
            ) => {
                assert_eq!(label.as_str(), expected);
                assert_eq!(
                    observed
                        .iter()
                        .map(crate::StepName::as_str)
                        .collect::<Vec<_>>(),
                    path
                );
                assert_eq!(read.len(), path.len());
            }
            (
                FilterAnswerState::Unsure {
                    step,
                    path: observed,
                    answers: read,
                },
                None,
            ) => {
                assert_eq!(
                    observed
                        .iter()
                        .map(crate::StepName::as_str)
                        .collect::<Vec<_>>(),
                    path
                );
                assert_eq!(Some(step.as_str()), path.last().copied());
                assert_eq!(read.len(), path.len());
            }
            _ => panic!("Unexpected tree terminal"),
        }
    }
}

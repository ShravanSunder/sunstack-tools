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
    let collected = filter_piece_for(
        &plan,
        &folder,
        &[piece()],
        &crate::rulebook_planning::rulebook_test_support::kinds(),
        &crate::rulebook_planning::rulebook_test_support::project(),
        6000,
    )
    .ok()
    .unwrap();
    let prepared = request_for(&plan, collected).ok().unwrap();
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

#[test]
fn html_filter_prepared_request_projects_text_but_keeps_raw_mechanics_and_reads_hash() {
    let project = crate::rulebook_planning::rulebook_test_support::project();
    let kinds = serde_json::from_value(json!({
        "outline": {
            "html_units":{"mode":"unit","units":["section"]},
            "fields":{"description":{"from":"section","text":"full"}}
        }
    }))
    .unwrap();
    let filter_name = FilterName::new("picture").unwrap();
    let wire = serde_json::from_value(json!({
        "per":"pages/{name}",
        "reads":{"file":"index.html","kind":"outline","fields":["description"]},
        "labels":["present","missing"],
        "questions":{"picture":{"type":"noul","instructions":{"check":"description"}}},
        "decision":{"start":"raw_tag","steps":{
            "raw_tag":{"mechanical_check":{"tool":"contains","field":"description","text":"<h2>Heading</h2>"},"yes":"picture","no":{"label":"missing"}},
            "picture":{"ask":"picture","yes_at_or_above":0.7,"no_at_or_below":0.3,"yes":{"label":"present"},"no":{"label":"missing"},"uncertain":"unsure"}
        }}
    })).unwrap();
    let plan = crate::admit_filters(
        &BTreeMap::from([(filter_name.clone(), wire)]),
        &kinds,
        &project,
    )
    .unwrap()
    .remove(&filter_name)
    .unwrap();
    let raw =
        r#"<h2>Heading</h2><p>Body</p><img alt="Flow"><ex-mermaid label="Steps"></ex-mermaid>"#;
    let mut raw_piece = piece();
    raw_piece.kind = EvidenceKindName::new("outline").unwrap();
    raw_piece.text = raw.into();
    let description = sunstack_evidence::FieldName::new("description").unwrap();
    raw_piece.fields = BTreeMap::from([(
        description.clone(),
        FieldValue::Text {
            value: raw.into(),
            spans: vec![raw_piece.location.span.clone()],
        },
    )]);
    let original_fields = raw_piece.fields.clone();
    let folder = FilterFolder {
        filter: filter_name,
        folder: "pages/one".into(),
        members: vec![],
        reads: ReadsPiece::One(0),
    };

    let collected = filter_piece_for(&plan, &folder, &[raw_piece], &kinds, &project, 6000)
        .ok()
        .unwrap();
    let prepared = request_for(&plan, collected.clone()).ok().unwrap();
    let request = serde_json::to_value(&prepared.request).unwrap();

    assert_eq!(
        request["state"]["description"],
        "Heading Body [alt: Flow] [label: Steps]"
    );
    assert_eq!(collected.fields, original_fields);
    assert_eq!(collected.text, raw);
    let expected_digest = sunstack_evidence::content_digest(
        &serde_json::to_vec(&json!({"description":raw})).unwrap(),
    );
    assert_eq!(prepared.reads_sha256, expected_digest);
    let answers =
        serde_json::from_value(json!({"picture":{"type":"noul","yes_probability":1.0}})).unwrap();
    let result = walk_filter(&plan, &collected, &Default::default(), &answers);
    assert!(
        matches!(result, FilterAnswerState::Judged { label, .. } if label.as_str() == "present")
    );
}

#[test]
fn filter_reads_with_no_declared_kind_are_undescribed() {
    let plan = plan();
    let folder = FilterFolder {
        filter: plan.name.clone(),
        folder: "skills/one".into(),
        members: vec![],
        reads: ReadsPiece::One(0),
    };
    let failure = filter_piece_for(
        &plan,
        &folder,
        &[piece()],
        &BTreeMap::new(),
        &crate::rulebook_planning::rulebook_test_support::project(),
        6000,
    )
    .err()
    .unwrap();
    assert!(matches!(failure.reason, FilterIncomplete::Undescribed));
}

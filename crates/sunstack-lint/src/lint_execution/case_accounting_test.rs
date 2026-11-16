#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

#[test]
fn selected_owner_does_not_borrow_a_same_line_siblings_unavailable_ending() {
    // Arrange: the selected original span passed; a different same-line piece was unavailable.
    let mut report = LintReport::empty(
        "config.json".into(),
        crate::ScopeChoice::All,
        crate::EffectiveLimits::default(),
    );
    report.case_selection = Some(serde_json::from_value(json!({
        "bindings":[{
            "binding":{"id":"first","kind":"rule","rule":"known","file":"page.html","lines":[1,1],"text_sha256":"captured"},
            "outcome":{"kind":"selected","owner":{"kind":"rule","rule":"known","file":"page.html","lines":[1,1],"bytes":[0,10]}}
        }],"excluded":[]
    })).unwrap());
    for (id, start, end, ending) in [(7, 0, 10, "pass"), (8, 10, 19, "unavailable")] {
        let mut result = json!({
            "text_sha256":null,"id":id,"request_ids":[],"name":"known","marker":"K001",
            "file":"page.html","first_line":1,"last_line":1,"step":null,"path":[],
            "outcome":ending
        });
        if ending == "unavailable" {
            result["reason"] = json!({"kind":"credential_unavailable"});
        }
        report.results.push(serde_json::from_value(result).unwrap());
        report.piece_evidence.insert(id, serde_json::from_value(json!({
            "piece":{
                "kind":"paragraph",
                "where":{"file":"page.html","first_line":1,"last_line":1,"span":{
                    "source":{"kind":"document","repository":"fixture","path":"page.html","content_identity":"captured"},
                    "byte_start":start,"byte_end":end,"start_line":1,"start_column":start+1,"end_line":1,"end_column":end+1
                }},
                "text":"fixture","fields":{},"model_view":{"state":"bounded","fields":{},"cut":[]}
            },"matches":{}
        })).unwrap());
    }

    // Act: settlement must compare the selected owner's actual original span.
    settle_case_accounting(&mut report, &[], false);

    // Assert: the sibling cannot replace the selected owner's completed ending.
    assert!(matches!(
        report.case_selection.as_ref().unwrap().bindings[0].outcome,
        CaseBindingStatus::Selected { .. }
    ));
}

#[test]
fn selected_unavailable_rule_and_filter_keep_typed_dispositions_and_actual_results() {
    // Arrange: uniquely bound subjects already produced unavailable assessments.
    let mut report = LintReport::empty(
        "config.json".into(),
        crate::ScopeChoice::All,
        crate::EffectiveLimits::default(),
    );
    report.case_selection = Some(serde_json::from_value(json!({
        "bindings":[
            {"binding":{"id":"rule-case","kind":"rule","rule":"known","file":"docs/one.md","lines":[1,1],"text_sha256":"captured"},
                "outcome":{"kind":"selected","owner":{"kind":"rule","rule":"known","file":"docs/one.md","lines":[1,1],"bytes":[0,4]}}},
            {"binding":{"id":"filter-case","kind":"filter","filter":"purpose","folder":"docs","reads_sha256":"captured"},
                "outcome":{"kind":"selected","owner":{"kind":"filter","filter":"purpose","folder":"docs"}}}
        ],"excluded":[]
    })).unwrap());
    report.piece_evidence.insert(7, serde_json::from_value(json!({
        "piece":{
            "kind":"paragraph",
            "where":{"file":"docs/one.md","first_line":1,"last_line":1,"span":{
                "source":{"kind":"document","repository":"fixture","path":"docs/one.md","content_identity":"captured"},
                "byte_start":0,"byte_end":4,"start_line":1,"start_column":1,"end_line":1,"end_column":5
            }},
            "text":"test","fields":{},"model_view":{"state":"bounded","fields":{},"cut":[]}
        },"matches":{}
    })).unwrap());
    report.results.push(serde_json::from_value(json!({
        "text_sha256":null,"id":7,"request_ids":[],"name":"known","marker":"K001",
        "file":"docs/one.md","first_line":1,"last_line":1,"step":null,"path":[],
        "outcome":"unavailable","reason":{"kind":"filter_unavailable","filter":"purpose","folder":"docs"}
    })).unwrap());
    report.filter_answers.push(
        serde_json::from_value(json!({
            "id":8,"request_ids":[],"filter":"purpose","folder":"docs","input_cut":false,
            "requested_model":"controlled","resolved_model":null,"state":"unavailable",
            "reason":{"kind":"credential_unavailable"},"field":null,"path":[],"answers":{}
        }))
        .unwrap(),
    );

    // Act: accounting must not count unavailable as a completed successful assessment.
    settle_case_accounting(&mut report, &[], false);

    // Assert: both selected IDs remain present and no provider request is fabricated.
    let selection = report.case_selection.as_ref().unwrap();
    assert_eq!(selection.bindings.len(), 2);
    assert!(
        matches!(&selection.bindings[0].outcome, CaseBindingStatus::Unavailable {
        reason: sunstack_rules::WorkUnavailableReason::FilterUnavailable { filter, folder }
    } if filter.as_str() == "purpose" && folder == std::path::Path::new("docs"))
    );
    assert!(matches!(
        selection.bindings[1].outcome,
        CaseBindingStatus::Unavailable {
            reason: sunstack_rules::WorkUnavailableReason::CredentialUnavailable {}
        }
    ));
    assert_eq!(report.results[0].id, 7);
    assert_eq!(report.filter_answers[0].id, 8);
    assert!(report.requests.is_empty());
}

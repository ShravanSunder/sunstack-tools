#![allow(clippy::unwrap_used)]
use super::*;
use serde_json::json;

fn result_document() -> serde_json::Value {
    json!({
        "text_sha256":null,"id":7,"request_ids":[],"name":"known","marker":"K001",
        "file":"docs/one.md","first_line":1,"last_line":1,"step":null,"path":[],
        "outcome":"unavailable","reason":{"kind":"credential_unavailable"}
    })
}

#[test]
fn flattened_endings_preserve_existing_wire_values_and_require_unavailable_reason() {
    // Arrange: established CLI endings keep their existing optional payloads.
    for ending in [
        json!({"outcome":"pass"}),
        json!({"outcome":"finding"}),
        json!({"outcome":"not_applicable"}),
        json!({"outcome":"not_applicable","reason":{"kind":"filter","filter":"purpose","label":"excluded"}}),
        json!({"outcome":"uncertain"}),
        json!({"outcome":"incomplete"}),
        json!({"outcome":"incomplete","gap":"request_limit_reached"}),
        json!({"outcome":"unavailable","reason":{"kind":"credential_unavailable"}}),
    ] {
        let mut document = result_document();
        document.as_object_mut().unwrap().remove("reason");
        document
            .as_object_mut()
            .unwrap()
            .extend(ending.as_object().unwrap().clone());

        // Act: use the actual result serializer and schema.
        let result: RulePieceResult = serde_json::from_value(document.clone()).unwrap();
        let encoded = serde_json::to_value(result).unwrap();

        // Assert: flattening adds no ending wrapper or unrelated reason/gap keys.
        assert_eq!(encoded, document);
    }
    let schema = jsonschema::validator_for(report_schema().as_value()).unwrap();
    let mut report = LintReport::empty(
        "config.json".into(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    report
        .results
        .push(serde_json::from_value(result_document()).unwrap());
    assert!(schema.is_valid(&serde_json::to_value(&report).unwrap()));
    for reason in [
        None,
        Some(json!({"kind":"filter_unavailable"})),
        Some(json!({"kind":"unknown"})),
    ] {
        let mut document = result_document();
        match reason {
            None => {
                document.as_object_mut().unwrap().remove("reason");
            }
            Some(reason) => document["reason"] = reason,
        }
        assert!(serde_json::from_value::<RulePieceResult>(document.clone()).is_err());
        let mut raw_report = serde_json::to_value(&report).unwrap();
        raw_report["results"][0] = document;
        assert!(!schema.is_valid(&raw_report));
    }
}

#[test]
fn unavailable_counts_keep_assessments_candidates_and_coverage_as_separate_units() {
    // Arrange: two rule pieces, one real filter folder, two candidates and one coverage scope.
    let mut report = LintReport::empty(
        "config.json".into(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    for id in [7, 8] {
        let mut document = result_document();
        document["id"] = json!(id);
        report
            .results
            .push(serde_json::from_value(document).unwrap());
    }
    report.filter_answers.push(
        serde_json::from_value(json!({
            "id":9,"request_ids":[],"filter":"purpose","folder":"docs","input_cut":false,
            "requested_model":"controlled","resolved_model":null,"state":"unavailable",
            "reason":{"kind":"credential_unavailable"},"field":null,"path":[],"answers":{}
        }))
        .unwrap(),
    );
    for line in [1, 2] {
        report.selections.push(serde_json::from_value(json!({
            "kind":"selected","where":{"file":"docs/one.md","first_line":line,"last_line":line,
                "span":{"source":{"kind":"document","repository":"fixture","path":"docs/one.md","content_identity":"fixture"},
                    "byte_start":0,"byte_end":1,"start_line":line,"start_column":1,"end_line":line,"end_column":2}},
            "first_line":"candidate","outcome":{"kind":"unavailable","reason":{"kind":"credential_unavailable"}}
        })).unwrap());
    }
    report.incomplete = vec![
        IncompletePart {
            field: None,
            rule: None,
            file: Some("docs/one.md".into()),
            reason: IncompleteReason::SelectionUnavailable {
                evidence_kind: EvidenceKindName::new("child").unwrap(),
                file: "docs/one.md".into(),
                selection_kind: EvidenceKindName::new("selected").unwrap(),
            },
        },
        IncompletePart {
            field: None,
            rule: None,
            file: None,
            reason: IncompleteReason::Unavailable {
                reason: sunstack_rules::WorkUnavailableReason::CredentialUnavailable {},
            },
        },
    ];

    // Act / Assert: incomplete assessment details do not inflate coverage units.
    assert_eq!(
        report.unavailable_counts(),
        UnavailableCounts {
            rule_pieces: 2,
            filter_folders: 1,
            selection_candidates: 2,
            evidence_scopes: 1
        }
    );
    assert!(
        serde_json::to_value(&report)
            .unwrap()
            .get("unavailable_counts")
            .is_none()
    );
    let output = crate::render(&report, crate::OutputFormat::Default);
    assert!(output.contains(
        "Unavailable: 2 rule pieces, 1 filter folders, 2 selection candidates, 1 evidence scopes"
    ));
}

#[test]
fn unavailable_selection_candidates_do_not_count_as_asked_or_selected() {
    // Arrange: one actual candidate can settle while a neighboring model boundary is unavailable.
    let mut report = LintReport::empty(
        "config.json".into(),
        ScopeChoice::All,
        EffectiveLimits::default(),
    );
    let entry: SelectionEntry = serde_json::from_value(json!({
        "kind":"selected","where":{"file":"docs/one.md","first_line":1,"last_line":1,
            "span":{"source":{"kind":"document","repository":"fixture","path":"docs/one.md","content_identity":"fixture"},
                "byte_start":0,"byte_end":1,"start_line":1,"start_column":1,"end_line":1,"end_column":2}},
        "first_line":"candidate","outcome":{"kind":"unavailable","reason":{"kind":"credential_unavailable"}}
    })).unwrap();
    let mut settled = entry.clone();
    settled.outcome = sunstack_rules::SelectionOutcome::Selected { yes: 0.9 };
    report.selections = vec![entry.clone(), settled];
    let mut catalog = sunstack_evidence::EvidenceCatalog::default();
    catalog.file_counts.push(sunstack_evidence::FileCount {
        file: entry.location.file,
        kinds: BTreeMap::from([(
            entry.kind,
            sunstack_evidence::KindCount {
                selection: Some(sunstack_evidence::SelectionCount {
                    candidates: 2,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )]),
    });

    // Act: project the actual selection endings through the ordinary report owner.
    crate::lint_reporting::outcome::project_selections(&mut catalog, &mut report);

    // Assert: unavailable contributes its own unit and consumes no asked/settled unit.
    let selection = catalog.file_counts[0]
        .kinds
        .values()
        .next()
        .unwrap()
        .selection
        .as_ref()
        .unwrap();
    assert_eq!(
        (
            selection.candidates,
            selection.asked,
            selection.selected,
            selection.not_selected,
            selection.uncertain,
            selection.incomplete,
            selection.unavailable
        ),
        (2, 1, 1, 0, 0, 0, 1)
    );
    assert!(report.requests.is_empty());
}

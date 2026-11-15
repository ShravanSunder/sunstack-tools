#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

fn document(payload: &str) -> String {
    format!(r#"{{"contract_revision":{CONTRACT_REVISION},"payload":{payload}}}"#)
}

fn configuration(reference: &str) -> String {
    format!(
        r#"{{"kind":"configuration","value":{{"id":"codec","base_directory":"/absent","marker_prefix":"K","check":{{"files":["*.md"]}},"decision_model":{{"model":"jev-latest"}},"evidence":{{"kind":"inline","matchers":{{}},"kinds":{{}}}},"rulebooks":[{{"kind":"inline","id":"rules","markdown":"Rule <!-- K001 -->","entries":{{"K001":{{"name":"rule","evidence":"piece","severity":"error","reference":{{"codec_boundary":{reference}}},"decision":{{"start":"step","steps":{{"step":{{"mechanical_check":{{"tool":"regex","field":"body","pattern":"x"}},"yes":"pass","no":{{"finding":"bad"}}}}}}}}}}}}}}]}}}}"#
    )
}

fn invalid_reason(text: &str, reason: DocumentInvalidReason) -> Vec<String> {
    match deserialize_document(text) {
        DeserializationOutcome::Invalid {
            reason: observed,
            diagnostics,
        } => {
            assert_eq!(observed, reason, "{diagnostics:?}");
            assert!(!diagnostics.is_empty());
            diagnostics[0].json_path.clone()
        }
        other => panic!("expected {reason:?}, got {other:?}"),
    }
}

#[test]
fn independently_authored_integer_tokens_stay_exact_in_structured_json() {
    let text = document(&configuration(
        r#"{"minimum":-9223372036854775808,"maximum":18446744073709551615,"fraction":0.5,"text":"18446744073709551615"}"#,
    ));
    let DeserializationOutcome::Decoded { value } = deserialize_document(&text) else {
        panic!(
            "{}",
            serde_json::to_string(&deserialize_document(&text)).unwrap()
        );
    };
    let normalized = serde_json::to_string(&value).unwrap();
    assert!(normalized.contains("-9223372036854775808"));
    assert!(normalized.contains("18446744073709551615"));
    assert!(normalized.contains(r#""text":"18446744073709551615""#));
}

#[test]
fn unsupported_numbers_fail_with_the_original_path() {
    for number in [
        "18446744073709551616",
        "-9223372036854775809",
        "9007199254740993.0",
        "1e400",
    ] {
        let path = invalid_reason(
            &document(&configuration(number)),
            DocumentInvalidReason::UnsupportedNumber,
        );
        assert_eq!(
            path.last().map(String::as_str),
            Some("codec_boundary"),
            "{number}: {path:?}"
        );
    }
}

#[test]
fn exact_integer_coerced_to_f64_is_refused_after_typed_normalization() {
    let payload = configuration("1").replace(r#""yes":"pass""#, r#""yes_at_or_above":9007199254740993,"no_at_or_below":0.2,"ask":"question","uncertain":"uncertain","yes":"pass""#).replace(r#""mechanical_check":{"tool":"regex","field":"body","pattern":"x"},"#, "");
    let path = invalid_reason(
        &document(&payload),
        DocumentInvalidReason::UnsupportedNumber,
    );
    assert_eq!(path.last().map(String::as_str), Some("yes_at_or_above"));
}

#[test]
fn revision_is_checked_before_payload_interpretation() {
    let text = format!(
        r#"{{"contract_revision":{},"payload":{{"future":184467440737095516160000}}}}"#,
        CONTRACT_REVISION + 1
    );
    assert!(
        matches!(deserialize_document(&text), DeserializationOutcome::Incompatible { actual_contract_revision, .. } if actual_contract_revision == CONTRACT_REVISION + 1)
    );
}

#[test]
fn syntax_wrapper_and_original_typed_duplicate_refusals_stay_distinct() {
    invalid_reason("{", DocumentInvalidReason::MalformedJson);
    invalid_reason(
        &format!(r#"{{"contract_revision":{CONTRACT_REVISION},"payload":null,"extra":1}}"#),
        DocumentInvalidReason::InvalidDocument,
    );
    invalid_reason(
        &document(r#"{"kind":"unknown","value":{}}"#),
        DocumentInvalidReason::InvalidValue,
    );
    let duplicate =
        configuration("1").replace(r#""id":"codec""#, r#""id":"codec","id":"duplicate""#);
    invalid_reason(&document(&duplicate), DocumentInvalidReason::InvalidValue);
}

#[test]
fn preflight_does_not_erase_a_duplicate_map_keys_unsupported_number() {
    let path = invalid_reason(
        &document(&configuration(r#"{"same":18446744073709551616,"same":1}"#)),
        DocumentInvalidReason::UnsupportedNumber,
    );
    assert_eq!(path.last().map(String::as_str), Some("same"));
}

fn unavailable_report() -> serde_json::Value {
    serde_json::json!({
        "version": "codec-fixture",
        "configuration": {"kind": "inline", "id": "codec", "label": "18446744073709551615"},
        "sources": [], "scope": {"kind": "all", "widened": null},
        "status": {"kind": "incomplete"}, "findings": [],
        "results": [{
            "id": 7, "text_sha256": "source-identity", "request_ids": [],
            "name": "known", "marker": "K001", "file": "docs/one.md",
            "first_line": 3, "last_line": 3, "step": "ask", "path": ["ask"],
            "outcome": "unavailable", "reason": {"kind": "credential_unavailable"}
        }],
        "uncertain": [], "selections": [], "file_counts": [], "skipped_rules": [],
        "counts": {"error": 0, "warn": 0},
        "unavailable_counts": {"rule_pieces": 1, "filter_folders": 0,
            "selection_candidates": 0, "evidence_scopes": 0},
        "exclusions": sunstack_lint::ExclusionCounts::default(), "incomplete": [],
        "usage": sunstack_lint::UsageSummary::default(), "evidence_seen": {},
        "limits": sunstack_lint::EffectiveLimits {
            max_file_bytes: u64::MAX,
            ..sunstack_lint::EffectiveLimits::default()
        },
        "diagnostics": [], "requests": [], "filter_answers": [], "filter_skips": [],
        "piece_evidence": {}
    })
}

fn report_document(report: serde_json::Value) -> String {
    document(&serde_json::json!({"kind": "report", "value": report}).to_string())
}

#[test]
fn unavailable_reasons_and_exact_source_values_roundtrip_through_native_document_codec() {
    // Each required payload is independent of the decoder's own classification.
    for reason in [
        serde_json::json!({"kind": "credential_unavailable"}),
        serde_json::json!({"kind": "filter_unavailable", "filter": "purpose", "folder": "skills/guide"}),
        serde_json::json!({"kind": "selection_unavailable", "evidence_kind": "paragraph", "file": "docs/one.md"}),
        serde_json::json!({"kind": "selected_field_unavailable", "field": "body"}),
    ] {
        let mut report = unavailable_report();
        report["results"][0]["reason"] = reason.clone();
        let text = report_document(report);

        let DeserializationOutcome::Decoded { value } = deserialize_document(&text) else {
            panic!(
                "refused valid unavailable report: {:?}",
                deserialize_document(&text)
            );
        };

        let SdkDocumentValue::Report(report) = &value else {
            panic!("wrong root");
        };
        assert_eq!(report.unavailable_counts.rule_pieces, 1);
        assert_eq!(report.results[0].id, 7);
        assert!(report.results[0].request_ids.is_empty());
        assert_eq!(report.limits.max_file_bytes, u64::MAX);
        let normalized = serde_json::to_value(&value).unwrap();
        assert_eq!(normalized["value"]["results"][0]["reason"], reason);
        assert_eq!(
            normalized["value"]["configuration"]["label"],
            "18446744073709551615"
        );
        assert!(matches!(
            deserialize_document(&document(&serde_json::to_string(&value).unwrap())),
            DeserializationOutcome::Decoded { .. }
        ));
    }
}

#[test]
fn unavailable_documents_refuse_missing_malformed_and_unknown_reason_payloads() {
    let mut no_reason = unavailable_report();
    no_reason["results"][0]
        .as_object_mut()
        .unwrap()
        .remove("reason");
    invalid_reason(
        &report_document(no_reason),
        DocumentInvalidReason::InvalidValue,
    );
    for reason in [
        serde_json::json!(null),
        serde_json::json!({"kind": "future_reason"}),
        serde_json::json!({"kind": "filter_unavailable", "filter": "purpose"}),
        serde_json::json!({"kind": "selection_unavailable", "file": "docs/one.md"}),
        serde_json::json!({"kind": "selected_field_unavailable", "field": 3}),
        serde_json::json!({"kind": "credential_unavailable", "folder": "invented"}),
    ] {
        let mut report = unavailable_report();
        report["results"][0]["reason"] = reason;
        invalid_reason(
            &report_document(report),
            DocumentInvalidReason::InvalidValue,
        );
    }
    let mut unknown_ending = unavailable_report();
    unknown_ending["results"][0]["outcome"] = "future_ending".into();
    invalid_reason(
        &report_document(unknown_ending),
        DocumentInvalidReason::InvalidValue,
    );
}

#[test]
fn sdk_counts_cannot_erase_canonical_unavailable_facts_in_any_report_outcome() {
    let mut forged = unavailable_report();
    forged["unavailable_counts"]["rule_pieces"] = 0.into();
    assert_eq!(
        invalid_reason(
            &report_document(forged.clone()),
            DocumentInvalidReason::InvalidValue
        ),
        ["payload", "value", "unavailable_counts"]
    );
    for category in [
        "rule_pieces",
        "filter_folders",
        "selection_candidates",
        "evidence_scopes",
    ] {
        let mut inflated = unavailable_report();
        inflated["unavailable_counts"][category] = 2.into();
        assert_eq!(
            invalid_reason(
                &report_document(inflated),
                DocumentInvalidReason::InvalidValue
            ),
            ["payload", "value", "unavailable_counts"]
        );
    }
    let mut missing_counts = unavailable_report();
    missing_counts
        .as_object_mut()
        .unwrap()
        .remove("unavailable_counts");
    invalid_reason(
        &report_document(missing_counts),
        DocumentInvalidReason::InvalidValue,
    );
    for kind in ["incomplete", "completed", "invalid", "cancelled", "failed"] {
        let status = match kind {
            "completed" => "findings",
            "failed" => "incomplete",
            other => other,
        };
        forged["status"]["kind"] = status.into();
        let outcome = serde_json::json!({
            "kind": kind, "report": forged, "diagnostics": [],
            "failure": {"phase": "after_dispatch", "effects": "unknown",
                "reason": "native_call_failed", "message": "fixture"}
        });
        let payload = serde_json::json!({"kind": "run_outcome", "value": outcome});
        assert_eq!(
            invalid_reason(
                &document(&payload.to_string()),
                DocumentInvalidReason::InvalidValue
            ),
            ["payload", "value", "report", "unavailable_counts"]
        );
    }
    let text = report_document(unavailable_report());
    invalid_reason(
        &text.replace("\"rule_pieces\":1", "\"rule_pieces\":18446744073709551616"),
        DocumentInvalidReason::UnsupportedNumber,
    );
}

fn selection_entry(file: &str, kind: &str, outcome: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "where": {"file": file, "first_line": 1, "last_line": 1,
            "span": {"source": {"kind": "document", "repository": "fixture",
                "path": file, "content_identity": "fixture"},
                "byte_start": 0, "byte_end": 1, "start_line": 1, "start_column": 1,
                "end_line": 1, "end_column": 2}},
        "first_line": "candidate", "outcome": outcome,
    })
}

fn unavailable_selection_report() -> serde_json::Value {
    let mut report = unavailable_report();
    let unavailable =
        serde_json::json!({"kind": "unavailable", "reason": {"kind": "credential_unavailable"}});
    let mut selected_neighbor = selection_entry(
        "docs/one.md",
        "selected",
        serde_json::json!({"kind": "selected", "yes": 0.9}),
    );
    selected_neighbor["where"]["first_line"] = 2.into();
    selected_neighbor["where"]["last_line"] = 2.into();
    selected_neighbor["where"]["span"]["byte_start"] = 2.into();
    selected_neighbor["where"]["span"]["byte_end"] = 3.into();
    selected_neighbor["where"]["span"]["start_line"] = 2.into();
    selected_neighbor["where"]["span"]["end_line"] = 2.into();
    report["selections"] = serde_json::json!([
        selection_entry("docs/one.md", "selected", unavailable.clone()),
        selected_neighbor,
        selection_entry("docs/one.md", "other", unavailable.clone()),
        selection_entry("docs/two.md", "selected", unavailable.clone()),
        selection_entry("context/extra.md", "selected", unavailable),
    ]);
    let selection_count = serde_json::json!({
        "candidates": 1, "asked": 0, "selected": 0, "not_selected": 0,
        "uncertain": 0, "incomplete": 0, "unavailable": 1,
    });
    let kind_count = serde_json::json!({
        "matches": 1, "pieces": 0, "picked": 0, "limited": false,
        "selection": selection_count,
    });
    report["file_counts"] = serde_json::json!([
        {"file": "docs/one.md", "kinds": {"selected": kind_count.clone(), "other": kind_count.clone()}},
        {"file": "docs/two.md", "kinds": {"selected": kind_count}},
    ]);
    let mixed = &mut report["file_counts"][0]["kinds"]["selected"];
    mixed["matches"] = 2.into();
    mixed["pieces"] = 1.into();
    mixed["picked"] = 1.into();
    mixed["selection"]["candidates"] = 2.into();
    mixed["selection"]["asked"] = 1.into();
    mixed["selection"]["selected"] = 1.into();
    report["unavailable_counts"]["selection_candidates"] = 4.into();
    report
}

#[test]
fn selection_unavailable_counts_accept_partial_metadata_and_match_actual_kind_file_entries() {
    // The context selection has no file-count scope; one represented scope mixes
    // a settled candidate with an unavailable neighbor under the existing projection.
    let report = unavailable_selection_report();
    let text = report_document(report.clone());
    let DeserializationOutcome::Decoded { value } = deserialize_document(&text) else {
        panic!(
            "refused partial metadata: {:?}",
            deserialize_document(&text)
        );
    };
    let SdkDocumentValue::Report(decoded) = value else {
        panic!("wrong root");
    };
    assert_eq!(decoded.selections.len(), 5);
    assert_eq!(decoded.file_counts.len(), 2);

    for (file_index, kind) in [(0, "selected"), (0, "other"), (1, "selected")] {
        for forged_count in [0, 2] {
            let mut forged = report.clone();
            forged["file_counts"][file_index]["kinds"][kind]["selection"]["unavailable"] =
                forged_count.into();
            assert_eq!(
                invalid_reason(
                    &report_document(forged),
                    DocumentInvalidReason::InvalidValue
                ),
                [
                    "payload",
                    "value",
                    "file_counts",
                    &file_index.to_string(),
                    "kinds",
                    kind,
                    "selection",
                    "unavailable"
                ],
            );
        }
    }
}

#[test]
fn embedded_run_listing_and_inspection_documents_validate_their_own_selection_metadata() {
    let report = unavailable_selection_report();
    let inspection = serde_json::json!({
        "version": report["version"], "configuration": report["configuration"], "sources": [],
        "scope": report["scope"], "status": {"kind": "incomplete"}, "pieces": [],
        "file_counts": report["file_counts"], "selections": report["selections"],
        "usage": report["usage"], "limits": report["limits"], "exclusions": report["exclusions"],
        "diagnostics": [], "incomplete": [],
    });
    let mut with_listing = report.clone();
    with_listing["evidence_listing"] = inspection.clone();
    let mut forged_report = report.clone();
    forged_report["file_counts"][0]["kinds"]["selected"]["selection"]["unavailable"] = 0.into();
    let mut forged_inspection = inspection.clone();
    forged_inspection["file_counts"][0]["kinds"]["selected"]["selection"]["unavailable"] = 0.into();
    let mut forged_listing = with_listing.clone();
    forged_listing["evidence_listing"] = forged_inspection.clone();
    for (kind, valid, forged) in [
        (
            "run_outcome",
            serde_json::json!({"kind": "incomplete", "report": report}),
            serde_json::json!({"kind": "incomplete", "report": forged_report}),
        ),
        ("report", with_listing, forged_listing),
        (
            "inspection_report",
            inspection.clone(),
            forged_inspection.clone(),
        ),
        (
            "inspection_outcome",
            serde_json::json!({"kind": "incomplete", "report": inspection}),
            serde_json::json!({"kind": "incomplete", "report": forged_inspection}),
        ),
    ] {
        let valid = document(&serde_json::json!({"kind": kind, "value": valid}).to_string());
        assert!(
            matches!(
                deserialize_document(&valid),
                DeserializationOutcome::Decoded { .. }
            ),
            "{kind}"
        );
        let forged = document(&serde_json::json!({"kind": kind, "value": forged}).to_string());
        let path = invalid_reason(&forged, DocumentInvalidReason::InvalidValue);
        assert_eq!(
            path.last().map(String::as_str),
            Some("unavailable"),
            "{kind}: {path:?}"
        );
    }
}

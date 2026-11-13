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

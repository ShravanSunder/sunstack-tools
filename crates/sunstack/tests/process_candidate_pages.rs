#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/candidate_code_fixture.rs"]
mod code_fixture;
#[path = "../../sunstack-decisions/tests/support/mod.rs"]
mod decision_fixture;
#[path = "../../sunstack-rules/tests/support/context_page_assertions.rs"]
mod page_assertions;
mod support;
use code_fixture::CodeFixture;
use serde_json::{Value, json};
use sunstack_decisions::DecisionBackend;

async fn run_observed(fixture: &CodeFixture, input: Value) -> (Value, Vec<Value>, Vec<String>) {
    let reply = decision_fixture::HttpReply::json(
        json!({"model":"typesafe/page-proof","answers":{"q000001":{"type":"noul","noul":0.9}},"usage":{"input_tokens":20,"output_tokens":5,"cost":0.001}}),
    );
    let mut server = decision_fixture::TestServer::start(vec![reply]).await;
    let mut input = input;
    input["backend"]["endpoint"] = json!(server.endpoint);
    let directory = fixture.repository.root.path().to_owned();
    let output = support::run(&directory, "evaluate_candidates", input).await;
    let mut requests = Vec::new();
    let mut raw = Vec::new();
    while let Ok(capture) = server.captures.try_recv() {
        requests.push(serde_json::from_slice(&capture.body).unwrap());
        raw.push(String::from_utf8(capture.body).unwrap());
    }
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(server.finish().await, requests.len());
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    (wire["result"]["outcome"]["report"].clone(), requests, raw)
}
fn one_match(fixture: &CodeFixture) {
    fixture.edit_rule(|rule| {
        rule["mechanical_pass"]["predicate"]["pattern"] = json!("flagged(\"CHANGED_MATCH\")");
    });
}
fn only_page(request: &Value) -> &str {
    let pages = request["state"]["contents"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|content| content.get("page").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert_eq!(
        pages.len(),
        1,
        "one canonical context page per request: {request}"
    );
    pages[0]
}
fn without_page_payload(mut request: Value) -> Value {
    for content in request["state"]["contents"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        if content.get("page").is_some() {
            content["page"] = json!("");
        }
    }
    request
}
fn empty_page_baseline(request: &Value) -> String {
    let empty = without_page_payload(request.clone());
    let request =
        serde_json::from_value(json!({"questions":empty["questions"],"state":empty["state"]}))
            .unwrap();
    let encoded = decision_fixture::backend("http://127.0.0.1:1/unused")
        .encode_request(&request)
        .unwrap();
    String::from_utf8(encoded.body().to_vec()).unwrap()
}
fn actual_context_chars(request: &Value, raw: &str) -> usize {
    let baseline = empty_page_baseline(request);
    let token = serde_json::to_string(only_page(request)).unwrap();
    assert_eq!(raw.matches(&token).count(), 1);
    assert_eq!(
        raw.replacen(&token, "\"\"", 1),
        baseline,
        "materialized empty-page baseline changes invariant wiring"
    );
    raw.chars().count() - baseline.chars().count()
}
fn assert_bundle_page_links(view: &Value) {
    let view = serde_json::from_value(view.clone()).unwrap();
    page_assertions::check_bundle_pages(&view).unwrap();
}

// Wrong builds omit labels or put context-dependent anchors/identities outside
// the canonical page. The two real committed contexts must differ only there.
#[tokio::test]
async fn one_page_has_native_labels_exact_delivery_and_invariant_request_wiring() {
    let fixture = CodeFixture::new("http://127.0.0.1:1/unused", false);
    one_match(&fixture);
    let input = fixture.code_input();
    let (first, first_requests, first_raw) = run_observed(&fixture, input.clone()).await;
    assert_eq!(first_requests.len(), 1);
    let first_entries = first["candidates"].as_array().unwrap();
    assert_eq!(first_entries.len(), 1);
    let view = &first_entries[0]["context"];
    assert_eq!(first_entries[0]["answers"][0]["page"], 1);
    let pages = view["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 1, "P5 has context but no projected pages");
    let text = only_page(&first_requests[0]);
    assert_bundle_page_links(view);
    assert_eq!(pages[0]["text"], text);
    assert_eq!(pages[0]["number"], 1);
    assert_eq!(pages[0]["of"], 1);
    let delivered = actual_context_chars(&first_requests[0], &first_raw[0]);
    assert_eq!(pages[0]["rendered_chars"], delivered);
    assert!(delivered <= 20_000);
    support::record_http(
        "page-empty-baseline",
        empty_page_baseline(&first_requests[0]).as_bytes(),
        0,
    );
    support::record_http("page-actual-request", first_raw[0].as_bytes(), 1);
    assert!(
        view["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .any(|piece| piece["disposition"] == json!({"kind":"placed","page":1}))
    );
    assert_eq!(
        pages[0]["content_identity"],
        sunstack_evidence::content_digest(text.as_bytes())
    );
    for sentinel in [
        "COMMITTED_CALLER",
        "COMMITTED_CALLEE",
        "missing_name",
        "CHANGED_MATCH",
        "UNCHANGED_MATCH",
    ] {
        assert!(
            text.contains(sentinel),
            "missing {sentinel} from delivered page"
        );
    }
    assert!(!text.contains("DIRTY_CALLER_MUST_NOT_APPEAR"));
    assert!(!text.contains("sk-proj-publicfixturecredential123456789"));
    let labels = view["labels"].as_object().unwrap();
    let context = &view["context"];
    let units = context["units"].as_array().unwrap();
    let input_unit = units
        .iter()
        .find(|unit| unit["unit"]["key"] == first_entries[0]["occurrence"]["unit"])
        .unwrap();
    assert_eq!(
        labels[input_unit["piece_id"].as_str().unwrap()],
        json!({"value":"related","basis":{"kind":"input_unit","unit":input_unit["unit"]["key"]}})
    );
    let mut found = std::collections::BTreeSet::new();
    for piece in view["context"]["related"].as_array().unwrap() {
        let label = &labels[piece["piece_id"].as_str().unwrap()];
        if piece["kind"] == "unresolved" && piece["name_as_written"] == "missing_name" {
            assert_eq!(label["value"], "uncertain");
            assert!(piece.get("unit").is_none());
            assert_eq!(piece["call_site"]["source"]["path"], "src/lib.rs");
            assert_eq!(
                piece["call_site"]["source"]["revision"]["commit_id"],
                fixture.after
            );
            assert!(
                label["reasons"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"kind":"graph","reason":piece["reason"]}))
            );
            assert!(
                pages
                    .iter()
                    .all(|page| !page["text"].as_str().unwrap().contains("fn missing_name"))
            );
            assert!(
                units
                    .iter()
                    .all(|unit| !unit["text"].as_str().unwrap().contains("fn missing_name"))
            );
            found.insert("missing");
        }
        if piece["uncertainties"]
            .as_array()
            .is_some_and(|items| items.contains(&json!("ambiguous_declaration")))
        {
            assert_eq!(label["value"], "uncertain");
            let resolved = units
                .iter()
                .find(|unit| unit["unit"]["key"] == piece["unit"])
                .unwrap();
            assert_eq!(resolved["text"], "fn ambiguous() {}");
            assert!(
                label["reasons"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"kind":"graph","reason":"ambiguous_declaration"}))
            );
            found.insert("ambiguous");
        }
        if piece["connection"] == "callers" && piece["kind"] == "resolved" {
            assert_eq!(label["value"], "probably_related");
            let caller = units
                .iter()
                .find(|unit| unit["unit"]["key"] == piece["unit"])
                .unwrap();
            assert!(
                caller["text"]
                    .as_str()
                    .unwrap()
                    .contains("COMMITTED_CALLER")
            );
            assert_eq!(label["fact"]["piece_id"], piece["piece_id"]);
            let position = label["fact"]["position"].as_u64().unwrap() as usize;
            let fact = &piece["facts"][position];
            assert!(fact.is_object());
            assert_eq!(
                fact["span"]["source"]["revision"]["commit_id"],
                fixture.after
            );
            assert!(
                piece["facts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|fact| fact["kind"] == "static_call"
                        && fact["span"]["source"]["path"] == "src/caller.rs")
            );
            found.insert("caller");
        }
    }
    assert_eq!(
        found,
        std::collections::BTreeSet::from(["missing", "ambiguous", "caller"])
    );
    assert!(labels.values().any(|label| label["value"] == "related"));
    assert!(labels.values().all(|label| label["value"] != "unrelated"));
    let coverage = view["coverage"].as_array().unwrap();
    assert_eq!(
        coverage.len(),
        units.len()
            + context["related"].as_array().unwrap().len()
            + context["imports"].as_array().unwrap().len()
            + 1
    );
    assert_eq!(
        labels.keys().collect::<std::collections::BTreeSet<_>>(),
        coverage
            .iter()
            .map(|piece| piece["piece_id"].as_str().unwrap())
            .map(|id| labels.get_key_value(id).unwrap().0)
            .collect()
    );
    assert!(
        view["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .any(|piece| piece["redacted"].as_u64().is_some_and(|count| count > 0))
    );

    fixture.repository.write(
        "src/caller.rs",
        "pub fn caller() { crate::touched(); let note = \"SECOND_CONTEXT_λ_\\\"_CONTENT\"; }\n",
    );
    fixture.repository.stage("src/caller.rs");
    let revision = fixture.repository.commit();
    let mut second_input = input;
    second_input["sources"][0]["source"]["revision"]["revision"] = json!(revision);
    let (second, second_requests, second_raw) = run_observed(&fixture, second_input).await;
    assert_eq!(second_requests.len(), 1);
    assert_eq!(
        second["candidates"][0]["context"]["pages"][0]["rendered_chars"],
        actual_context_chars(&second_requests[0], &second_raw[0])
    );
    assert!(only_page(&second_requests[0]).contains("SECOND_CONTEXT_λ"));
    assert_ne!(
        only_page(&first_requests[0]),
        only_page(&second_requests[0])
    );
    assert_eq!(
        without_page_payload(first_requests[0].clone()),
        without_page_payload(second_requests[0].clone()),
        "context-derived bytes escaped the page payload"
    );
    let stripped = |request: &Value, raw: &str| {
        raw.replacen(
            &serde_json::to_string(only_page(request)).unwrap(),
            "\"\"",
            1,
        )
    };
    assert_eq!(
        stripped(&first_requests[0], &first_raw[0]),
        stripped(&second_requests[0], &second_raw[0])
    );
    assert_eq!(
        second["candidates"][0]["context"]["pages"][0]["text"],
        only_page(&second_requests[0])
    );
}

// Wrong build asks once over a context larger than the cap, or sends only its
// first page and reports an answer as if it had seen the whole context.
#[tokio::test]
async fn whole_context_needing_multiple_pages_is_unresolved_without_http() {
    let fixture = CodeFixture::new("http://127.0.0.1:1/unused", false);
    one_match(&fixture);
    for (path, signature, statement) in [
        ("src/caller.rs", "pub fn caller()", "crate::touched();"),
        ("src/helpers.rs", "pub fn callee()", ""),
    ] {
        let mut text = format!("{signature} {{\n{statement}\n");
        for number in 0..100 {
            text.push_str(&format!(
                "// {number:03} {}\n",
                "whole_unit_context_λ_".repeat(5)
            ));
        }
        text.push_str("}\n");
        fixture.repository.write(path, &text);
        fixture.repository.stage(path);
    }
    let revision = fixture.repository.commit();
    let mut input = fixture.code_input();
    input["sources"][0]["source"]["revision"]["revision"] = json!(revision);
    let (report, requests, raw) = run_observed(&fixture, input).await;
    assert!(raw.is_empty());
    assert!(
        requests.is_empty(),
        "whole_context must not dispatch a partial or oversized context"
    );
    let entries = report["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    let pages = entries[0]["context"]["pages"].as_array().unwrap();
    assert!(pages.len() >= 2);
    assert_eq!(
        entries[0]["outcome"],
        json!({"kind":"unresolved","at":{"kind":"verdict"},"missing":{"kind":"needs_whole_context","pages":pages.len()}})
    );
    assert_eq!(report["usage"]["requests"], 0);
    assert!(entries[0]["answers"].as_array().unwrap().is_empty());
    assert!(
        entries[0]["context"]["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .any(|piece| piece["disposition"]["kind"] == "placed")
    );
    for page in pages {
        assert_bundle_page_links(&entries[0]["context"]);
        assert!(page["rendered_chars"].as_u64().unwrap() <= 20_000);
        let delivered = serde_json::to_string(&page["text"])
            .unwrap()
            .chars()
            .count();
        let empty = serde_json::to_string(&json!("")).unwrap().chars().count();
        assert_eq!(page["rendered_chars"], delivered - empty);
    }
}

// The real input field must change the retained source, not merely a pure
// packer's options. Both cases use the same immutable Git source and judge.
#[tokio::test]
async fn runtime_truncation_settings_keep_exact_signature_head_gap_tail_and_source_range() {
    let fixture = CodeFixture::new("http://127.0.0.1:1/unused", false);
    one_match(&fixture);
    let mut source = "pub fn oversized() {\n    flagged(\"CHANGED_MATCH\");\n".to_owned();
    for number in 1..=300 {
        source.push_str(&format!("    // row{number:03} {}\n", "body".repeat(25)));
    }
    source.push_str("}\n");
    assert_eq!(source.lines().count(), 303);
    fixture.repository.write("src/lib.rs", &source);
    fixture.repository.stage("src/lib.rs");
    let revision = fixture.repository.commit();
    let mut delivered_texts = Vec::new();
    for (head, tail) in [(3, 2), (6, 4)] {
        let mut input = fixture.code_input();
        input["sources"][0]["source"]["revision"]["revision"] = json!(revision);
        input["context_settings"] = json!({"truncation":{"head_lines":head,"tail_lines":tail}});
        let (report, requests, raw) = run_observed(&fixture, input).await;
        assert_eq!(
            requests.len(),
            1,
            "an individually truncated unit must not leave artificial pages"
        );
        let entry = &report["candidates"][0];
        assert_eq!(report["candidates"].as_array().unwrap().len(), 1);
        assert_eq!(entry["outcome"]["kind"], "finding");
        assert_eq!(entry["answers"][0]["page"], 1);
        let pages = entry["context"]["pages"].as_array().unwrap();
        assert_eq!(pages.len(), 1);
        let page = &pages[0];
        assert_bundle_page_links(&entry["context"]);
        let text = only_page(&requests[0]);
        assert_eq!(page["text"], text);
        let count = actual_context_chars(&requests[0], &raw[0]);
        assert_eq!(page["rendered_chars"], count);
        assert!(count <= 20_000);
        assert!(text.contains("signature: pub fn oversized()"));
        let expected_head = source.split_inclusive('\n').take(head).collect::<String>();
        let expected_tail = source
            .lines()
            .skip(303 - tail)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains(&expected_head));
        assert!(text.contains(&expected_tail));
        assert!(text.contains(&format!("… {} lines truncated …", 303 - head - tail)));
        assert_eq!(text.matches("lines truncated").count(), 1);
        let unit = entry["context"]["context"]["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["unit"]["key"] == entry["occurrence"]["unit"])
            .unwrap();
        let coverage = entry["context"]["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .find(|piece| piece["piece_id"] == unit["piece_id"])
            .unwrap();
        assert_eq!(
            coverage["disposition"],
            json!({"kind":"truncated","page":1,"omitted_lines":{"first":head + 1,"last":303 - tail}})
        );
        delivered_texts.push(text.to_owned());
    }
    assert_ne!(
        delivered_texts[0], delivered_texts[1],
        "runtime settings were ignored"
    );
}

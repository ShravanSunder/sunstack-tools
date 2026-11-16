//! Exact source spans decide identity before the real selection and request paths.
use super::*;
use std::{path::Path, time::Duration};

const ADJACENT_HTML: &str = "<p>SELECTED</p><p>EXCLUDED</p>";

fn html_project(root: &Path, endpoint: &str, model_selection: bool) {
    let mut config = fixture::judged(root, endpoint);
    config["check"]["files"] = json!(["docs/*.html"]);
    config["evidence"]["kinds"] = json!({
        "paragraph":{"rule":"paragraph","fields":{"body":{"from":"remainder"}}}
    });
    if model_selection {
        config["evidence"]["kinds"]["paragraph"]["select"] = json!({
            "question":{"type":"noul","instructions":{"check":"body","task":"Keep this paragraph?"}},
            "yes_at_or_above":0.8,"no_at_or_below":0.2
        });
    }
    fixture::config_file(root, &config);
    std::fs::write(
        root.join("evidence/rules/skill.yml"),
        "id: paragraph\nlanguage: Html\nrule: {kind: element}\n",
    )
    .unwrap();
    std::fs::write(root.join("docs/one.html"), ADJACENT_HTML).unwrap();
    fixture::entry_file(
        root,
        &json!({"K001":{
            "name":"body","evidence":"paragraph","severity":"error",
            "questions":{"complies":{"type":"noul","instructions":{
                "check":"body","task":"Does this exact paragraph comply?"
            }}},
            "decision":{"start":"complies","steps":{"complies":{
                "ask":"complies","yes_at_or_above":0.8,"no_at_or_below":0.3,
                "yes":"pass","no":{"finding":"Paragraph does not comply."},"uncertain":"uncertain"
            }}}
        }}),
    );
}

fn byte_binding(id: &str, file: &str, bytes: [usize; 2], text: &[u8]) -> CaseBinding {
    serde_json::from_value(json!({
        "id":id,"kind":"rule","rule":"body","file":file,"lines":[1,1],
        "bytes":bytes,"text_sha256":sunstack_evidence::content_digest(text)
    }))
    .unwrap()
}

fn selected_request(root: &Path, bindings: Vec<CaseBinding>) -> LintRequest {
    let mut request = fixture::request(root);
    request.case_selection = Some(bindings);
    request
}

async fn observe(
    server: &mut controlled::ControlledServer,
    request: LintRequest,
) -> (LintReport, Vec<Vec<u8>>) {
    let mut operation = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC_BYTE_BINDING".into()),
    ));
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut bodies = Vec::new();
        loop {
            tokio::select! {
                report = &mut operation => break (report.unwrap(), bodies),
                request = server.receive() => {
                    bodies.push(request.raw_body.clone());
                    request.noul(0.9);
                }
            }
        }
    })
    .await
    .unwrap()
}

fn require_selected(report: &LintReport, id: &str, bytes: [usize; 2]) {
    let bindings = &report.case_selection.as_ref().unwrap().bindings;
    let matching = bindings
        .iter()
        .filter(|binding| binding.binding.id == id)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1, "{report:?}");
    assert!(matches!(&matching[0].binding.subject,
        CaseSubjectBinding::Rule { file, lines: [1, 1], bytes: Some(actual), .. }
        if file == Path::new("docs/one.html") && *actual == bytes));
    assert!(
        matches!(&matching[0].outcome,
        CaseBindingStatus::Selected { owner: CaseExecutionOwner::Rule { rule, file, lines: [1, 1], bytes: actual } }
        if rule.as_str() == "body" && file == Path::new("docs/one.html") && *actual == bytes),
        "{report:?}"
    );
}

fn saved_spans(report: &LintReport) -> Vec<[usize; 2]> {
    report
        .results
        .iter()
        .map(|result| {
            let piece = &report.piece_evidence[&result.id].piece;
            assert_eq!((result.first_line, result.last_line), (1, 1));
            assert_eq!(piece.location.file, Path::new("docs/one.html"));
            [piece.location.span.byte_start, piece.location.span.byte_end]
        })
        .collect()
}

#[tokio::test]
async fn adjacent_same_line_html_nodes_dispatch_only_the_exact_owner_with_unchanged_request_bytes()
{
    // Arrange: the real configured engine emits two adjacent, exclusive-end spans.
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    html_project(root.path(), &server.endpoint, false);
    let (ordinary, ordinary_http) = observe(&mut server, fixture::request(root.path())).await;
    assert_eq!(ordinary.status, LintStatus::Clean, "{ordinary:?}");
    assert_eq!(ordinary_http.len(), 2);
    assert_eq!(saved_spans(&ordinary), vec![[0, 15], [15, 30]]);
    let expected = ordinary_http
        .iter()
        .find(|raw| String::from_utf8_lossy(raw).contains("SELECTED"))
        .unwrap();

    // Act: distinct IDs for one owner must retain two rows and execute it once.
    let bindings = vec![
        byte_binding("selected", "docs/one.html", [0, 15], b"<p>SELECTED</p>"),
        byte_binding("same-owner", "docs/one.html", [0, 15], b"<p>SELECTED</p>"),
    ];
    let (selected, selected_http) =
        observe(&mut server, selected_request(root.path(), bindings)).await;

    // Assert: all request bytes and actual saved spans survive owner filtering.
    assert_eq!(selected.status, LintStatus::Clean, "{selected:?}");
    assert_eq!(selected_http, vec![expected.clone()]);
    assert_eq!(selected.usage.model_requests, 1);
    assert_eq!(saved_spans(&selected), vec![[0, 15]]);
    assert_eq!(selected.case_selection.as_ref().unwrap().bindings.len(), 2);
    require_selected(&selected, "selected", [0, 15]);
    require_selected(&selected, "same-owner", [0, 15]);
    assert_eq!(selected.case_selection.as_ref().unwrap().excluded.len(), 1);
    assert!(matches!(
        &selected.case_selection.as_ref().unwrap().excluded[0],
        CaseExecutionOwner::Rule {
            bytes: [15, 30],
            ..
        }
    ));
    assert!(matches!(&selected.requests[0].owner,
        RequestOwner::Rule { piece, .. }
        if piece.span.byte_start == 0 && piece.span.byte_end == 15));
    assert_eq!(
        selected.results[0].text_sha256.as_ref(),
        Some(&sunstack_evidence::content_digest(ADJACENT_HTML.as_bytes())),
        "the existing result digest keeps its whole-line meaning"
    );
    assert_eq!(server.finish().await, 3);
}

#[tokio::test]
async fn real_same_range_document_and_element_occurrences_remain_ambiguous() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    html_project(root.path(), &server.endpoint, false);
    let (positive, positive_http) = observe(&mut server, fixture::request(root.path())).await;
    assert_eq!(positive.status, LintStatus::Clean, "{positive:?}");
    assert_eq!(positive_http.len(), 2);
    std::fs::write(
        root.path().join("evidence/rules/skill.yml"),
        "id: paragraph\nlanguage: Html\nrule: {any: [{kind: document}, {kind: element}]}\n",
    )
    .unwrap();
    std::fs::write(root.path().join("docs/one.html"), "<p>one café</p>").unwrap();
    let binding = byte_binding(
        "ambiguous",
        "docs/one.html",
        [0, 16],
        "<p>one café</p>".as_bytes(),
    );

    let (report, http) = observe(&mut server, selected_request(root.path(), vec![binding])).await;

    assert!(http.is_empty(), "ambiguous occurrences must not dispatch");
    assert!(report.results.is_empty());
    let bindings = &report.case_selection.as_ref().unwrap().bindings;
    assert_eq!(bindings.len(), 1);
    let CaseBindingStatus::Ambiguous { owners } = &bindings[0].outcome else {
        panic!("genuine same-range occurrences were collapsed: {report:?}");
    };
    assert_eq!(owners.len(), 2);
    assert_eq!(
        owners[0], owners[1],
        "ambiguity counts occurrences, not distinct coordinates"
    );
    assert!(owners.iter().all(|owner| matches!(
        owner,
        CaseExecutionOwner::Rule {
            bytes: [0, 16],
            lines: [1, 1],
            ..
        }
    )));
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn byte_identity_hashes_crlf_and_partial_utf8_without_expanding_to_a_nearby_piece() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    html_project(root.path(), &server.endpoint, false);
    let original = "<p>café</p>\r\n";
    std::fs::write(root.path().join("docs/one.html"), original).unwrap();
    let valid = byte_binding("valid", "docs/one.html", [0, 12], "<p>café</p>".as_bytes());
    let (positive, positive_http) =
        observe(&mut server, selected_request(root.path(), vec![valid])).await;
    assert_eq!(positive.status, LintStatus::Clean, "{positive:?}");
    assert_eq!(positive_http.len(), 1);
    assert_eq!(saved_spans(&positive), vec![[0, 12]]);
    assert_eq!(
        positive.results[0].text_sha256.as_ref(),
        Some(&sunstack_evidence::content_digest(original.as_bytes()))
    );

    // These literal offsets include an off-by-one element end, half of é and CRLF.
    // Correct original-byte hashes are unmatched; a wrong hash is stale first.
    for (id, bytes, text, stale) in [
        ("off-by-one", [0, 11], "<p>café</p".as_bytes(), false),
        ("partial-utf8", [6, 7], &[0xc3][..], false),
        ("includes-crlf", [0, 14], original.as_bytes(), false),
        ("changed-piece", [0, 12], &b"changed"[..], true),
        ("changed-unknown-span", [0, 11], &b"changed"[..], true),
    ] {
        let binding = byte_binding(id, "docs/one.html", bytes, text);
        let (report, http) =
            observe(&mut server, selected_request(root.path(), vec![binding])).await;
        assert!(http.is_empty(), "{id}: {report:?}");
        assert!(report.requests.is_empty());
        assert!(report.results.is_empty());
        assert_ne!(report.status, LintStatus::Invalid, "{id}: {report:?}");
        let bindings = &report.case_selection.as_ref().unwrap().bindings;
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].binding.id, id);
        if stale {
            assert!(
                matches!(bindings[0].outcome, CaseBindingStatus::Stale),
                "{id}: {report:?}"
            );
        } else {
            assert!(
                matches!(bindings[0].outcome, CaseBindingStatus::Unmatched),
                "{id}: {report:?}"
            );
        }
    }
    assert_eq!(server.finish().await, 1);
}

#[tokio::test]
async fn direct_invalid_geometry_and_captured_bounds_refuse_before_any_model_selection() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    html_project(root.path(), &server.endpoint, true);
    let valid = byte_binding("valid", "docs/one.html", [0, 15], b"<p>SELECTED</p>");
    let (positive, positive_http) =
        observe(&mut server, selected_request(root.path(), vec![valid])).await;
    assert_eq!(positive.status, LintStatus::Clean, "{positive:?}");
    assert_eq!(
        positive_http.len(),
        2,
        "same observer witnesses selection and rule traffic"
    );
    assert!(
        positive
            .requests
            .iter()
            .any(|request| matches!(request.owner, RequestOwner::Selection { .. }))
    );
    require_selected(&positive, "valid", [0, 15]);

    for bytes in [[15, 0], [0, 31]] {
        let binding = byte_binding("invalid", "docs/one.html", bytes, b"arbitrary");
        let (report, http) =
            observe(&mut server, selected_request(root.path(), vec![binding])).await;
        assert_eq!(report.status, LintStatus::Invalid, "{report:?}");
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == LintInvalidCode::UnsupportedInputs),
            "{report:?}"
        );
        assert!(http.is_empty());
        assert!(report.requests.is_empty());
        assert_eq!(report.usage.model_requests, 0);
        assert!(report.results.is_empty());
    }
    assert_eq!(server.finish().await, 2);
}

#[tokio::test]
async fn stale_byte_bindings_do_not_demand_selection_alone_or_in_an_otherwise_unused_file() {
    let root = tempfile::tempdir().unwrap();
    let mut server = controlled::ControlledServer::start().await;
    html_project(root.path(), &server.endpoint, true);
    std::fs::write(root.path().join("docs/two.html"), "<p>OTHER</p>").unwrap();
    let valid = byte_binding("valid", "docs/one.html", [0, 15], b"<p>SELECTED</p>");
    let (positive, positive_http) = observe(
        &mut server,
        selected_request(root.path(), vec![valid.clone()]),
    )
    .await;
    assert_eq!(positive.status, LintStatus::Clean, "{positive:?}");
    assert_eq!(positive_http.len(), 2);
    require_selected(&positive, "valid", [0, 15]);

    let stale = byte_binding("stale", "docs/two.html", [0, 12], b"wrong original bytes");
    let (alone, alone_http) = observe(
        &mut server,
        selected_request(root.path(), vec![stale.clone()]),
    )
    .await;
    assert!(
        alone_http.is_empty(),
        "stale-only must not spend a selection request"
    );
    assert!(alone.requests.is_empty());
    assert_eq!(alone.usage.model_requests, 0);
    assert!(alone.results.is_empty());
    let rows = &alone.case_selection.as_ref().unwrap().bindings;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].binding.id, "stale");
    assert!(matches!(
        rows[0].binding.subject,
        CaseSubjectBinding::Rule {
            bytes: Some([0, 12]),
            ..
        }
    ));
    assert!(matches!(rows[0].outcome, CaseBindingStatus::Stale));

    let (mixed, mixed_http) = observe(
        &mut server,
        selected_request(root.path(), vec![valid, stale]),
    )
    .await;
    assert_eq!(
        mixed_http, positive_http,
        "the stale file must not add or alter selection or rule payloads"
    );
    assert_eq!(mixed.usage.model_requests, 2);
    assert_eq!(saved_spans(&mixed), vec![[0, 15]]);
    require_selected(&mixed, "valid", [0, 15]);
    let rows = &mixed.case_selection.as_ref().unwrap().bindings;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].binding.id, "stale");
    assert!(matches!(rows[1].outcome, CaseBindingStatus::Stale));
    assert_eq!(server.finish().await, 4);
}

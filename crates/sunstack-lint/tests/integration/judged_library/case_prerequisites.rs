//! Selection prerequisites retain the complete local composition before owner filtering.
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::{EvidenceKindName, FieldName, FieldUnavailable, FieldValue, ModelView};
use sunstack_lint::test_support::lint_fixture as fixture;
use sunstack_lint::*;
use sunstack_rules::test_support::controlled_http::ControlledServer;

const FIRST_DOCUMENT: &str = "# One\n\nFirst child café.\n\nSecond child β.\n";
const OTHER_DOCUMENT: &str = "# Two\n\nOther child 猫.\n\nOther neighbor γ.\n";

fn composed_project(root: &Path, endpoint: &str) {
    let mut config = fixture::judged(root, endpoint);
    config["evidence"]["kinds"] = json!({
        "outline": {
            "markdown_units":{"mode":"unit","units":["section"]},"fields":{}
        },
        "child": {
            "markdown_units":{"mode":"unit","units":["paragraph"]},
            "fields":{
                "body":{"from":"first_line"},
                "outline":{"from":"same_file","kind":"outline","text":"full"}
            },
            "select":{
                "question":{"type":"noul","instructions":{
                    "check":"body","context":["outline"],"task":"Keep this child?"
                }},
                "yes_at_or_above":0.8,"no_at_or_below":0.2,"shared":["outline"]
            }
        },
        "document": {
            "markdown_units":{"mode":"unit","units":["section"]},
            "fields":{
                "body":{"from":"section","text":"full"},
                "inside":{"from":"inside","kind":"child"},
                "siblings":{"from":"same_file","kind":"child","text":"full"}
            }
        }
    });
    fixture::config_file(root, &config);
    std::fs::write(root.join("docs/one.md"), FIRST_DOCUMENT).unwrap();
    std::fs::write(root.join("docs/two.md"), OTHER_DOCUMENT).unwrap();
    fixture::entry_file(
        root,
        &json!({"K001":{
            "name":"composed","evidence":"document","severity":"error",
            "questions":{"complies":{"type":"noul","instructions":{
                "check":"body","context":["inside","siblings","where"],
                "task":"Does the document agree with its settled child context?"
            }}},
            "decision":{"start":"complies","steps":{"complies":{
                "ask":"complies","yes_at_or_above":0.8,"no_at_or_below":0.2,
                "yes":"pass","no":{"finding":"Document does not comply."},
                "uncertain":"uncertain"
            }}}
        }}),
    );
}

fn selected_request(root: &Path) -> LintRequest {
    let mut request = fixture::request(root);
    request.case_selection = Some(
        serde_json::from_value(json!([{
            "id":"composed-one","kind":"rule","rule":"composed","file":"docs/one.md",
            "lines":[1,5],"text_sha256":sunstack_evidence::content_digest(FIRST_DOCUMENT.as_bytes())
        }]))
        .unwrap(),
    );
    request
}

struct CapturedInput {
    raw: Vec<u8>,
    body: Value,
}

async fn observe(
    server: &mut ControlledServer,
    request: LintRequest,
    child_probability: f64,
) -> (LintReport, Vec<CapturedInput>) {
    let mut operation = tokio::spawn(run(
        request,
        CancellationToken::new(),
        Some("SYNTHETIC_COMPOSED_SELECTION".into()),
    ));
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                report = &mut operation => break (report.unwrap(), captured),
                request = server.receive() => {
                    let selecting = request.body["state"]["candidates"].is_object();
                    captured.push(CapturedInput { raw: request.raw_body.clone(), body: request.body.clone() });
                    request.noul(if selecting { child_probability } else { 0.9 });
                }
            }
        }
    }).await.unwrap()
}

fn input_for<'a>(captured: &'a [CapturedInput], file: &str, selection: bool) -> &'a CapturedInput {
    let matching = captured
        .iter()
        .filter(|input| {
            input.body["state"]["where"]["file"] == file
                && input.body["state"]["candidates"].is_object() == selection
        })
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "exactly one request for {file}, selection={selection}"
    );
    matching[0]
}

fn require_prerequisite_content(input: &CapturedInput) {
    let candidates = input.body["state"]["candidates"].as_object().unwrap();
    assert_eq!(
        candidates.len(),
        2,
        "selection retains both children in the selected file"
    );
    let mut bodies = candidates
        .values()
        .map(|candidate| candidate["body"].as_str().unwrap());
    assert_eq!(bodies.next(), Some("First child café."));
    assert_eq!(bodies.next(), Some("Second child β."));
    assert_eq!(bodies.next(), None);
    assert_eq!(
        input.body["state"]["shared"]["outline"],
        json!([FIRST_DOCUMENT])
    );
}

fn require_same_bytes(left: &CapturedInput, right: &CapturedInput) {
    assert!(!left.raw.is_empty());
    assert!(!right.raw.is_empty());
    assert_eq!(
        right.raw, left.raw,
        "complete raw HTTP body, including model, stays identical"
    );
    assert_eq!(
        sunstack_evidence::content_digest(&right.raw),
        sunstack_evidence::content_digest(&left.raw)
    );
}

#[tokio::test]
async fn selected_section_retains_composed_child_prerequisites_through_positive_negative_and_pending_settlement()
 {
    // Arrange: both files are in ordinary scope; only one's section is a bound owner.
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    composed_project(root.path(), &server.endpoint);
    let selected = selected_request(root.path());
    let ordinary = fixture::request(root.path());

    // Keyless prepare must hold the final owner while its child model view is unresolved.
    let pending = prepare(selected.clone(), CancellationToken::new(), vec![]).await;
    assert_eq!(pending.status, LintStatus::Prepared, "{pending:?}");
    let accounting = pending.case_selection.as_ref().unwrap();
    assert_eq!(accounting.bindings.len(), 1);
    assert_eq!(accounting.bindings[0].binding.id, "composed-one");
    assert!(matches!(&accounting.bindings[0].outcome,
        CaseBindingStatus::SelectionPending { evidence_kind, file }
        if evidence_kind.as_str() == "document" && file == Path::new("docs/one.md")));
    assert_eq!(pending.requests.len(), 1);
    assert!(matches!(&pending.requests[0].owner,
        RequestOwner::Selection { selection_kind, file }
        if selection_kind.as_str() == "child" && file == Path::new("docs/one.md")));
    assert!(
        pending.results.is_empty(),
        "no fabricated final result before prerequisite answers"
    );
    let prepared_digest = pending.requests[0].input_sha256.as_ref().unwrap();
    assert_eq!(
        prepared_digest,
        &sunstack_evidence::content_digest(
            &serde_json::to_vec(&pending.requests[0].request).unwrap()
        )
    );
    assert!(
        pending
            .preparation
            .as_ref()
            .unwrap()
            .needs_answer
            .iter()
            .any(
                |entry| entry.kind.as_str() == "document" && entry.file == Path::new("docs/one.md")
            )
    );

    // Listing is the public view of the same built evidence; pending is not empty context.
    let mut listing = selected.clone();
    listing.evidence_listing = Some(Some(EvidenceKindName::new("document").unwrap()));
    let listing = prepare(listing, CancellationToken::new(), vec![]).await;
    let pieces = &listing.evidence_listing.as_ref().unwrap().pieces;
    let pending_piece = pieces
        .iter()
        .find(|piece| piece.location.file == Path::new("docs/one.md"))
        .unwrap();
    assert!(matches!(
        &pending_piece.model_view,
        ModelView::SelectionPending
    ));
    for field in ["inside", "siblings"] {
        assert!(matches!(
            pending_piece.fields.get(&FieldName::new(field).unwrap()),
            Some(FieldValue::Unavailable {
                reason: FieldUnavailable::SelectionPending
            })
        ));
    }

    // Act: settle every child yes. The original observer sees each file's prerequisite and rule.
    let (ordinary_yes, ordinary_yes_http) = observe(&mut server, ordinary.clone(), 0.9).await;
    let (selected_yes, selected_yes_http) = observe(&mut server, selected.clone(), 0.9).await;
    assert_eq!(ordinary_yes.status, LintStatus::Clean, "{ordinary_yes:?}");
    assert_eq!(selected_yes.status, LintStatus::Clean, "{selected_yes:?}");
    assert_eq!(ordinary_yes_http.len(), 4);
    assert_eq!(selected_yes_http.len(), 2);
    assert_eq!(
        ordinary_yes.file_counts.len(),
        2,
        "ordinary scope has both authored checked files"
    );
    let ordinary_selection = input_for(&ordinary_yes_http, "docs/one.md", true);
    let selected_selection = input_for(&selected_yes_http, "docs/one.md", true);
    require_prerequisite_content(ordinary_selection);
    require_prerequisite_content(selected_selection);
    require_same_bytes(ordinary_selection, selected_selection);
    let ordinary_rule = input_for(&ordinary_yes_http, "docs/one.md", false);
    let selected_rule = input_for(&selected_yes_http, "docs/one.md", false);
    require_same_bytes(ordinary_rule, selected_rule);
    assert_eq!(selected_rule.body["state"]["body"], FIRST_DOCUMENT);
    assert_eq!(
        selected_rule.body["state"]["inside"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        selected_rule.body["state"]["siblings"],
        selected_rule.body["state"]["inside"]
    );
    assert_eq!(selected_yes.results.len(), 1);
    assert_eq!(selected_yes.results[0].file, Path::new("docs/one.md"));
    assert!(matches!(
        selected_yes.case_selection.as_ref().unwrap().bindings[0].outcome,
        CaseBindingStatus::Selected { .. }
    ));
    assert!(selected_yes.case_selection.as_ref().unwrap().excluded.iter().any(|owner|
        matches!(owner, CaseExecutionOwner::Rule { file, .. } if file == Path::new("docs/two.md"))));
    assert!(
        selected_yes_http
            .iter()
            .all(|input| input.body["state"]["where"]["file"] == "docs/one.md")
    );
    let actual_selection = selected_yes.requests.iter().find(|request|
        matches!(&request.owner, RequestOwner::Selection { file, .. } if file == Path::new("docs/one.md"))).unwrap();
    assert_eq!(
        actual_selection.input_sha256.as_ref(),
        Some(prepared_digest)
    );
    assert_eq!(actual_selection.request, pending.requests[0].request);

    // Negative child answers remove context children, not the independently matched section.
    let (ordinary_no, ordinary_no_http) = observe(&mut server, ordinary, 0.1).await;
    let (selected_no, selected_no_http) = observe(&mut server, selected, 0.1).await;
    assert_eq!(ordinary_no.status, LintStatus::Clean, "{ordinary_no:?}");
    assert_eq!(selected_no.status, LintStatus::Clean, "{selected_no:?}");
    assert_eq!(ordinary_no_http.len(), 4);
    assert_eq!(selected_no_http.len(), 2);
    require_same_bytes(
        ordinary_selection,
        input_for(&selected_no_http, "docs/one.md", true),
    );
    require_same_bytes(
        input_for(&ordinary_no_http, "docs/one.md", true),
        input_for(&selected_no_http, "docs/one.md", true),
    );
    let negative_rule = input_for(&selected_no_http, "docs/one.md", false);
    require_same_bytes(
        input_for(&ordinary_no_http, "docs/one.md", false),
        negative_rule,
    );
    assert_ne!(
        negative_rule.raw, selected_rule.raw,
        "settlement materially changes composed context"
    );
    assert_eq!(negative_rule.body["state"]["inside"], json!([]));
    assert_eq!(negative_rule.body["state"]["siblings"], json!([]));
    assert_eq!(negative_rule.body["state"]["body"], FIRST_DOCUMENT);
    assert_eq!(
        selected_no.results.len(),
        1,
        "the real section still exists after all children are rejected"
    );
    assert_eq!(selected_no.results[0].file, Path::new("docs/one.md"));
    assert!(matches!(
        selected_no.case_selection.as_ref().unwrap().bindings[0].outcome,
        CaseBindingStatus::Selected { .. }
    ));
    assert!(
        selected_no_http
            .iter()
            .all(|input| input.body["state"]["where"]["file"] == "docs/one.md")
    );
    assert_eq!(
        server.finish().await,
        12,
        "preparing never contacted the real HTTP observer"
    );
}

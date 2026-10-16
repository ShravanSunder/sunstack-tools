#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod support;
use serde_json::json;
use support::*;

#[tokio::test]
async fn raw_protocol_errors_never_fabricate_an_operation_result() {
    for (bytes, code) in [
        (b"".as_slice(), "malformed_json"),
        (b"{".as_slice(), "malformed_json"),
        (b"[]".as_slice(), "invalid_framing"),
        (b"{} {}".as_slice(), "invalid_framing"),
        (br#"{"protocol_version":2,"operation":"prepare_evidence","input":{}}"#.as_slice(), "unsupported_protocol_version"),
        (br#"{"protocol_version":1,"operation":"not_an_operation","input":{}}"#.as_slice(), "unsupported_operation"),
        (br#"{"protocol_version":1,"protocol_version":1,"operation":"prepare_evidence","input":{}}"#.as_slice(), "duplicate_field"),
        (br#"{"protocol_version":1,"operation":"prepare_evidence","input":{},"AUTHOR_SECRET_KEY":3}"#.as_slice(), "unknown_field"),
    ] {
        let output = run_raw(&workspace_root(), bytes).await;
        let wire = response(&output, 2);
        assert_eq!(wire["kind"], "protocol_error");
        assert_eq!(wire["protocol_version"], 1);
        assert_eq!(wire["diagnostics"]["items"][0]["code"], code);
        assert!(wire.get("result").is_none());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("AUTHOR_SECRET_KEY"));
    }
}

#[tokio::test]
async fn evidence_success_and_required_optional_refusals_use_real_selections_and_exits() {
    let fixture = tempfile::tempdir().unwrap();
    std::fs::write(
        fixture.path().join("skill.md"),
        "U001: SELECTED_PROCESS_SENTINEL\n\nU002: PRIVATE_UNSELECTED_SENTINEL\n",
    )
    .unwrap();
    let input = evidence_input(fixture.path());
    let output = run(fixture.path(), "prepare_evidence", input.clone()).await;
    let wire = response(&output, 0);
    assert_eq!(wire["kind"], "operation_result");
    assert_eq!(wire["result"]["operation"], "prepare_evidence");
    assert_eq!(wire["result"]["outcome"]["kind"], "prepared");
    assert_eq!(wire["result"]["outcome"]["change_selections"], json!([]));
    assert_eq!(
        wire["result"]["outcome"]["selections"][0]["readiness"]["content"]["text"],
        "U001: SELECTED_PROCESS_SENTINEL"
    );
    assert_eq!(wire["result"]["outcome"]["measurements"]["files_read"], 1);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_UNSELECTED_SENTINEL"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("SELECTED_PROCESS_SENTINEL"));
    for (requirement, exit, kind) in [("required", 3, "incomplete"), ("optional", 0, "prepared")] {
        let mut missing = input.clone();
        missing["declarations"][0]["requirement"] = json!(requirement);
        missing["declarations"][0]["select"]["target_id"] = json!("U999");
        let wire = response(
            &run(fixture.path(), "prepare_evidence", missing).await,
            exit,
        );
        assert_eq!(wire["result"]["outcome"]["kind"], kind);
        assert_eq!(
            wire["result"]["outcome"]["selections"][0]["readiness"]["kind"],
            "refused"
        );
    }
    let mut invalid = input;
    invalid["sources"]["skill"]["path"] = json!("../outside.md");
    let wire = response(&run(fixture.path(), "prepare_evidence", invalid).await, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "refused");
}

#[tokio::test]
async fn unknown_field_inside_full_scope_is_refused_at_the_real_process_boundary() {
    let fixture = tempfile::tempdir().unwrap();
    std::fs::write(
        fixture.path().join("skill.md"),
        "U001: STRICT_VARIANT_PROCESS_CONTROL\n",
    )
    .unwrap();
    let valid = evidence_input(fixture.path());
    let output = run(fixture.path(), "prepare_evidence", valid.clone()).await;
    let wire = response(&output, 0);
    assert_eq!(wire["result"]["outcome"]["kind"], "prepared");
    assert_eq!(
        wire["result"]["outcome"]["selections"][0]["readiness"]["content"]["text"],
        "U001: STRICT_VARIANT_PROCESS_CONTROL"
    );
    let mut invalid = valid;
    invalid["repositories"][0]["change_scope"]["unexpected"] = json!(1);
    let output = run(fixture.path(), "prepare_evidence", invalid).await;
    let wire = response(&output, 2);
    assert_eq!(wire["kind"], "operation_result");
    assert_eq!(wire["result"]["operation"], "prepare_evidence");
    let outcome = &wire["result"]["outcome"];
    assert_eq!(outcome["kind"], "refused");
    assert_eq!(outcome["diagnostics"]["items"][0]["code"], "unknown_field");
    assert_eq!(outcome["diagnostics"]["items"][0]["origin"], "request");
    assert_eq!(
        outcome["diagnostics"]["items"][0]["location"],
        json!({"kind":"field","field_path":["repositories","change_scope"]})
    );
    assert!(outcome.get("selections").is_none());
    assert!(outcome.get("measurements").is_none());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("STRICT_VARIANT_PROCESS_CONTROL"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("unexpected"));
}

#[tokio::test]
async fn malformed_named_source_reports_only_schema_owned_location() {
    let fixture = tempfile::tempdir().unwrap();
    std::fs::write(fixture.path().join("skill.md"), "U001: source control\n").unwrap();
    let valid = evidence_input(fixture.path());
    let wire = response(
        &run(fixture.path(), "prepare_evidence", valid.clone()).await,
        0,
    );
    assert_eq!(wire["result"]["outcome"]["kind"], "prepared");
    let mut invalid = valid;
    let mut source = invalid["sources"]["skill"].take();
    source["AUTHOR_SECRET_KEY"] = json!("AUTHOR_SECRET_VALUE");
    invalid["sources"] = json!({"author_secret_source":source});
    invalid["declarations"][0]["input"] = json!("author_secret_source");
    let output = run(fixture.path(), "prepare_evidence", invalid).await;
    let wire = response(&output, 2);
    assert_eq!(wire["kind"], "operation_result");
    assert_eq!(wire["result"]["operation"], "prepare_evidence");
    assert_eq!(wire["result"]["outcome"]["kind"], "refused");
    let diagnostic = &wire["result"]["outcome"]["diagnostics"]["items"][0];
    assert_eq!(diagnostic["code"], "unknown_field");
    assert_eq!(diagnostic["origin"], "request");
    assert_eq!(
        diagnostic["location"],
        json!({"kind":"field","field_path":["sources"]})
    );
    for stream in [&output.stdout, &output.stderr] {
        let rendered = String::from_utf8_lossy(stream);
        for sentinel in [
            "author_secret_source",
            "AUTHOR_SECRET_KEY",
            "AUTHOR_SECRET_VALUE",
        ] {
            assert!(!rendered.contains(sentinel));
        }
    }
}

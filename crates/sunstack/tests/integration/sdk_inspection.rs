#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Real admission and evidence acquisition, without a provider or rule evaluation.
use serde_json::{Value, json};
use std::path::Path;
use sunstack::sdk_host::{
    self, CredentialSource,
    inspection_projection::{
        InspectRequest, InspectedStatus, InspectionOutcome, inspect_with_credential,
    },
    report_projection::RunOutcome,
    sdk_contracts::{CheckOutcome, Configuration, RunRequest, Scope},
};
use sunstack_decisions::CancellationToken;
use sunstack_lint::{LintStatus, ScopeChoice};

const SOURCE_TEXT: &str = "// café\nconsole.log('one');\n";
const MATCHED_TEXT: &str = "console.log('one')";

fn entries() -> Value {
    json!({"K001":{
        "name":"forbidden_call", "evidence":"call", "severity":"error",
        "decision":{"start":"allowed", "steps":{"allowed":{
            "mechanical_check":{"tool":"regex", "field":"body", "pattern":"^never"},
            "yes":"pass", "no":{"finding":"Call is forbidden.", "at":"body"}
        }}}
    }})
}

fn configuration_value(base: &Path, with_rulebook: bool) -> Value {
    let rulebooks = if with_rulebook {
        json!([{"kind":"inline", "id":"standard", "markdown":"Call is forbidden. <!-- K001 -->\n", "entries":entries()}])
    } else {
        json!([])
    };
    json!({
        "id":"inspection", "base_directory":base, "marker_prefix":"K",
        "check":{"files":["*.ts"], "ignore":["ignored.ts"]},
        "decision_model":{"model":"jev-latest"},
        "evidence":{"kind":"inline", "matchers":{"calls":{
            "language":"TypeScript", "rule":{"pattern":"console.log($$$ARGS)"}
        }}, "kinds":{"call":{"rule":"calls", "fields":{"body":{"from":"first_line"}}}}},
        "rulebooks":rulebooks
    })
}

fn configuration(value: &Value) -> Configuration {
    sdk_host::decode_configuration(&value.to_string()).unwrap()
}

fn request(root: &Path) -> InspectRequest {
    InspectRequest {
        root: root.to_path_buf(),
        scope: Scope::All,
        paths: vec![],
        evidence_kind: None,
    }
}

async fn inspect(value: &Value, root: &Path) -> InspectionOutcome {
    inspect_with_credential(
        configuration(value),
        request(root),
        CancellationToken::new(),
        CredentialSource::Supplied(None),
    )
    .await
}

fn write_source(root: &Path) {
    std::fs::write(root.join("one.ts"), SOURCE_TEXT).unwrap();
    std::fs::write(root.join("zero.ts"), "export const answer = 42;\n").unwrap();
    std::fs::write(root.join("ignored.ts"), "console.log('ignored');\n").unwrap();
}

#[tokio::test]
async fn empty_rulebook_inspection_returns_exact_source_and_zero_match_coverage() {
    // Arrange: no rulebook, one match, one no-match file and an explicit exclusion.
    let root = tempfile::tempdir().unwrap();
    write_source(root.path());
    let value = configuration_value(root.path(), false);

    // Act.
    let outcome = inspect(&value, root.path()).await;
    let InspectionOutcome::Inspected { report } = outcome else {
        panic!("{outcome:?}")
    };

    // Assert: the literal byte offset includes the multibyte comment prefix.
    assert_eq!(report.status, InspectedStatus::Clean);
    assert_eq!(report.pieces.len(), 1);
    let piece = &report.pieces[0];
    assert_eq!(piece.text, MATCHED_TEXT);
    assert_eq!(piece.location.file, Path::new("one.ts"));
    assert_eq!(piece.location.span.byte_start, 9);
    assert_eq!(piece.location.span.byte_end, 27);
    assert_eq!(
        (
            piece.location.span.start_line,
            piece.location.span.start_column
        ),
        (2, 1)
    );
    assert_eq!(
        (piece.location.span.end_line, piece.location.span.end_column),
        (2, 19)
    );
    assert_eq!(report.file_counts.len(), 2);
    let no_match = report
        .file_counts
        .iter()
        .find(|count| count.file == Path::new("zero.ts"))
        .unwrap();
    assert_eq!(
        (
            no_match.kinds[&sunstack_evidence::EvidenceKindName::new("call").unwrap()].matches,
            no_match.kinds[&sunstack_evidence::EvidenceKindName::new("call").unwrap()].pieces,
            no_match.kinds[&sunstack_evidence::EvidenceKindName::new("call").unwrap()].picked
        ),
        (0, 0, 0)
    );
    assert_eq!(report.exclusions.ignored, 1);
    assert_eq!(report.exclusions.zero_matches, 1);
    assert_eq!(report.usage.model_requests, 0);
    assert_eq!(report.usage.checked_files_acquired, 2);
    assert!(report.diagnostics.is_empty());
    assert!(report.incomplete.is_empty());

    // Changed bytes outside the expression identify a different source revision.
    let original_source = piece.location.span.source.clone();
    std::fs::write(
        root.path().join("one.ts"),
        "// other\nconsole.log('one');\n",
    )
    .unwrap();
    let changed = inspect(&value, root.path()).await;
    let InspectionOutcome::Inspected { report: changed } = changed else {
        panic!("{changed:?}")
    };
    assert_eq!(changed.pieces[0].text, MATCHED_TEXT);
    assert_eq!(changed.pieces[0].location.span.byte_start, 9);
    assert_eq!(changed.pieces[0].location.span.byte_end, 27);
    assert_ne!(changed.pieces[0].location.span.source, original_source);
}

#[tokio::test]
async fn clean_inspection_does_not_evaluate_a_rule_that_would_find_a_violation() {
    // Arrange: an admitted mechanical rule rejects this exact call.
    let root = tempfile::tempdir().unwrap();
    write_source(root.path());
    let value = configuration_value(root.path(), true);

    // Act.
    let inspected = inspect(&value, root.path()).await;
    let run = sdk_host::run_with_credential(
        configuration(&value),
        RunRequest {
            root: root.path().to_path_buf(),
            scope: Scope::All,
            paths: vec![],
        },
        CancellationToken::new(),
        CredentialSource::Supplied(None),
    )
    .await;

    // Assert: listing completion is independent of the rule verdict.
    let InspectionOutcome::Inspected { report } = inspected else {
        panic!("{inspected:?}")
    };
    assert_eq!(report.status, InspectedStatus::Clean);
    assert_eq!(report.pieces.len(), 1);
    let RunOutcome::Completed { report } = run else {
        panic!("{run:?}")
    };
    assert_eq!(report.findings.len(), 1);
}

#[tokio::test]
async fn inspection_admission_preserves_legacy_refusals_and_rejects_supplied_bad_definitions() {
    let root = tempfile::tempdir().unwrap();
    write_source(root.path());
    let empty = configuration_value(root.path(), false);

    let checked = sdk_host::check(configuration(&empty), CancellationToken::new()).await;
    let run = sdk_host::run_with_credential(
        configuration(&empty),
        RunRequest {
            root: root.path().to_path_buf(),
            scope: Scope::All,
            paths: vec![],
        },
        CancellationToken::new(),
        CredentialSource::Supplied(None),
    )
    .await;
    assert!(
        matches!(checked, CheckOutcome::Invalid { .. }),
        "{checked:?}"
    );
    assert!(matches!(run, RunOutcome::Invalid { .. }), "{run:?}");

    for field in ["files", "kinds", "matcher", "rulebook"] {
        let mut value = configuration_value(root.path(), field == "rulebook");
        match field {
            "files" => value["check"]["files"] = json!([]),
            "kinds" => value["evidence"]["kinds"] = json!({}),
            "matcher" => {
                value["evidence"]["matchers"]["calls"]["rule"] = json!({"matches":"undefined"})
            }
            "rulebook" => value["rulebooks"][0]["entries"]["K001"]["evidence"] = json!("undefined"),
            _ => unreachable!(),
        }
        let outcome = inspect(&value, root.path()).await;
        let InspectionOutcome::Invalid { diagnostics } = outcome else {
            panic!("{field}: {outcome:?}")
        };
        assert!(!diagnostics.is_empty(), "{field}");
    }

    let mut unknown = request(root.path());
    unknown.evidence_kind = Some(serde_json::from_value(json!("undefined")).unwrap());
    let outcome = inspect_with_credential(
        configuration(&empty),
        unknown,
        CancellationToken::new(),
        CredentialSource::Supplied(None),
    )
    .await;
    assert!(
        matches!(outcome, InspectionOutcome::Invalid { .. }),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn no_matches_are_clean_but_limits_and_cancellation_remain_explicit() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("zero.ts"), "export const answer = 42;\n").unwrap();
    let mut value = configuration_value(root.path(), false);
    let clean = inspect(&value, root.path()).await;
    let InspectionOutcome::Inspected { report } = clean else {
        panic!("{clean:?}")
    };
    assert_eq!(report.status, InspectedStatus::Clean);
    assert!(report.pieces.is_empty());
    assert_eq!(
        report.file_counts[0].kinds[&sunstack_evidence::EvidenceKindName::new("call").unwrap()]
            .picked,
        0
    );

    std::fs::write(root.path().join("one.ts"), SOURCE_TEXT).unwrap();
    value["limits"] = json!({"max_files":1});
    let partial = inspect(&value, root.path()).await;
    let InspectionOutcome::Incomplete { report } = partial else {
        panic!("{partial:?}")
    };
    assert_eq!(report.limits.max_files, 1);
    assert_eq!(report.file_counts.len(), 1);
    assert!(!report.incomplete.is_empty());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "max_files")
    );

    // Cancellation precedes even a missing definition's acquisition.
    value["rulebooks"] = json!([{"kind":"files", "markdown":"absent.md", "entries":"absent.json"}]);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let stopped = inspect_with_credential(
        configuration(&value),
        request(root.path()),
        cancellation,
        CredentialSource::Supplied(None),
    )
    .await;
    assert!(
        matches!(stopped, InspectionOutcome::Cancelled { report: None }),
        "{stopped:?}"
    );
}

#[tokio::test]
async fn cli_listing_and_direct_inspection_share_pieces_and_all_coverage_fields() {
    // Arrange: equivalent file and inline definitions, with a partial file limit.
    let root = tempfile::tempdir().unwrap();
    write_source(root.path());
    std::fs::create_dir(root.path().join("matchers")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [matchers]\n").unwrap();
    std::fs::write(
        root.path().join("matchers/calls.yml"),
        "id: calls\nlanguage: TypeScript\nrule: {pattern: 'console.log($$$ARGS)'}\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("standard.md"),
        "Call is forbidden. <!-- K001 -->\n",
    )
    .unwrap();
    std::fs::write(root.path().join("entries.json"), entries().to_string()).unwrap();
    let mut inline = configuration_value(root.path(), true);
    inline["limits"] = json!({"max_files":1});
    let cli = json!({
        "rulebook":[{"markdown":"standard.md", "rules":"entries.json"}],
        "marker_prefix":"K", "check":inline["check"], "decision_model":inline["decision_model"],
        "limits":inline["limits"], "evidence":{"ast_grep":"sgconfig.yml", "kinds":inline["evidence"]["kinds"]}
    });
    std::fs::write(root.path().join("sunstack.lint.json"), cli.to_string()).unwrap();

    // Act: the real file-config entry used by the CLI and the direct SDK entry.
    let cli = sunstack_lint::run(
        sunstack_lint::LintRequest {
            config_path: Some(root.path().join("sunstack.lint.json")),
            root: Some(root.path().to_path_buf()),
            paths: vec![],
            scope: ScopeChoice::All,
            max_warnings: None,
            evidence_listing: Some(None),
        },
        CancellationToken::new(),
        None,
    )
    .await;
    let sdk = inspect(&inline, root.path()).await;

    // Assert: the CLI listing's additive fields preserve the report's partial ending.
    assert_eq!(cli.status, LintStatus::Incomplete, "{cli:?}");
    let listing = cli.evidence_listing.unwrap();
    let InspectionOutcome::Incomplete { report } = sdk else {
        panic!("{sdk:?}")
    };
    assert_eq!(listing.status, LintStatus::Incomplete);
    assert_eq!(listing.pieces, report.pieces);
    assert_eq!(listing.limits, report.limits);
    assert_eq!(
        serde_json::to_value(listing.file_counts).unwrap(),
        serde_json::to_value(report.file_counts).unwrap()
    );
    assert_eq!(
        serde_json::to_value(listing.exclusions).unwrap(),
        serde_json::to_value(report.exclusions).unwrap()
    );
    assert_eq!(
        serde_json::to_value(listing.incomplete).unwrap(),
        serde_json::to_value(report.incomplete).unwrap()
    );
    assert_eq!(listing.diagnostics.len(), report.diagnostics.len());
    assert_eq!(
        listing.diagnostics[0].message,
        report.diagnostics[0].message
    );
    assert_eq!(listing.usage.model_requests, 0);

    // File decoding retains its original empty-rulebook refusal, even for --evidence.
    let mut invalid_cli = serde_json::from_str::<Value>(
        &std::fs::read_to_string(root.path().join("sunstack.lint.json")).unwrap(),
    )
    .unwrap();
    invalid_cli["rulebook"] = json!([]);
    std::fs::write(
        root.path().join("sunstack.lint.json"),
        invalid_cli.to_string(),
    )
    .unwrap();
    let refused = sunstack_lint::run(
        sunstack_lint::LintRequest {
            config_path: Some(root.path().join("sunstack.lint.json")),
            root: Some(root.path().to_path_buf()),
            paths: vec![],
            scope: ScopeChoice::All,
            max_warnings: None,
            evidence_listing: Some(None),
        },
        CancellationToken::new(),
        None,
    )
    .await;
    assert_eq!(refused.status, LintStatus::Invalid);
    let listing = refused.evidence_listing.unwrap();
    assert_eq!(listing.status, LintStatus::Invalid);
    assert!(!listing.diagnostics.is_empty());
    assert!(listing.pieces.is_empty());
}

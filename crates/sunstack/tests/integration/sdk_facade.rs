#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The shared SDK facade over real files, real admission and the real executor.
//! Mechanical rules keep these runs keyless; model-backed runs live in the SDK package.
use serde_json::{Value, json};
use std::path::Path;
use sunstack::sdk_host::{
    self, CredentialSource,
    report_projection::RunOutcome,
    sdk_contracts::{
        CheckOutcome, Configuration, DiagnosticCode, LoadOptions, LoadOutcome, RunRequest, Scope,
    },
};
use sunstack_decisions::CancellationToken;
use sunstack_evidence::{DefinitionOrigin, SnapshotAcquisition, SnapshotRepresentation};

const RULE_A: &str = "^Use when";
const RULE_B: &str = "^Wrong";

fn entries(pattern: &str) -> Value {
    json!({"K001":{"name":"description","evidence":"skill","severity":"error","decision":{"start":"starts","steps":{"starts":{"mechanical_check":{"tool":"regex","field":"description","pattern":pattern},"yes":"pass","no":{"finding":"Description must begin with Use when.","at":"description"}}}}}})
}

/// A CLI project whose definitions live in nested folders, and a separate checked tree.
fn write_project(definitions: &Path, checked: &Path) {
    std::fs::create_dir_all(definitions.join("lint/evidence/rules/nested")).unwrap();
    std::fs::create_dir_all(definitions.join("lint/evidence/utils")).unwrap();
    std::fs::create_dir_all(definitions.join("lint/standards")).unwrap();
    std::fs::create_dir_all(checked.join("docs")).unwrap();
    std::fs::write(
        checked.join("docs/one.md"),
        "---\ndescription: Wrong opening\n---\n\n# One\n",
    )
    .unwrap();
    std::fs::write(
        definitions.join("lint/evidence/sgconfig.yml"),
        "ruleDirs: [rules]\nutilDirs: [utils]\n",
    )
    .unwrap();
    std::fs::write(
        definitions.join("lint/evidence/utils/front.yml"),
        "id: front-matter\nlanguage: Markdown\nrule: {matches: frontmatter}\n",
    )
    .unwrap();
    std::fs::write(
        definitions.join("lint/evidence/rules/nested/skill.yml"),
        "id: skill\nlanguage: md\nrule: {matches: front-matter}\n",
    )
    .unwrap();
    std::fs::write(
        definitions.join("lint/standards/rulebook.md"),
        "Description must begin with Use when. <!-- K001 -->\n",
    )
    .unwrap();
    std::fs::write(
        definitions.join("lint/standards/rules.json"),
        serde_json::to_vec_pretty(&entries(RULE_A)).unwrap(),
    )
    .unwrap();
    std::fs::write(
        definitions.join("lint/sunstack.lint.json"),
        serde_json::to_vec_pretty(&json!({
            "rulebook":[{"markdown":"standards/rulebook.md","rules":"standards/rules.json"}],
            "marker_prefix":"K",
            "check":{"files":["docs/*.md"]},
            "evidence":{"ast_grep":"evidence/sgconfig.yml","kinds":{"skill":{"rule":"skill","fields":{"description":{"from":"yaml","key":"description"}}}}},
            "decision_model":{"model":"jev-latest"},
            "limits":{"max_file_bytes":18446744073709551615u64}
        }))
        .unwrap(),
    )
    .unwrap();
}

async fn load(directory: &Path) -> sunstack::sdk_host::sdk_contracts::LoadedConfiguration {
    match sdk_host::load(LoadOptions {
        directory: directory.to_path_buf(),
        config_path: None,
    })
    .await
    {
        LoadOutcome::Loaded { value } => value,
        other => panic!("load failed: {other:?}"),
    }
}

/// The loaded value reaches the facade as the transported JSON form TypeScript sends.
fn transported(value: &impl serde::Serialize) -> Configuration {
    sdk_host::decode_configuration(&serde_json::to_string(value).unwrap()).unwrap()
}

async fn run_all(configuration: Configuration, root: &Path) -> RunOutcome {
    sdk_host::run_with_credential(
        configuration,
        RunRequest {
            root: root.to_path_buf(),
            scope: Scope::All,
            paths: vec![],
        },
        CancellationToken::new(),
        CredentialSource::Supplied(None),
    )
    .await
}

fn finding_count(outcome: &RunOutcome) -> usize {
    match outcome {
        RunOutcome::Completed { report } => report.findings.len(),
        other => panic!("run did not complete: {other:?}"),
    }
}

fn completed_sources(outcome: &RunOutcome) -> Vec<sunstack_evidence::DefinitionSnapshot> {
    match outcome {
        RunOutcome::Completed { report } => report.sources.clone(),
        other => panic!("run did not complete: {other:?}"),
    }
}

#[tokio::test]
async fn load_discovers_upward_and_resolves_references_from_the_config_directory() {
    let definitions = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    write_project(definitions.path(), checked.path());
    let nested = definitions.path().join("lint/standards");
    let loaded = load(&nested).await;
    let configuration = &loaded.configuration;
    let config_file = definitions
        .path()
        .join("lint/sunstack.lint.json")
        .canonicalize()
        .unwrap();
    assert_eq!(configuration.id, config_file.display().to_string());
    assert_eq!(configuration.base_directory, config_file.parent().unwrap());
    assert_eq!(configuration.rulebooks.len(), 1);
    assert!(configuration.evidence.matchers.contains_key("skill"));
    assert!(
        configuration
            .evidence
            .utilities
            .contains_key("front-matter")
    );
    // File aliases are normalized to the canonical upstream language spelling.
    assert_eq!(
        configuration.evidence.matchers["skill"].value["language"],
        "Markdown"
    );
    // u64::MAX survives load and transport exactly.
    let encoded = serde_json::to_value(&loaded).unwrap();
    assert_eq!(
        encoded["configuration"]["limits"]["max_file_bytes"].to_string(),
        "18446744073709551615"
    );
    let rules_file = config_file.parent().unwrap().join("standards/rules.json");
    let receipt = loaded
        .sources
        .iter()
        .find(|source| source.origin == DefinitionOrigin::file(&rules_file))
        .unwrap();
    assert_eq!(
        receipt.content,
        std::fs::read_to_string(&rules_file).unwrap()
    );
    assert_eq!(
        receipt.sha256,
        sunstack_evidence::content_digest(&std::fs::read(&rules_file).unwrap())
    );
    assert_eq!(receipt.acquisition, SnapshotAcquisition::Read);
}

#[tokio::test]
async fn held_snapshot_copy_and_reload_each_execute_their_own_definitions() {
    let definitions = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    write_project(definitions.path(), checked.path());
    let project = definitions.path().join("lint");
    let held = load(&project).await;

    // Disk changes to B after the load.
    let rules_file = project.join("standards/rules.json");
    std::fs::write(&rules_file, serde_json::to_vec(&entries(RULE_B)).unwrap()).unwrap();

    let held_run = run_all(transported(&held.configuration), checked.path()).await;
    assert_eq!(finding_count(&held_run), 1, "held A still executes A");
    let held_entries = completed_sources(&held_run)
        .into_iter()
        .find(|source| source.origin == DefinitionOrigin::file(rules_file.canonicalize().unwrap()))
        .unwrap();
    assert_eq!(
        held_entries.acquisition,
        SnapshotAcquisition::SuppliedSnapshot
    );
    assert!(
        held_entries.content.contains(RULE_A),
        "provenance keeps A's bytes"
    );

    // A copied value with an edited rule executes the edit with edited provenance.
    let mut copy = serde_json::to_value(&held.configuration).unwrap();
    copy["rulebooks"][0]["entries"]["value"]["K001"]["decision"]["steps"]["starts"]["mechanical_check"]
        ["pattern"] = json!(RULE_B);
    let edited_run = run_all(transported(&copy), checked.path()).await;
    assert_eq!(
        finding_count(&edited_run),
        0,
        "edited copy executes its edit"
    );
    let edited = completed_sources(&edited_run)
        .into_iter()
        .find(|source| matches!(source.origin, DefinitionOrigin::Edited { .. }))
        .expect("edited origin");
    assert_eq!(
        edited.representation,
        SnapshotRepresentation::NormalizedValue
    );
    assert!(edited.content.contains(RULE_B));
    match &edited.origin {
        DefinitionOrigin::Edited { based_on, .. } => assert_eq!(
            based_on,
            &sunstack_evidence::SourceLocator::File {
                path: rules_file.canonicalize().unwrap()
            }
        ),
        _ => unreachable!(),
    }

    // The original held value is unchanged by the copy, and only reload sees B.
    let again = run_all(transported(&held.configuration), checked.path()).await;
    assert_eq!(finding_count(&again), 1);
    let reloaded = load(&project).await;
    assert_eq!(
        finding_count(&run_all(transported(&reloaded.configuration), checked.path()).await),
        0
    );
}

#[tokio::test]
async fn deleting_definition_files_after_load_does_not_affect_the_snapshot() {
    let definitions = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    write_project(definitions.path(), checked.path());
    let project = definitions.path().join("lint");
    let held = load(&project).await;
    std::fs::remove_dir_all(project.join("evidence")).unwrap();
    std::fs::remove_dir_all(project.join("standards")).unwrap();
    assert_eq!(
        finding_count(&run_all(transported(&held.configuration), checked.path()).await),
        1
    );
}

#[tokio::test]
async fn tampered_source_metadata_is_refused_and_receipts_never_execute() {
    let definitions = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    write_project(definitions.path(), checked.path());
    let held = load(&definitions.path().join("lint")).await;

    let mut malformed = serde_json::to_value(&held.configuration).unwrap();
    malformed["rulebooks"][0]["entries"]["original"]["content"] = json!("{not json");
    let outcome = sdk_host::check(transported(&malformed), CancellationToken::new()).await;
    let CheckOutcome::Invalid { diagnostics } = outcome else {
        panic!("malformed original must be invalid: {outcome:?}");
    };
    assert_eq!(diagnostics[0].code, DiagnosticCode::CaptureMetadataInvalid);

    let mut wrong_id = serde_json::to_value(&held.configuration).unwrap();
    wrong_id["evidence"]["matchers"]["skill"]["original"]["definition_id"] = json!("absent");
    let CheckOutcome::Invalid { diagnostics } =
        sdk_host::check(transported(&wrong_id), CancellationToken::new()).await
    else {
        panic!("ambiguous definition id must be invalid");
    };
    assert_eq!(diagnostics[0].code, DiagnosticCode::CaptureMetadataInvalid);

    // The informational receipt is not configuration input at all.
    let mut with_receipt = serde_json::to_value(&held.configuration).unwrap();
    with_receipt["sources"] = serde_json::to_value(&held.sources).unwrap();
    let refused = sdk_host::decode_configuration(&with_receipt.to_string()).unwrap_err();
    assert_eq!(refused[0].code, DiagnosticCode::UnknownField);
}

fn inline_configuration(base: &Path, pattern: &str) -> Value {
    json!({
        "id": "inline-standards",
        "base_directory": base,
        "marker_prefix": "K",
        "check": {"files": ["docs/*.md"]},
        "decision_model": {"model": "jev-latest"},
        "limits": {"max_file_bytes": 18446744073709551615u64},
        "evidence": {
            "kind": "inline",
            "utilities": {"front-matter": {"language": "Markdown", "rule": {"matches": "frontmatter"}}},
            "matchers": {"skill": {"language": "Markdown", "rule": {"matches": "front-matter"}}},
            "kinds": {"skill": {"rule": "skill", "fields": {"description": {"from": "yaml", "key": "description"}}}}
        },
        "rulebooks": [{
            "kind": "inline",
            "id": "ownership-standards",
            "markdown": "Description must begin with Use when. <!-- K001 -->\n",
            "entries": entries(pattern)
        }]
    })
}

#[tokio::test]
async fn inline_definitions_run_without_definition_files_and_match_the_file_fixture() {
    let definitions = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    write_project(definitions.path(), checked.path());
    let empty_base = tempfile::tempdir().unwrap();
    let inline = inline_configuration(empty_base.path(), RULE_A);
    let inline_outcome = run_all(transported(&inline), checked.path()).await;
    let file_outcome = run_all(
        transported(&load(&definitions.path().join("lint")).await.configuration),
        checked.path(),
    )
    .await;
    let (RunOutcome::Completed { report: inline }, RunOutcome::Completed { report: file }) =
        (&inline_outcome, &file_outcome)
    else {
        panic!("both runs complete: {inline_outcome:?} {file_outcome:?}");
    };
    assert_eq!(inline.findings.len(), 1);
    // Only declared origin representations may differ.
    let semantic = |report: &serde_json::Value| {
        let mut report = report.clone();
        for key in ["configuration", "sources", "findings", "requests"] {
            report.as_object_mut().unwrap().remove(key);
        }
        report
    };
    assert_eq!(
        semantic(&serde_json::to_value(inline).unwrap()),
        semantic(&serde_json::to_value(file).unwrap())
    );
    assert_eq!(inline.findings[0].file, file.findings[0].file);
    assert_eq!(inline.findings[0].message, file.findings[0].message);
    // Every inline source is visibly inline, with real content and digests.
    assert_eq!(
        inline.configuration,
        DefinitionOrigin::Inline {
            id: "inline-standards".into(),
            label: "inline-standards".into()
        }
    );
    let ids = inline
        .sources
        .iter()
        .map(|source| match &source.origin {
            DefinitionOrigin::Inline { id, .. } => id.clone(),
            other => panic!("non-inline source {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "inline-standards",
            "ownership-standards",
            "inline-standards#/rulebooks/0/entries",
            "inline-standards#/evidence/utilities/front-matter",
            "inline-standards#/evidence/matchers/skill",
        ]
    );
    for source in &inline.sources {
        assert_eq!(
            source.sha256,
            sunstack_evidence::content_digest(source.content.as_bytes())
        );
    }
    // The standard's related place names the inline rulebook, never a file.
    let standard = &inline.findings[0].related[0];
    assert_eq!(
        serde_json::to_value(&standard.location).unwrap()["origin"]["kind"],
        "inline"
    );
}

#[tokio::test]
async fn check_needs_no_credential_and_refuses_like_the_cli() {
    let base = tempfile::tempdir().unwrap();
    let mut asking = inline_configuration(base.path(), RULE_A);
    asking["rulebooks"][0]["entries"]["K001"] = json!({"name":"description","evidence":"skill","severity":"error","questions":{"complies":{"type":"noul","instructions":{"check":"description","task":"Does it comply?"}}},"decision":{"start":"complies","steps":{"complies":{"ask":"complies","yes_at_or_above":0.8,"no_at_or_below":0.3,"yes":"pass","no":{"finding":"No.","at":"description"},"uncertain":"uncertain"}}}});
    let CheckOutcome::Valid { summary, .. } =
        sdk_host::check(transported(&asking), CancellationToken::new()).await
    else {
        panic!("asking config admits without a credential");
    };
    assert_eq!(summary.active_rule_count, 1);
    assert_eq!(summary.evidence_kind_count, 1);

    // A missing marker refuses in both paths.
    let mut orphan = inline_configuration(base.path(), RULE_A);
    orphan["rulebooks"][0]["markdown"] = json!("No marker here.\n");
    let CheckOutcome::Invalid { diagnostics } =
        sdk_host::check(transported(&orphan), CancellationToken::new()).await
    else {
        panic!("missing marker refuses");
    };
    assert!(diagnostics[0].message.contains("no rulebook marker"));
    assert_eq!(
        diagnostics[0].origin,
        Some(DefinitionOrigin::Inline {
            id: "inline-standards#/rulebooks/0/entries".into(),
            label: "inline-standards#/rulebooks/0/entries".into()
        })
    );

    // Nested unknown rule keys refuse; wrapper-level unknown keys are ignored.
    let mut nested = inline_configuration(base.path(), RULE_A);
    nested["evidence"]["matchers"]["skill"]["rule"]["matchez"] = json!("x");
    assert!(matches!(
        sdk_host::check(transported(&nested), CancellationToken::new()).await,
        CheckOutcome::Invalid { .. }
    ));
    let mut wrapper = inline_configuration(base.path(), RULE_A);
    wrapper["evidence"]["matchers"]["skill"]["unrelated"] = json!(true);
    assert!(matches!(
        sdk_host::check(transported(&wrapper), CancellationToken::new()).await,
        CheckOutcome::Valid { .. }
    ));
    // An inline id is refused as the file path refuses inline_rule ids.
    let mut declared = inline_configuration(base.path(), RULE_A);
    declared["evidence"]["matchers"]["skill"]["id"] = json!("skill");
    assert!(matches!(
        sdk_host::check(transported(&declared), CancellationToken::new()).await,
        CheckOutcome::Invalid { .. }
    ));
    // Strict Sunstack objects refuse unknown keys at transport.
    let mut strict = inline_configuration(base.path(), RULE_A);
    strict["check"]["unexpected"] = json!(1);
    assert_eq!(
        sdk_host::decode_configuration(&strict.to_string()).unwrap_err()[0].code,
        DiagnosticCode::UnknownField
    );
}

#[test]
fn wide_integers_are_exact_and_constructors_stay_in_rust() {
    let base = tempfile::tempdir().unwrap();
    let mut value = inline_configuration(base.path(), RULE_A);
    let configuration = sdk_host::decode_configuration(&value.to_string()).unwrap();
    assert_eq!(
        serde_json::to_value(&configuration).unwrap()["limits"]["max_file_bytes"].to_string(),
        "18446744073709551615"
    );
    let text = value
        .to_string()
        .replace("18446744073709551615", "18446744073709551616");
    assert!(sdk_host::decode_configuration(&text).is_err());
    value["limits"]["max_file_bytes"] = json!(0);
    let refused = sdk_host::decode_configuration(&value.to_string()).unwrap_err();
    assert_eq!(refused[0].json_path, ["limits", "max_file_bytes"]);
}

#[tokio::test]
async fn relative_roots_and_definition_bases_are_refused_before_acquisition() {
    let base = tempfile::tempdir().unwrap();
    let configuration = transported(&inline_configuration(base.path(), RULE_A));
    let outcome = sdk_host::run_with_credential(
        configuration,
        RunRequest {
            root: "relative/root".into(),
            scope: Scope::All,
            paths: vec![],
        },
        CancellationToken::new(),
        CredentialSource::Supplied(None),
    )
    .await;
    let RunOutcome::Invalid {
        diagnostics,
        report,
    } = outcome
    else {
        panic!("relative root refused");
    };
    assert!(report.is_none());
    assert_eq!(diagnostics[0].code, DiagnosticCode::UnsupportedInputs);

    let mut relative_base = inline_configuration(base.path(), RULE_A);
    relative_base["base_directory"] = json!("relative");
    assert!(matches!(
        sdk_host::check(transported(&relative_base), CancellationToken::new()).await,
        CheckOutcome::Invalid { .. }
    ));
}

#[tokio::test]
async fn missing_definition_files_are_unavailable_and_malformed_ones_invalid() {
    let base = tempfile::tempdir().unwrap();
    let missing = sdk_host::load(LoadOptions {
        directory: base.path().to_path_buf(),
        config_path: None,
    })
    .await;
    assert!(
        matches!(missing, LoadOutcome::Unavailable { .. }),
        "{missing:?}"
    );

    let definitions = tempfile::tempdir().unwrap();
    let checked = tempfile::tempdir().unwrap();
    write_project(definitions.path(), checked.path());
    let project = definitions.path().join("lint");
    std::fs::remove_file(project.join("standards/rulebook.md")).unwrap();
    let LoadOutcome::Unavailable { diagnostics } = sdk_host::load(LoadOptions {
        directory: project.clone(),
        config_path: None,
    })
    .await
    else {
        panic!("missing rulebook is unavailable");
    };
    assert_eq!(diagnostics[0].code, DiagnosticCode::SourceNotFound);

    write_project(definitions.path(), checked.path());
    std::fs::write(project.join("evidence/rules/nested/skill.yml"), "id: [").unwrap();
    assert!(matches!(
        sdk_host::load(LoadOptions {
            directory: project,
            config_path: None,
        })
        .await,
        LoadOutcome::Invalid { .. }
    ));
}

#[tokio::test]
async fn already_cancelled_calls_start_no_work() {
    let base = tempfile::tempdir().unwrap();
    let mut missing_files = inline_configuration(base.path(), RULE_A);
    missing_files["rulebooks"] =
        json!([{"kind":"files","markdown":"absent.md","entries":"absent.json"}]);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert!(matches!(
        sdk_host::check(transported(&missing_files), cancellation.clone()).await,
        CheckOutcome::Cancelled { .. }
    ));
    let outcome = sdk_host::run_with_credential(
        transported(&missing_files),
        RunRequest {
            root: base.path().to_path_buf(),
            scope: Scope::All,
            paths: vec![],
        },
        cancellation,
        CredentialSource::Supplied(None),
    )
    .await;
    assert!(matches!(outcome, RunOutcome::Cancelled { report: None }));
}

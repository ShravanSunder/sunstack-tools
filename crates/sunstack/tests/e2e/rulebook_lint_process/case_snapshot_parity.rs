use super::*;
use std::{collections::BTreeSet, os::unix::fs::PermissionsExt};
use sunstack_decisions::CancellationToken;

fn shell_argument(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn snapshot_observer(directory: &Path, evidence: &Path) -> PathBuf {
    let isolated = support::isolated_command(directory);
    let binary = Path::new(isolated.as_std().get_args().next().unwrap())
        .canonicalize()
        .unwrap();
    let digest = sunstack_evidence::content_digest(&std::fs::read(&binary).unwrap());
    let node = std::process::Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .unwrap();
    assert!(
        node.status.success(),
        "Node is required for the explicitly run observer proof"
    );
    let node = String::from_utf8(node.stdout).unwrap();
    let script = support::workspace_root()
        .join("scripts/verify/sunstack-rule/case-dispatch/snapshot-observer.ts");
    let wrapper = directory.join("snapshot-observer");
    let command = format!(
        "#!/bin/sh\nexec env -u OPENROUTER_API_KEY SUNSTACK_CREDENTIAL_STORE=off CASE_DISPATCH_REAL_CLI={} CASE_DISPATCH_EVIDENCE={} CASE_DISPATCH_EXPECTED_SHA256={} {} {} \"$@\"\n",
        shell_argument(binary.to_str().unwrap()),
        shell_argument(evidence.to_str().unwrap()),
        shell_argument(&digest),
        shell_argument(node.trim()),
        shell_argument(script.to_str().unwrap()),
    );
    std::fs::write(&wrapper, command).unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    wrapper
}

fn require_snapshot_receipts(evidence: &Path) -> (usize, usize, usize) {
    let mut snapshots = 0;
    let mut selected = 0;
    let mut ordinary = 0;
    for entry in std::fs::read_dir(evidence).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_name().to_string_lossy().starts_with("snapshot-") {
            continue;
        }
        let process = read(&entry.path().join("process.json"));
        assert_eq!(process["ordinaryExit"], 6, "{process}");
        assert_eq!(process["selectedExit"], 6, "{process}");
        assert!(process["ordinarySignal"].is_null() && process["selectedSignal"].is_null());
        let parity = read(&entry.path().join("parity.json"));
        assert_eq!(
            parity["parity"],
            "matched_raw_request_and_native_input_sha256"
        );
        let selected_count = parity["selectedExecutionOwners"].as_u64().unwrap() as usize;
        assert!(
            selected_count > 0,
            "an empty prepared set cannot prove parity"
        );
        assert_eq!(parity["owners"].as_array().unwrap().len(), selected_count);
        selected += selected_count;
        ordinary += parity["ordinaryExecutionOwners"].as_u64().unwrap() as usize;
        snapshots += 1;
    }
    assert!(snapshots > 0);
    (snapshots, selected, ordinary)
}

// These opt-in proofs compose the declared verifier wrapper and installed Node
// dependencies. Ordinary Rust CI still owns the native HTTP/selection regressions.
#[tokio::test]
#[ignore = "requires the case-dispatch Node observer dependencies; run explicitly for this proof"]
async fn selected_case_requests_equal_ordinary_requests_inside_the_same_historical_snapshot() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("docs")).unwrap();
    std::fs::write(
        root.path().join("docs/one.md"),
        "Selected café.\n\nExcluded β.\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.path().join("evidence/rules")).unwrap();
    std::fs::write(
        root.path().join("evidence/sgconfig.yml"),
        "ruleDirs: [rules]\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("evidence/rules/paragraph.yml"),
        "id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    )
    .unwrap();
    save(
        &root.path().join("sunstack.lint.json"),
        &json!({
            "rulebook":[{"markdown":"parity.md","rules":"parity.json"}],"marker_prefix":"K",
            "check":{"files":["docs/*.md"]},"decision_model":{"model":"jev-latest"},
            "evidence":{"ast_grep":"evidence/sgconfig.yml","kinds":{"paragraph":{"rule":"paragraph","fields":{"body":{"from":"remainder"}}}}}
        }),
    );
    std::fs::write(
        root.path().join("parity.md"),
        "Paragraph is clear. <!-- K001 -->\n",
    )
    .unwrap();
    save(
        &root.path().join("parity.json"),
        &json!({"K001":{
            "name":"clear","evidence":"paragraph","questions":{"clear":{"type":"noul","instructions":{"check":"body","task":"Is this clear?"}}},
            "decision":{"start":"clear","steps":{"clear":{"ask":"clear","yes_at_or_above":0.8,"no_at_or_below":0.2,"yes":"pass","no":{"finding":"Unclear."},"uncertain":"uncertain"}}}
        }}),
    );
    let revision = super::case_runs::commit_tree(root.path());
    let make_case = |id, line, text: &str| {
        json!({"id":id,"revision":revision,"set":"development","label_from":"blind-agents",
        "kind":"rule","rule":"clear","file":"docs/one.md","lines":[line,line],"text_sha256":sunstack_evidence::content_digest(text.as_bytes()),"expected":"pass"})
    };
    save(
        &root.path().join("parity.cases.json"),
        &json!({"cases":[
            make_case("selected",1,"Selected café.\n"),make_case("same-owner",1,"Selected café.\n"),make_case("excluded",3,"Excluded β.\n")
        ]}),
    );
    let owned = tempfile::tempdir().unwrap();
    let evidence = owned.path().join("evidence");
    std::fs::create_dir(&evidence).unwrap();
    let wrapper = snapshot_observer(owned.path(), &evidence);
    let report = sunstack_lint::case_runs::prepare_cases(
        &root.path().join("parity.cases.json"),
        Some(&root.path().join("sunstack.lint.json")),
        Some("development"),
        &["selected".into(), "same-owner".into()],
        &wrapper,
        CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        report.exit_code(),
        6,
        "{}",
        serde_json::to_string(&report).unwrap()
    );
    assert!(report.errors.is_empty());
    assert_eq!(report.cases.len(), 2);
    assert_eq!(report.not_selected, vec!["excluded"]);
    assert_eq!(report.cases[0].result_id, report.cases[1].result_id);
    assert_eq!(require_snapshot_receipts(&evidence), (1, 1, 2));
}

#[tokio::test]
#[ignore = "requires explicitly commissioned corpus manifest/copies/config/evidence environment paths"]
async fn declared_case_dispatch_corpus_matches_ordinary_requests_in_engine_owned_snapshots() {
    let required = |name| {
        PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("missing {name}")))
            .canonicalize()
            .unwrap()
    };
    let manifest = required("CASE_DISPATCH_CORPUS_MANIFEST");
    let copies = required("CASE_DISPATCH_CORPUS_COPIES");
    let config = required("CASE_DISPATCH_CORPUS_CONFIG");
    let evidence = required("CASE_DISPATCH_CORPUS_EVIDENCE");
    let directory = tempfile::tempdir().unwrap();
    let wrapper = snapshot_observer(directory.path(), &evidence);
    let manifest = read(&manifest);
    let mut total = 0;
    let mut inventories = Vec::new();
    for (file, selection) in manifest["files"].as_object().unwrap() {
        let ids = selection["score"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert!(!ids.is_empty());
        let report = sunstack_lint::case_runs::prepare_cases(
            &copies.join(file),
            Some(&config),
            Some("development"),
            &ids,
            &wrapper,
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
        save(
            &evidence.join(format!("{file}.report.json")),
            &serde_json::to_value(&report).unwrap(),
        );
        assert_eq!(
            report.exit_code(),
            6,
            "{}",
            serde_json::to_string(&report).unwrap()
        );
        assert!(report.errors.is_empty());
        assert_eq!(
            report
                .cases
                .iter()
                .map(|case| case.id.clone())
                .collect::<BTreeSet<_>>(),
            ids.iter().cloned().collect()
        );
        assert!(report.cases.iter().all(|case| matches!(
            case.outcome,
            sunstack_lint::case_runs::CaseOutcome::Prepared
        )));
        total += report.cases.len();
        inventories.push(json!({"file":file,"selected_ids":ids,"requests":report.runs.iter().map(|run| run.report.requests.len()).sum::<usize>(),"not_selected":report.not_selected}));
    }
    let (snapshots, selected, ordinary) = require_snapshot_receipts(&evidence);
    assert_eq!(selected, total);
    assert!(ordinary >= selected);
    save(
        &evidence.join("corpus-parity-summary.json"),
        &json!({"selected_cases":total,"snapshots":snapshots,"selected_execution_owners":selected,"ordinary_execution_owners":ordinary,"inventories":inventories}),
    );
}

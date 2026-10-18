#![allow(clippy::unwrap_used, clippy::panic)]
#[path = "support/rule_fixture.rs"]
mod fixture;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use sunstack_decisions::{BackendSelection, CancellationToken, DecisionBackend};
use sunstack_evidence::{AdapterRegistry, MarkdownByteRange, MarkdownDocument, content_digest};
use sunstack_rules::*;

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
struct ScanMatch {
    start: usize,
    end: usize,
    text: String,
}
#[derive(Deserialize)]
struct ScanCase {
    source: String,
    sha256: String,
    matches: BTreeMap<String, Vec<ScanMatch>>,
}
#[derive(Deserialize)]
struct ScanOracle {
    cli_version: String,
    rules: BTreeMap<String, Value>,
    cases: Vec<ScanCase>,
}
fn oracle() -> ScanOracle {
    let oracle: ScanOracle =
        serde_json::from_str(include_str!("fixtures/mechanical-ast/scan-oracle.json")).unwrap();
    assert_eq!(oracle.cli_version, "ast-grep 0.51.0");
    oracle
}
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read_case(case: &ScanCase) -> String {
    let text = std::fs::read_to_string(workspace().join(&case.source)).unwrap();
    assert_eq!(content_digest(text.as_bytes()), case.sha256);
    text
}
struct RefusingFactory(BackendSelection);
impl RuleBackendFactory for RefusingFactory {
    fn selection(&self) -> &BackendSelection {
        &self.0
    }
    fn prepare(self: Arc<Self>) -> Result<Arc<dyn DecisionBackend>, DiagnosticList> {
        Err(DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ))
    }
}
fn rule(id: &str, kind: &str, ast_grep: &Value) -> Value {
    let mut rule = fixture::rule(id);
    rule["evidence"][0]["select"] = json!({"kind":"markdown_units","mode":"unit","units":[kind]});
    rule["verdict"] = json!({"kind":"mechanical","explanation":"A structural standard","check":{"kind":"ast_grep","ast_grep":ast_grep}});
    rule
}
async fn evaluate(root: &Path) -> RuleInvocationReport {
    let mut wire =
        serde_json::to_value(fixture::input(root, "not-a-constructible-endpoint")).unwrap();
    wire["limits"]["max_matches"] = json!(2048);
    wire["limits"]["max_file_bytes"] = json!(262144);
    wire["limits"]["max_total_bytes"] = json!(2097152);
    let input: RuleInvocationInput = serde_json::from_value(wire).unwrap();
    let backend = Arc::new(RefusingFactory(input.backend.clone()));
    let outcome = evaluate_rules(
        input,
        RuleExecutionContext {
            backend,
            adapters: AdapterRegistry::builtins().unwrap(),
            cancellation: CancellationToken::new(),
        },
    )
    .await;
    let RuleInvocationOutcome::CompleteReport { report } = outcome else {
        panic!("{outcome:?}")
    };
    report
}

#[test]
fn metadata_matcher_equals_full_document_scan_on_explicit_header_and_body_ranges() {
    let oracle = oracle();
    let mut violations = 0;
    for case in &oracle.cases {
        let text = read_case(case);
        let end = if text.starts_with("---\n") {
            text.find("\n---\n").unwrap() + 5
        } else {
            0
        };
        let serialized = serde_json::from_value::<ast_grep_config::SerializableRule>(
            oracle.rules["metadata_description"].clone(),
        )
        .unwrap();
        let matcher =
            ast_grep_config::DeserializeEnv::new(ast_grep_language::SupportLang::Markdown)
                .deserialize_rule(serialized)
                .unwrap();
        let document = MarkdownDocument::parse(&text);
        for range in [
            MarkdownByteRange { start: 0, end },
            MarkdownByteRange {
                start: end,
                end: text.len(),
            },
        ] {
            let (actual, limited) = document.matching_spans(&matcher, &range, 2048);
            assert!(!limited);
            let actual = actual
                .into_iter()
                .map(|span| ScanMatch {
                    start: span.byte_start,
                    end: span.byte_end,
                    text: text[span.byte_start..span.byte_end].into(),
                })
                .collect::<Vec<_>>();
            let expected = case.matches["metadata_description"]
                .iter()
                .filter(|found| found.start >= range.start && found.end <= range.end)
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(
                actual, expected,
                "{} {}..{}",
                case.source, range.start, range.end
            );
            violations += actual.len();
        }
    }
    assert_eq!(
        violations, 1,
        "the adverse description must reach a real positive match"
    );
}

#[tokio::test]
async fn real_unit_checks_equal_full_original_scan_on_five_skills_and_positive_negative_controls() {
    let oracle = oracle();
    let mut positives = 0;
    let mut calls = 0;
    let mut list_positives = 0;
    let mut corpus_files = 0;
    for case in &oracle.cases {
        if case.source.contains("metadata-pass") || case.source.contains("metadata-fail") {
            continue;
        }
        let text = read_case(case);
        let root = tempfile::tempdir().unwrap();
        fixture::write_set(
            root.path(),
            &[
                rule("local.call", "paragraph", &oracle.rules["call_paragraph"]),
                rule(
                    "local.inside.paragraph",
                    "paragraph",
                    &oracle.rules["inside_list"],
                ),
                rule(
                    "local.inside.item",
                    "list_item",
                    &oracle.rules["inside_list"],
                ),
            ],
            vec![],
        );
        std::fs::write(root.path().join("source.md"), &text).unwrap();
        let report = evaluate(root.path()).await;
        assert_eq!(report.measurements.model_requests, 0);
        assert!(report.is_complete());
        if case.source.contains("markdown-skill-corpus") {
            corpus_files += 1;
            assert!(!report.occurrences.is_empty());
        }
        for result in &report.occurrences {
            let RuleOccurrenceResult::Checked { key, outcome, .. } = result else {
                panic!("{result:?}")
            };
            let RuleOccurrenceTarget::MarkdownUnit { range, source } = key.target.as_ref() else {
                panic!("expected unit")
            };
            assert!(
                serde_json::to_string(source)
                    .unwrap()
                    .contains(&case.sha256)
            );
            let id = if key.rule_id.as_str() == "local.call" {
                "call_paragraph"
            } else {
                "inside_list"
            };
            let expected = case.matches[id]
                .iter()
                .filter(|found| found.start >= range.start && found.end <= range.end)
                .cloned()
                .collect::<Vec<_>>();
            let LocalSettlement::Completed {
                result,
                observations,
            } = outcome
            else {
                panic!("{outcome:?}")
            };
            let actual = observations
                .iter()
                .map(|observation| match observation {
                    LocalObservation::AstMatch { span } => ScanMatch {
                        start: span.byte_start,
                        end: span.byte_end,
                        text: text[span.byte_start..span.byte_end].into(),
                    },
                })
                .collect::<Vec<_>>();
            assert_eq!(
                actual,
                expected,
                "{} {} {}..{}",
                case.source,
                key.rule_id.as_str(),
                range.start,
                range.end
            );
            assert_eq!(
                *result,
                if expected.is_empty() {
                    MechanicalResult::Pass
                } else {
                    MechanicalResult::Fail
                }
            );
            positives += usize::from(!actual.is_empty());
            calls += usize::from(id == "call_paragraph" && !actual.is_empty());
            list_positives +=
                usize::from(key.rule_id.as_str() == "local.inside.item" && !actual.is_empty());
        }
    }
    assert_eq!(corpus_files, 5);
    assert!(positives > 0);
    assert!(calls > 0);
    assert!(list_positives > 0);
}

#[tokio::test]
async fn ancestor_outside_the_selected_paragraph_remains_visible_but_a_sibling_match_is_not_attributed()
 {
    let oracle = oracle();
    let case = oracle
        .cases
        .iter()
        .find(|case| case.source.ends_with("ancestor-sibling.md"))
        .unwrap();
    let text = read_case(case);
    assert_eq!(case.matches["inside_list"].len(), 2);
    let root = tempfile::tempdir().unwrap();
    let mut selected = rule("local.inside", "paragraph", &oracle.rules["inside_list"]);
    selected["evidence"][0]["select"]["starts_with"] = json!("Target");
    fixture::write_set(root.path(), &[selected], vec![]);
    std::fs::write(root.path().join("source.md"), &text).unwrap();
    let report = evaluate(root.path()).await;
    assert_eq!(report.occurrences.len(), 1);
    let RuleOccurrenceResult::Checked { key, outcome, .. } = &report.occurrences[0] else {
        panic!("expected checked")
    };
    let RuleOccurrenceTarget::MarkdownUnit { range, .. } = key.target.as_ref() else {
        panic!("expected paragraph")
    };
    assert_eq!((range.start, range.end), (2, text.find('\n').unwrap() + 1));
    let LocalSettlement::Completed {
        result: MechanicalResult::Fail,
        observations,
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    assert_eq!(observations.len(), 1);
    let LocalObservation::AstMatch { span } = &observations[0];
    assert_eq!((span.byte_start, span.byte_end), (range.start, range.end));
    assert!(!text[span.byte_start..span.byte_end].contains("Sibling"));
}

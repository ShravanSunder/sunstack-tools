#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
#[path = "candidate_evidence_fixture.rs"]
mod evidence_fixture;
use serde_json::{Value, json};
use sunstack_evals::*;
use sunstack_evidence::*;
use sunstack_rules::*;

pub const LABEL_SENTINEL: &str = "PRIVATE_LABEL_ONLY_SENTINEL_CASE_0";
pub enum FrozenFact {
    Document,
    Unavailable,
    Payload(Value),
}
pub struct EvalFixture {
    pub root: tempfile::TempDir,
    pub input: Value,
    pub manifest: std::path::PathBuf,
    pub specification: FreezeInput,
}
impl EvalFixture {
    pub fn new(endpoint: &str) -> Self {
        Self::configured(endpoint, 1, FrozenFact::Document)
    }
    pub fn configured(endpoint: &str, records: usize, fact: FrozenFact) -> Self {
        let root = tempfile::tempdir().unwrap();
        let document = !matches!(fact, FrozenFact::Payload(_));
        let mut input = evidence_fixture::fixture(root.path(), endpoint, document);
        std::fs::write(root.path().join("owner.md"), "OWNER_PUBLIC_BOUND_FACT\n").unwrap();
        let text=(0..records).map(|position|json!({"type":"assistant","message":{"id":format!("message_{position}"),"content":format!("MATCH public occurrence {position}")}}).to_string()+"\n").collect::<String>();
        std::fs::write(root.path().join("first.jsonl"), text).unwrap();
        let source: SourceInput =
            serde_json::from_value(input["sources"][0]["source"].clone()).unwrap();
        let identity = source.source_identity().unwrap();
        let bytes = std::fs::read(root.path().join("first.jsonl")).unwrap();
        let registry = AdapterRegistry::builtins().unwrap();
        let format = TranscriptFormatName::new("claude_code_jsonl").unwrap();
        let lines = registry
            .event_reader(&format)
            .unwrap()
            .lines(&CapturedTranscript {
                bytes: &bytes,
                source_identity: identity.as_str(),
            });
        let projected = project_positioned(&identity, &lines).unwrap();
        assert_eq!(projected.records.len(), records);
        let discovered = projected.records.as_slice().discover(None, 0).unwrap();
        assert_eq!(discovered.matches.len(), records);
        let binding = match fact {
            FrozenFact::Document => {
                json!({"kind":"bound","binding":{"kind":"document","repository":"facts","path":"owner.md"}})
            }
            FrozenFact::Unavailable => json!({"kind":"unavailable","reason":"withheld"}),
            FrozenFact::Payload(value) => {
                json!({"kind":"bound","binding":{"kind":"payload","value":value}})
            }
        };
        let evidence = serde_json::from_value(json!({"fact":binding})).unwrap();
        let specification = FreezeInput {
            definition_root: root.path().into(),
            entry: CandidateDefinitions::RuleSets {
                rule_sets: vec!["set.json".into()],
            },
            payload_schemas: serde_json::from_value(input["payload_schemas"].clone()).unwrap(),
            rule: RuleRef {
                rule_set: "set.json".into(),
                rule_id: RuleId::new("candidate.one").unwrap(),
            },
            split: Split::HeldOut,
            sources: vec![CaseSource {
                source_id: CaseSourceId::new("source_1").unwrap(),
                source,
                evidence,
            }],
            labels: discovered
                .matches
                .iter()
                .enumerate()
                .map(|(position, matched)| LabeledCase {
                    case_id: CaseId::new(if position == 0 {
                        LABEL_SENTINEL.to_owned()
                    } else {
                        format!("PRIVATE_LABEL_ONLY_SENTINEL_CASE_{position}")
                    })
                    .unwrap(),
                    source: CaseSourceId::new("source_1").unwrap(),
                    project: None,
                    source_anchor: matched.occurrence.clone(),
                    question: QuestionRef::Verdict {},
                    label: Label::Yes,
                    split: Split::HeldOut,
                })
                .collect(),
            source_files: if document {
                vec![
                    root.path().join("first.jsonl"),
                    root.path().join("owner.md"),
                ]
            } else {
                vec![root.path().join("first.jsonl")]
            },
            definition_files: vec![
                root.path().join("set.json"),
                root.path().join("rules.jsonl"),
            ],
        };
        let _frozen = freeze_cases(&root.path().join("case-set"), &specification).unwrap();
        let manifest = root.path().join("case-set/manifest.json");
        for key in [
            "sources",
            "definitions",
            "definition_root",
            "payload_schemas",
            "context_settings",
            "samples_per_pass",
        ] {
            input.as_object_mut().unwrap().remove(key);
        }
        input["case_set"] = json!(manifest);
        input["evidence"] = if matches!(
            specification.sources[0].evidence.values().next().unwrap(),
            EvidenceBinding::Unavailable { .. }
        ) {
            json!({"fact":{"kind":"bound","binding":{"kind":"document","repository":"facts","path":"owner.md"}}})
        } else {
            json!({"fact":{"kind":"unavailable","reason":"withheld"}})
        };
        Self {
            root,
            input,
            manifest,
            specification,
        }
    }
    pub fn freeze_as(&mut self, name: &str) {
        let _frozen = freeze_cases(&self.root.path().join(name), &self.specification).unwrap();
        self.manifest = self.root.path().join(name).join("manifest.json");
        self.input["case_set"] = json!(self.manifest);
    }
    pub fn labels_path(&self) -> std::path::PathBuf {
        serde_json::from_value::<CaseSetManifest>(read_json(&self.manifest))
            .unwrap()
            .labels
            .path
    }
    pub fn assert_label_confinement(&self) {
        assert!(
            std::fs::read_to_string(self.labels_path())
                .unwrap()
                .contains(LABEL_SENTINEL)
        );
        let manifest: CaseSetManifest = serde_json::from_value(read_json(&self.manifest)).unwrap();
        let mut paths = vec![manifest.sources.path];
        paths.extend(manifest.source_files.into_iter().map(|file| file.path));
        paths.extend(manifest.definition_files.into_iter().map(|file| {
            self.manifest
                .parent()
                .unwrap()
                .join("definitions")
                .join(file.path)
        }));
        paths.extend(manifest.payload_schemas.into_iter().map(|schema| {
            self.manifest
                .parent()
                .unwrap()
                .join("definitions")
                .join(schema.path)
        }));
        for path in paths {
            assert!(
                !String::from_utf8_lossy(&std::fs::read(&path).unwrap()).contains(LABEL_SENTINEL),
                "label marker in {}",
                path.display()
            );
        }
        assert!(!self.input.to_string().contains(LABEL_SENTINEL));
    }
    pub fn mutate_live_rule(&self) {
        let path = self.root.path().join("rules.jsonl");
        let mut rule = read_json(&path);
        rule["verdict"]["threshold"] = json!(1.0);
        rule["verdict"]["question"]["instructions"] = json!("LIVE_RULE_MUTATION_NEVER_USED");
        write_json(&path, &rule);
    }
    pub fn candidate_input(&self) -> Value {
        let frozen = load_case_set(&self.manifest).unwrap();
        let mut value = self.input.clone();
        value.as_object_mut().unwrap().remove("case_set");
        value["sources"] = serde_json::to_value(
            frozen
                .sources()
                .iter()
                .map(|source| SourceEntry {
                    source: source.source.clone(),
                    evidence: source.evidence.clone(),
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        value["definitions"] = serde_json::to_value(&frozen.manifest().entry).unwrap();
        value["definition_root"] = json!(frozen.definition_root());
        value["payload_schemas"] = serde_json::to_value(
            frozen
                .manifest()
                .payload_schemas
                .iter()
                .map(|schema| PayloadSchemaPath {
                    name: schema.name.clone(),
                    path: schema.path.clone(),
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        value
    }
}
pub fn read_json(path: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
pub fn write_json(path: &std::path::Path, value: &Value) {
    std::fs::write(path, value.to_string()).unwrap();
}
pub fn response(output: &std::process::Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["result"]["operation"], "score_eval");
    value
}

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/eval_process_fixture.rs"]
mod eval_fixture;
#[path = "support/signal_process.rs"]
mod signal_process;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use eval_fixture::{EvalFixture, LABEL_SENTINEL, read_json, response, write_json};
use serde_json::{Value, json};
use sunstack_evals::*;
use sunstack_evidence::*;

fn native_anchor(source: &SourceInput, path: &std::path::Path) -> SourceOccurrence {
    let identity = source.source_identity().unwrap();
    let bytes = std::fs::read(path).unwrap();
    let registry = AdapterRegistry::builtins().unwrap();
    let (registry, format) = match source {
        SourceInput::SessionPair { .. } => (
            registry,
            TranscriptFormatName::new("claude_code_jsonl").unwrap(),
        ),
        SourceInput::CustomEntry { schema, .. } => registry.with_custom_schema(schema).unwrap(),
        _ => unreachable!(),
    };
    let lines = registry
        .event_reader(&format)
        .unwrap()
        .lines(&CapturedTranscript {
            bytes: &bytes,
            source_identity: identity.as_str(),
        });
    let positioned = project_positioned(&identity, &lines).unwrap();
    assert_eq!(positioned.records.len(), 1);
    let discovered = positioned.records.as_slice().discover(None, 0).unwrap();
    assert_eq!(discovered.matches.len(), 1);
    discovered.matches[0].occurrence.clone()
}
fn configure_two_native_sources(fixture: &mut EvalFixture, custom: bool) {
    let root = fixture.root.path();
    let path = root.join("rules.jsonl");
    let mut rule = read_json(&path);
    rule.as_object_mut().unwrap().remove("mechanical_pass");
    write_json(&path, &rule);
    let original = fixture.specification.sources[0].clone();
    let original_label = fixture.specification.labels[0].clone();
    let mut sources = Vec::new();
    let mut labels = Vec::new();
    for (position, name) in ["first", "second"].into_iter().enumerate() {
        let path = root.join(format!("{name}.jsonl"));
        let source = if custom {
            let schema=serde_json::from_value(json!({"format":"jsonl","record_schema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]},"fields":{"text":"/text"}})).unwrap();
            std::fs::write(
                &path,
                json!({"text":"MATCH identical public occurrence"}).to_string() + "\n",
            )
            .unwrap();
            SourceInput::CustomEntry {
                location: path.clone(),
                schema,
            }
        } else {
            std::fs::write(&path,json!({"type":"assistant","message":{"id":"identical_message","content":"MATCH identical public occurrence"}}).to_string()+"\n").unwrap();
            SourceInput::SessionPair {
                provider: ProviderName::new("claude_code").unwrap(),
                session: SessionId::new(name).unwrap(),
            }
        };
        let source_id = CaseSourceId::new(format!("source_{}", position + 1)).unwrap();
        let mut label = original_label.clone();
        label.case_id = CaseId::new(format!("{LABEL_SENTINEL}_{position}")).unwrap();
        label.source = source_id.clone();
        label.source_anchor = native_anchor(&source, &path);
        labels.push(label);
        sources.push(CaseSource {
            source_id,
            source,
            evidence: original.evidence.clone(),
        });
    }
    fixture.specification.sources = sources;
    fixture.specification.labels = labels;
    fixture.specification.source_files = vec![
        root.join("first.jsonl"),
        root.join("second.jsonl"),
        root.join("owner.md"),
    ];
    fixture.freeze_as("native-association-cases");
}
async fn start(fixture: &EvalFixture) -> tokio::task::JoinHandle<std::process::Output> {
    use tokio::io::AsyncWriteExt;
    let directory = fixture.root.path().to_owned();
    let bytes = serde_json::to_vec(
        &json!({"protocol_version":1,"operation":"score_eval","input":fixture.input}),
    )
    .unwrap();
    let (mut process, stderr, transcript) = signal_process::ready_process(&directory).await;
    tokio::spawn(async move {
        let mut stdin = process.stdin.take().unwrap();
        stdin.write_all(&bytes).await.unwrap();
        drop(stdin);
        let output = signal_process::collect_output(process, stderr, transcript).await;
        support::record_output(&directory, &bytes, &output);
        output
    })
}
fn cross_attach_label(fixture: &EvalFixture) {
    let path = fixture.labels_path();
    let text = std::fs::read_to_string(&path).unwrap();
    let mut labels = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    labels[0]["source"] = json!(fixture.specification.sources[1].source_id);
    let bytes = labels
        .into_iter()
        .map(|label| label.to_string() + "\n")
        .collect::<String>();
    std::fs::write(&path, &bytes).unwrap();
    let mut manifest = read_json(&fixture.manifest);
    manifest["labels"]["content_identity"] = json!(content_digest(bytes.as_bytes()));
    write_json(&fixture.manifest, &manifest);
}
async fn assert_binary_source_association(custom: bool) {
    let mut server = transport::controlled::ControlledServer::start().await;
    let mut fixture = EvalFixture::new(&server.endpoint);
    configure_two_native_sources(&mut fixture, custom);

    let mut valid = start(&fixture).await;
    for _ in 0..2 {
        let request = tokio::select! {
            output=&mut valid=>panic!("valid native sources stopped before their requests: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
            request=server.next()=>request,
        };
        assert!(!request.body.to_string().contains(LABEL_SENTINEL));
        request.noul(0.9);
    }
    let output = valid.await.unwrap();
    let wire = response(&output, 0);
    assert_eq!(wire["result"]["outcome"]["kind"], "eval_report");
    assert_eq!(
        wire["result"]["outcome"]["report"]["end_result"]["true_findings"],
        json!([
            "PRIVATE_LABEL_ONLY_SENTINEL_CASE_0_0",
            "PRIVATE_LABEL_ONLY_SENTINEL_CASE_0_1"
        ])
    );

    cross_attach_label(&fixture);
    let mut forged = start(&fixture).await;
    for _ in 0..2 {
        let request = tokio::select! {
            output=&mut forged=>panic!("record association labels were interpreted before bundle completion: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
            request=server.next()=>request,
        };
        assert!(!request.body.to_string().contains(LABEL_SENTINEL));
        request.noul(0.9);
    }
    let output = forged.await.unwrap();
    let wire = response(&output, 2);
    assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
    assert!(wire["result"]["outcome"].get("report").is_none());
    assert!(
        wire["result"]["outcome"]["diagnostics"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "invalid_configuration"
                && diagnostic["location"]["field_path"]
                    == json!(["labels", "source_anchor", "record"]))
    );
    assert_eq!(server.finish().await, 4);
}
#[tokio::test]
async fn binary_session_foreign_record_refuses_after_native_bundle() {
    assert_binary_source_association(false).await;
}
#[tokio::test]
async fn binary_custom_foreign_record_refuses_after_native_bundle() {
    assert_binary_source_association(true).await;
}

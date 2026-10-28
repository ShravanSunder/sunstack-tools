#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[tokio::test]
async fn recipe_only_provider_and_cursor_keep_actual_source_attribution() {
    for cursor in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
        let provider = if cursor { "cursor" } else { "codex-copy" };
        let text = if cursor {
            json!({"role":"assistant","message":{"content":[{"type":"text","text":"MATCH CURSOR_SOURCE"},{"type":"tool_use","name":"Read","input":{"path":"public.rs"}}]}})
        } else {
            json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"MATCH RECIPE_SOURCE"}]}})
        };
        std::fs::create_dir(root.path().join("copied")).unwrap();
        let path = root.path().join("copied/source-one.jsonl");
        std::fs::write(&path, text.to_string() + "\n").unwrap();
        input["sources"] =
            json!([{"source":{"kind":"session_pair","provider":provider,"session":"one"}}]);
        input["recipes"] = json!([{"provider":provider,"locate":{"roots":[root.path().join("copied")],"pattern":"source-{session}.jsonl"},"read":{"format":if cursor {"cursor_export_jsonl"} else {"codex_rollout_jsonl"}},"map":{"fields":{}}}]);
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let request = tokio::select! {output=&mut run=>panic!("source refused {}",String::from_utf8_lossy(&output.unwrap().stdout)), request=server.next()=>request};
        assert!(request.body.to_string().contains(if cursor {
            "CURSOR_SOURCE"
        } else {
            "RECIPE_SOURCE"
        }));
        request.noul(0.9);
        let output = run.await.unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let report = &wire["result"]["outcome"]["report"];
        assert_eq!(
            report["inputs"][0]["location"]["path"],
            json!(path.canonicalize().unwrap())
        );
        let source: sunstack_evidence::SourceInput = serde_json::from_value(
            json!({"kind":"session_pair","provider":provider,"session":"one"}),
        )
        .unwrap();
        assert_eq!(
            report["inputs"][0]["input_id"],
            source.source_identity().unwrap().as_str()
        );
        assert_eq!(report["candidates"].as_array().unwrap().len(), 1);
        let candidate = &report["candidates"][0];
        assert_eq!(candidate["outcome"]["kind"], "finding");
        let identity = source.source_identity().unwrap();
        let record = candidate["occurrence"]["record"].as_str().unwrap();
        assert!(record.starts_with(&format!("{}:", identity.as_str())));
        assert_eq!(candidate["candidate_id"]["occurrence"], record);
        assert_eq!(
            candidate["occurrence"]["matched_at"],
            json!({"line":1,"byte_offset":0})
        );
        assert_eq!(
            candidate["occurrence"]["positions"],
            json!([{"line":1,"byte_offset":0}])
        );
        assert_eq!(candidate["occurrence"]["author"], "assistant");
        assert_eq!(server.finish().await, 1);
    }
}

#[tokio::test]
async fn custom_json_and_jsonl_map_valid_records_and_keep_unfit_positions() {
    for format in ["jsonl", "json"] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
        let records = [
            json!({"quality":1,"payload":{"text":"MATCH CUSTOM_FIRST"}}),
            json!({"quality":"wrong","payload":{"text":"MATCH WRONG_SCHEMA"}}),
            json!({"quality":1}),
            json!({"quality":2,"payload":{"text":"MATCH CUSTOM_LAST"}}),
        ];
        let path = root.path().join(format!("custom.{format}"));
        let text = if format == "jsonl" {
            records
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        } else {
            format!(
                "[\n{}\n]",
                records
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join(",\n")
            )
        };
        std::fs::write(&path, &text).unwrap();
        input["sources"] = json!([{"source":{"kind":"custom_entry","location":path,"schema":{"format":format,"record_schema":{"type":"object","properties":{"quality":{"type":"integer"}},"required":["quality"]},"fields":{"text":"/payload/text"}}}}]);
        let rule_path = root.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&rule_path).unwrap()).unwrap();
        rule["mechanical_pass"]["predicate"]["field"] = json!("/body/fields/text");
        std::fs::write(rule_path, rule.to_string()).unwrap();
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..2 {
            let request = tokio::select! {output=&mut run=>panic!("custom source stopped {}",String::from_utf8_lossy(&output.unwrap().stdout)),request=server.next()=>request};
            let body = request.body.to_string();
            assert!(!body.contains("WRONG_SCHEMA"));
            let first = body.contains("CUSTOM_FIRST");
            assert_ne!(first, body.contains("CUSTOM_LAST"));
            assert!(seen.insert(first));
            request.noul(0.9);
        }
        let output = run.await.unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let report = &wire["result"]["outcome"]["report"];
        assert_eq!(report["inputs"][0]["records"], 2);
        assert_eq!(report["inputs"][0]["capture"], "complete");
        let gaps = report["inputs"][0]["unfit"].as_array().unwrap();
        assert_eq!(gaps.len(), 2, "{wire}");
        let expected_gaps = records[1..3]
            .iter()
            .map(|record| text.find(&record.to_string()).unwrap() as u64)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            gaps.iter()
                .map(|gap| gap["position"].as_u64().unwrap())
                .collect::<std::collections::BTreeSet<_>>(),
            expected_gaps
        );
        let entries = report["candidates"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        let expected = [
            (0, if format == "jsonl" { 1 } else { 2 }),
            (3, if format == "jsonl" { 4 } else { 5 }),
        ]
        .into_iter()
        .map(|(index, line)| (text.find(&records[index].to_string()).unwrap() as u64, line))
        .collect::<std::collections::BTreeMap<_, _>>();
        let mut seen = std::collections::BTreeSet::new();
        for entry in entries {
            let offset = entry["occurrence"]["matched_at"]["byte_offset"]
                .as_u64()
                .unwrap();
            assert!(seen.insert(offset));
            let line = expected.get(&offset).unwrap();
            assert_eq!(
                entry["occurrence"]["matched_at"],
                json!({"byte_offset":offset,"line":line})
            );
            assert_eq!(
                entry["occurrence"]["positions"],
                json!([{"byte_offset":offset,"line":line}])
            );
            let record = entry["occurrence"]["record"].as_str().unwrap();
            assert!(record.starts_with(&format!(
                "{}:",
                report["inputs"][0]["input_id"].as_str().unwrap()
            )));
            assert_eq!(entry["candidate_id"]["occurrence"], record);
            assert_eq!(entry["outcome"]["kind"], "finding");
            assert_eq!(entry["occurrence"]["author"], "custom");
        }
        assert_eq!(seen, expected.keys().copied().collect());
        assert_eq!(server.finish().await, 2);
    }
}

#[tokio::test]
async fn missing_ambiguous_and_partial_document_never_report_complete_empty() {
    for mode in ["missing", "ambiguous", "partial_document"] {
        let root = tempfile::tempdir().unwrap();
        let server = ControlledServer::start().await;
        let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
        input["sources"].as_array_mut().unwrap().truncate(1);
        if mode == "ambiguous" {
            for directory in ["a", "b"] {
                std::fs::create_dir(root.path().join(directory)).unwrap();
                std::fs::write(root.path().join(directory).join("first.jsonl"), "{}\n").unwrap();
            }
            input["recipes"][0]["locate"]["pattern"] = json!("**/{session}.jsonl");
        } else if mode == "partial_document" {
            let path = root.path().join("partial.json");
            std::fs::write(&path, "[{\"text\":\"unfinished").unwrap();
            input["sources"] = json!([{"source":{"kind":"custom_entry","location":path,"schema":{"format":"json","record_schema":{"type":"object"},"fields":{"text":"/text"}}}}]);
        }
        let output =
            support::run_without_credential(root.path(), "evaluate_candidates", input).await;
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let report = &wire["result"]["outcome"]["report"];
        assert_eq!(report["completeness"]["kind"], "partial");
        assert_eq!(report["candidates"], json!([]));
        assert_eq!(report["usage"]["requests"], 0);
        if mode == "partial_document" {
            assert_eq!(report["inputs"][0]["capture"], "incomplete");
            assert_eq!(report["inputs"][0]["unfit"], json!([]));
            assert_eq!(report["completeness"]["stopped_by"], "incomplete_input");
        } else {
            assert_eq!(report["inputs"][0]["location"]["kind"], "unlocatable");
            assert_eq!(report["inputs"][0]["location"]["reason"], mode);
        }
        assert_eq!(server.finish().await, 0);
    }
}

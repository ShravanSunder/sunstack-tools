#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/candidate_fixture.rs"]
mod candidate_fixture;
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[tokio::test]
async fn mapped_source_privacy_reaches_actual_requests_per_rule_without_opaque_bytes() {
    for kind in ["thinking", "analysis_channel", "encrypted_reasoning"] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = candidate_fixture::fixture(root.path(), &server.endpoint);
        let (format, raw, public, private) = match kind {
            "thinking" => (
                "claude_code_jsonl",
                json!({"type":"assistant","message":{"id":"mixed","content":[{"type":"text","text":"MATCH PUBLIC_MAPPING"},{"type":"thinking","thinking":"PRIVATE_MAPPING"}]}}),
                "/message/content/0/text",
                "/message/content/1/thinking",
            ),
            "analysis_channel" => (
                "codex_rollout_jsonl",
                json!({"type":"response_item","public":"MATCH PUBLIC_MAPPING","payload":{"type":"message","role":"assistant","channel":"analysis","content":[{"type":"output_text","text":"PRIVATE_MAPPING"}]}}),
                "/public",
                "/payload/content/0/text",
            ),
            _ => (
                "codex_rollout_jsonl",
                json!({"type":"response_item","public":"MATCH PUBLIC_MAPPING","payload":{"type":"reasoning","encrypted_content":"OPAQUE_MAPPING_NEVER_MATERIALIZE","summary":[{"type":"summary_text","text":"PRIVATE_MAPPING"}]}}),
                "/public",
                "/payload/summary/0/text",
            ),
        };
        std::fs::write(root.path().join("first.jsonl"), raw.to_string() + "\n").unwrap();
        input["sources"] =
            json!([{"source":{"kind":"session_pair","provider":"mapped-host","session":"first"}}]);
        input["recipes"] = json!([{"provider":"mapped-host","locate":{"roots":[root.path()],"pattern":"{session}.jsonl"},"read":{"format":format},"map":{"fields":{"public":public,"private":private}}}]);
        if kind == "encrypted_reasoning" {
            input["recipes"][0]["map"]["fields"]["opaque"] = json!("/payload/encrypted_content");
        }
        let path = root.path().join("rules.jsonl");
        let mut ordinary: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        ordinary["rule_id"] = json!("ordinary");
        ordinary["mechanical_pass"]["predicate"]["field"] = json!("/body/fields/public");
        ordinary["verdict"]["question"]["instructions"] = json!("ORDINARY_MAPPING_RULE");
        let mut granted = ordinary.clone();
        granted["rule_id"] = json!("granted");
        granted["private_access"] = json!([kind]);
        granted["verdict"]["question"]["instructions"] = json!("GRANTED_MAPPING_RULE");
        std::fs::write(path, format!("{ordinary}\n{granted}\n")).unwrap();
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..2 {
            let request = tokio::select! {output=&mut run=>panic!("mapped source stopped {}",String::from_utf8_lossy(&output.unwrap().stdout)),request=server.next()=>request};
            let body = request.body.to_string();
            let is_granted = body.contains("GRANTED_MAPPING_RULE");
            assert_ne!(is_granted, body.contains("ORDINARY_MAPPING_RULE"));
            assert!(seen.insert(is_granted));
            assert!(body.contains("PUBLIC_MAPPING"));
            assert_eq!(body.contains("PRIVATE_MAPPING"), is_granted);
            assert!(!body.contains("OPAQUE_MAPPING_NEVER_MATERIALIZE"));
            request.noul(0.9);
        }
        let output = run.await.unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            !String::from_utf8_lossy(&output.stdout).contains("OPAQUE_MAPPING_NEVER_MATERIALIZE")
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let report = &wire["result"]["outcome"]["report"];
        let entries = report["candidates"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0]["candidate_id"]["occurrence"],
            entries[1]["candidate_id"]["occurrence"]
        );
        for entry in entries {
            let granted = entry["rule"]["rule_id"] == "granted";
            assert_eq!(entry["outcome"]["kind"], "finding");
            assert_eq!(entry["occurrence"]["author"], "assistant");
            assert_eq!(
                entry["privacy_view"]["grants"],
                if granted { json!([kind]) } else { json!([]) }
            );
            if !granted {
                assert!(!entry.to_string().contains("PRIVATE_MAPPING"));
            }
        }
        let mut outside_granted = report.clone();
        outside_granted["candidates"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["rule"]["rule_id"] != "granted");
        assert!(
            !outside_granted.to_string().contains("PRIVATE_MAPPING"),
            "private text outside the authorized candidate"
        );
        assert_eq!(
            report["privacy"]["granted"],
            json!([{"rule":{"rule_set":root.path().join("set.json").canonicalize().unwrap(),"rule_id":"granted"},"private_kind":kind}])
        );
        assert!(
            report["privacy"]["left_out"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["private_kind"] == kind && entry["count"].as_u64().unwrap() > 0)
        );
        assert_eq!(server.finish().await, 2);
    }
}

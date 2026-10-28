#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../../sunstack-rules/tests/support/controlled_http.rs"]
mod controlled_http;
#[path = "support/candidate_evidence_fixture.rs"]
mod evidence_fixture;
mod support;
use controlled_http::ControlledServer;
use serde_json::{Value, json};

#[tokio::test]
async fn multiple_hits_in_one_record_keep_one_candidate_and_the_first_permitted_attribution() {
    for (granted, public_hit) in [(false, true), (true, true), (false, false)] {
        let root = tempfile::tempdir().unwrap();
        let mut server = ControlledServer::start().await;
        let mut input = evidence_fixture::record_fixture(root.path(), &server.endpoint);
        input["sources"].as_array_mut().unwrap().truncate(1);
        let lines = [
            json!({"type":"assistant","message":{"id":"stream","content":[{"type":"thinking","thinking":"PRIVATE_MATCH MATCH"}]}}),
            json!({"type":"assistant","message":{"id":"stream","content":[{"type":"text","text":if public_hit {"PUBLIC_MATCH MATCH MATCH"} else {"public ordinary"}}]}}),
        ].map(|line|line.to_string()+"\n");
        std::fs::write(root.path().join("first.jsonl"), lines.concat()).unwrap();
        let path = root.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        // Reverse authored predicate order: attribution must follow field/span order instead.
        rule["mechanical_pass"] = json!({"predicate":{"kind":"any","predicates":[
            {"kind":"regex","field":"/body/blocks/1/block/text","pattern":"MATCH"},
            {"kind":"regex","field":"/body/blocks/0/block/text","pattern":"MATCH"}
        ]}});
        if granted {
            rule["private_access"] = json!(["thinking"]);
        }
        std::fs::write(path, rule.to_string()).unwrap();
        let directory = root.path().to_owned();
        let mut run =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let output = if granted || public_hit {
            let request = tokio::select! {
                output = &mut run => panic!("permitted record never reached judge: {}",String::from_utf8_lossy(&output.unwrap().stdout)),
                request = server.next() => request,
            };
            assert_eq!(request.body.to_string().contains("PRIVATE_MATCH"), granted);
            request.noul(0.9);
            tokio::select! {
                output = &mut run => output.unwrap(),
                request = server.next() => panic!("same record was judged twice: {}",request.body),
            }
        } else {
            tokio::select! {
                output = &mut run => output.unwrap(),
                request = server.next() => panic!("private-only record was judged: {}",request.body),
            }
        };
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
        let entries = wire["result"]["outcome"]["report"]["candidates"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(
            entry["candidate_id"]["occurrence"],
            entry["occurrence"]["record"]
        );
        if granted || public_hit {
            assert_eq!(entry["outcome"]["kind"], "finding");
            assert_eq!(
                entry["occurrence"]["matched_at"]["line"],
                if granted { 1 } else { 2 }
            );
            assert_eq!(
                entry["occurrence"]["span"]["byte_start"],
                if granted { 8 } else { 7 }
            );
            assert_eq!(server.finish().await, 1);
        } else {
            assert_eq!(
                entry["outcome"],
                json!({"kind":"not_processed","reason":"left_out_for_privacy"})
            );
            assert_eq!(entry["occurrence"]["matched_at"]["line"], 1);
            assert_eq!(server.finish().await, 0);
        }
    }
}

#[tokio::test]
async fn repeated_source_entries_are_refused_before_capture_or_dispatch() {
    for (changed_evidence, duplicate_position) in [(false, 1), (true, 1), (true, 2)] {
        let root = tempfile::tempdir().unwrap();
        let server = ControlledServer::start().await;
        let mut input = evidence_fixture::fixture(root.path(), &server.endpoint, false);
        input["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"DENY_CONTEXT"}}}});
        let mut repeated = input["sources"][0].clone();
        if changed_evidence {
            repeated["evidence"] = json!({"fact":{"kind":"bound","binding":{"kind":"payload","value":{"decision":"ALLOW_CONTEXT"}}}});
        }
        if duplicate_position == 2 {
            input["sources"].as_array_mut().unwrap().push(json!({"source":{"kind":"session_pair","provider":"claude_code","session":"distinct"}}));
        }
        input["sources"].as_array_mut().unwrap().push(repeated);
        // Even absent source bytes cannot turn this caller error into a capture outcome.
        std::fs::remove_file(root.path().join("first.jsonl")).unwrap();
        let output =
            support::run_without_credential(root.path(), "evaluate_candidates", input).await;
        let outcome = evidence_fixture::report(&output, 2);
        assert_eq!(outcome["kind"], "invalid");
        assert!(outcome.get("report").is_none());
        assert!(
            outcome["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["location"]["field_path"]
                    == json!(["sources", duplicate_position.to_string(), "source"]))
        );
        assert_eq!(server.finish().await, 0);
    }
}

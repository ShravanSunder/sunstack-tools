use super::narrowing_sample_runtime::{
    answer_until_process_finishes, candidate_line, narrowing_answer, noul_response,
    report_from_process_output, start_candidate_process,
};
use super::{controlled_http::ControlledServer, narrowing_sample_fixture};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[tokio::test]
async fn narrowing_samples_use_rule_projected_private_views_and_exclude_opaque_bytes() {
    let root = tempfile::tempdir().unwrap();
    let mut server = ControlledServer::start().await;
    let mut input = narrowing_sample_fixture::record_run(
        root.path(),
        &server.endpoint,
        &[(0..3)
            .map(|index| format!("MATCH PUBLIC_SAMPLE_CANARY_{index}"))
            .collect()],
        1,
    );
    input["sources"] = json!([{
        "source":{"kind":"session_pair","provider":"mapped-host","session":"first"}
    }]);
    input["recipes"] = json!([{
        "provider":"mapped-host",
        "locate":{"roots":[root.path()],"pattern":"{session}.jsonl"},
        "read":{"format":"codex_rollout_jsonl"},
        "map":{"fields":{
            "public":"/public",
            "private":"/payload/summary/0/text",
            "opaque":"/payload/encrypted_content"
        }}
    }]);
    let source_records = (0..3)
        .map(|index| {
            json!({
                "type":"response_item",
                "public":format!("MATCH PUBLIC_SAMPLE_CANARY_{index}"),
                "payload":{
                    "type":"reasoning",
                    "encrypted_content":format!("OPAQUE_SAMPLE_CANARY_{index}"),
                    "summary":[{"type":"summary_text","text":format!("PRIVATE_SAMPLE_CANARY_{index}")}]
                }
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        root.path().join("first.jsonl"),
        format!("{source_records}\n"),
    )
    .unwrap();

    let rules_path = root.path().join("rules.jsonl");
    let mut restricted: Value =
        serde_json::from_slice(&std::fs::read(&rules_path).unwrap()).unwrap();
    restricted["rule_id"] = json!("candidate.restricted");
    restricted["mechanical_pass"]["predicate"]["field"] = json!("/body/fields/public");
    restricted["private_access"] = json!([]);
    restricted["narrowing"]["question"]["instructions"] = json!("NARROW_RESTRICTED");
    let mut granted = restricted.clone();
    granted["rule_id"] = json!("candidate.granted");
    granted["private_access"] = json!(["encrypted_reasoning"]);
    granted["narrowing"]["question"]["instructions"] = json!("NARROW_GRANTED");
    std::fs::write(rules_path, format!("{restricted}\n{granted}\n")).unwrap();

    let mut process = start_candidate_process(root.path(), input).await;
    let mut request_views = BTreeMap::<bool, BTreeMap<usize, String>>::new();
    let output = answer_until_process_finishes(&mut server, &mut process, |body| {
        let serialized = body.to_string();
        let granted_view = serialized.contains("NARROW_GRANTED");
        assert_ne!(granted_view, serialized.contains("NARROW_RESTRICTED"));
        let record_index = (0..3)
            .find(|index| serialized.contains(&format!("PUBLIC_SAMPLE_CANARY_{index}")))
            .unwrap_or_else(|| panic!("request omitted its public record: {serialized}"));
        assert_eq!(
            (0..3)
                .filter(|index| serialized.contains(&format!("PUBLIC_SAMPLE_CANARY_{index}")))
                .count(),
            1,
            "request must project one source record: {serialized}"
        );
        assert_eq!(
            serialized.contains(&format!("PRIVATE_SAMPLE_CANARY_{record_index}")),
            granted_view
        );
        assert!(!serialized.contains("OPAQUE_SAMPLE_CANARY"));
        request_views
            .entry(granted_view)
            .or_default()
            .insert(record_index, serialized.clone());
        noul_response(body, |question| {
            if question.to_string().contains("NARROW_") {
                0.1
            } else {
                0.9
            }
        })
    })
    .await;
    let report = report_from_process_output(&output);
    assert_eq!(server.finish().await, 6);

    let candidates = report["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 6);
    for candidate in candidates {
        assert_eq!(candidate["outcome"]["kind"], "dropped");
        assert_eq!(candidate["outcome"]["at"]["kind"], "narrowing");
        assert_eq!(
            narrowing_answer(candidate)["answer"]["yes_probability"],
            0.1
        );
    }
    for rule_id in ["candidate.restricted", "candidate.granted"] {
        let rule_candidates = candidates
            .iter()
            .filter(|candidate| candidate["rule"]["rule_id"] == rule_id)
            .collect::<Vec<_>>();
        assert_eq!(rule_candidates.len(), 3);
        let mut candidate_lines = rule_candidates
            .iter()
            .map(|candidate| candidate_line(candidate))
            .collect::<Vec<_>>();
        candidate_lines.sort_unstable();
        assert_eq!(candidate_lines, vec![1, 2, 3]);
    }
    assert_eq!(
        request_views.len(),
        2,
        "restricted and granted views both run"
    );
    for granted_view in [false, true] {
        let views = request_views.get(&granted_view).unwrap();
        assert_eq!(views.keys().copied().collect::<Vec<_>>(), vec![0, 1, 2]);
        for (record_index, request) in views {
            assert!(request.contains(if granted_view {
                "NARROW_GRANTED"
            } else {
                "NARROW_RESTRICTED"
            }));
            assert!(request.contains(&format!("PUBLIC_SAMPLE_CANARY_{record_index}")));
            assert_eq!(
                request.contains(&format!("PRIVATE_SAMPLE_CANARY_{record_index}")),
                granted_view
            );
            assert!(!request.contains("OPAQUE_SAMPLE_CANARY"));
        }
    }
    let samples = report["dropped_samples"].as_array().unwrap();
    assert_eq!(samples.len(), 2);
    let sample_rule_ids = samples
        .iter()
        .map(|sample| sample["rule"]["rule_id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        sample_rule_ids,
        std::collections::BTreeSet::from(["candidate.restricted", "candidate.granted"])
    );
    for sample in samples {
        assert_eq!(sample["pass"], "narrowing");
        let granted_view = sample["rule"]["rule_id"] == "candidate.granted";
        let candidate = candidates
            .iter()
            .find(|candidate| {
                candidate["rule"] == sample["rule"]
                    && candidate["occurrence"] == sample["occurrence"]
            })
            .unwrap();
        assert_eq!(candidate["occurrence"], sample["occurrence"]);
        assert_eq!(
            candidate_line(candidate),
            1,
            "cap one selects source ordinal zero"
        );
        let excerpt: Value = serde_json::from_str(sample["excerpt"].as_str().unwrap()).unwrap();
        let serialized_excerpt = excerpt.to_string();
        assert!(serialized_excerpt.contains("PUBLIC_SAMPLE_CANARY_0"));
        assert_eq!(
            serialized_excerpt.contains("PRIVATE_SAMPLE_CANARY_0"),
            granted_view
        );
        assert!(!serialized_excerpt.contains("OPAQUE_SAMPLE_CANARY"));
        for other_index in [1, 2] {
            assert!(!serialized_excerpt.contains(&format!("PUBLIC_SAMPLE_CANARY_{other_index}")));
            assert!(!serialized_excerpt.contains(&format!("PRIVATE_SAMPLE_CANARY_{other_index}")));
            assert!(!serialized_excerpt.contains(&format!("OPAQUE_SAMPLE_CANARY_{other_index}")));
        }
        let request = request_views.get(&granted_view).unwrap().get(&0).unwrap();
        assert_eq!(request.contains("PRIVATE_SAMPLE_CANARY_0"), granted_view);
        assert!(!request.contains("OPAQUE_SAMPLE_CANARY"));
    }
    let mut sample_privacy_counts = report["privacy"]["left_out"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|count| count["route"] == "samples")
        .map(|count| {
            (
                count["private_kind"].as_str().unwrap(),
                count["count"].as_u64().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    sample_privacy_counts.sort_unstable();
    assert_eq!(
        sample_privacy_counts,
        vec![("encrypted_reasoning", 1), ("encrypted_reasoning", 2)]
    );
    assert_eq!(
        report["privacy"]["granted"],
        json!([{
            "rule":{"rule_set":root.path().join("set.json").canonicalize().unwrap(),"rule_id":"candidate.granted"},
            "private_kind":"encrypted_reasoning"
        }])
    );
}

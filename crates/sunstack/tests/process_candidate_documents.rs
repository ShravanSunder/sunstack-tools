#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/document_candidate_fixture.rs"]
mod document_fixture;
use document_fixture::*;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use serde_json::{Value, json};
use transport::{AnswerServer, ReplyMode, RouterReply};

#[tokio::test]
async fn document_target_selects_each_paragraph_once_with_original_crlf_and_unicode_spans() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let (repository, input, text, revision) = fixture(
        &server.endpoint,
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
        None,
    );
    let output = support::run(repository.root.path(), "evaluate_candidates", input.clone()).await;
    let wire = candidate_response(&output, 0);
    let report = &wire["result"]["outcome"]["report"];
    let entries = report["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), 3, "{wire}");
    let repeated = support::run(repository.root.path(), "evaluate_candidates", input).await;
    let repeated = candidate_response(&repeated, 0);
    let identities = |report: &Value| {
        report["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|candidate| {
                candidate["candidate_id"]["occurrence"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(
        identities(report),
        identities(&repeated["result"]["outcome"]["report"])
    );
    let (count, bodies) = server.finish().await;
    assert_eq!(count, 6);
    let expected = [
        (
            "If café is ready, load alpha.\r\n",
            "If café is ready, load alpha.",
        ),
        ("Ordinary words remain.\r\n", "Ordinary words remain."),
        ("When ready, load beta.\r\n", "When ready, load beta."),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for candidate in entries {
        assert_eq!(candidate["occurrence"]["kind"], "document_unit");
        assert_eq!(candidate["outcome"]["kind"], "finding");
        let span = &candidate["occurrence"]["occurrence"]["span"];
        let start = span["byte_start"].as_u64().unwrap() as usize;
        let (raw, _) = expected
            .iter()
            .find(|(raw, _)| text.find(raw) == Some(start))
            .unwrap();
        assert!(seen.insert(start));
        assert_document_excerpt(span, &text, raw, &revision);
        assert_eq!(candidate["answers"].as_array().unwrap().len(), 1);
    }
    assert_eq!(seen.len(), 3);
    let mut received = std::collections::BTreeMap::new();
    for body in bodies {
        let units = request_markdown_units(&body);
        assert_eq!(units.len(), 1);
        let (raw, visible) = expected
            .iter()
            .find(|(raw, _)| units[0]["text"] == *raw)
            .unwrap();
        assert_eq!(units[0]["visible_text"], *visible);
        *received.entry(*raw).or_insert(0usize) += 1;
        let start = text.find(raw).unwrap();
        for attribution in request_markdown_attributions(&body) {
            assert_document_span(
                &attribution["span"],
                &text,
                start,
                start + raw.len(),
                &revision,
            );
        }
    }
    assert_eq!(
        received,
        expected.iter().map(|(raw, _)| (*raw, 2usize)).collect()
    );
    assert_eq!(report["inputs"][0]["discovery"][0]["status"], "completed");
}
#[tokio::test]
async fn document_mechanical_pass_filters_selected_visible_units_and_samples_only_rejections() {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let (repository, input, text, revision) = fixture(
        &server.endpoint,
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
        Some(json!({"predicate":{"kind":"regex","field":"/text","pattern":"^If"}})),
    );
    let output = support::run(repository.root.path(), "evaluate_candidates", input).await;
    let wire = candidate_response(&output, 0);
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(report["candidates"].as_array().unwrap().len(), 1, "{wire}");
    assert_eq!(report["dropped_samples"].as_array().unwrap().len(), 2);
    let (count, bodies) = server.finish().await;
    assert_eq!(count, 1);
    let kept = &report["candidates"][0]["occurrence"]["occurrence"]["span"];
    assert_document_excerpt(kept, &text, "If café is ready, load alpha.\r\n", &revision);
    let request_units = request_markdown_units(&bodies[0]);
    assert_eq!(request_units.len(), 1);
    assert_eq!(
        request_units[0]["text"],
        "If café is ready, load alpha.\r\n"
    );
    assert_eq!(
        request_units[0]["visible_text"],
        "If café is ready, load alpha."
    );
    let mut sampled = std::collections::BTreeSet::new();
    for sample in report["dropped_samples"].as_array().unwrap() {
        assert_eq!(sample["pass"], "mechanical");
        let expected = match sample["excerpt"].as_str().unwrap() {
            "Ordinary words remain." => "Ordinary words remain.\r\n",
            "When ready, load beta." => "When ready, load beta.\r\n",
            other => panic!("wrong sample {other}"),
        };
        assert!(sampled.insert(expected));
        assert_document_excerpt(
            &sample["occurrence"]["occurrence"]["span"],
            &text,
            expected,
            &revision,
        );
    }
    assert_eq!(
        sampled,
        std::collections::BTreeSet::from([
            "Ordinary words remain.\r\n",
            "When ready, load beta.\r\n"
        ])
    );
}
#[tokio::test]
async fn document_target_refuses_ast_and_nontext_mechanical_filters_before_requests() {
    for predicate in [
        json!({"kind":"ast","language":"rust","pattern":"flagged($A)"}),
        json!({"kind":"literal","field":"/body","text":"If"}),
    ] {
        let (repository, input, _, _) = fixture(
            "http://127.0.0.1:1/v1/systemone",
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            Some(json!({"predicate":predicate})),
        );
        let output =
            support::run_without_credential(repository.root.path(), "evaluate_candidates", input)
                .await;
        let wire = candidate_response(&output, 2);
        assert_eq!(wire["result"]["outcome"]["kind"], "invalid");
        assert!(
            wire["result"]["outcome"]["diagnostics"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["location"]["field_path"]
                    .as_array()
                    .is_some_and(|path| path.contains(&json!("mechanical_pass"))))
        );
    }
}

#[tokio::test]
async fn local_document_expectations_report_conformance_answers_and_authored_error_severity() {
    for (pattern, expected, settlement) in [
        ("^Use", "dropped", "passed"),
        ("^Never", "finding", "did_not_pass"),
    ] {
        let (repository, input, text, revision) = fixture(
            "http://127.0.0.1:1/v1/systemone",
            json!({"kind":"frontmatter_field","field":"description"}),
            None,
        );
        let path = repository.root.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        rule["severity"] = json!("error");
        rule["verdict"] = json!({"kind":"mechanical","explanation":"Description conforms","check":{"kind":"expect","expect":{"matches":pattern}}});
        std::fs::write(path, rule.to_string()).unwrap();
        let output =
            support::run_without_credential(repository.root.path(), "evaluate_candidates", input)
                .await;
        let wire = candidate_response(&output, 0);
        let report = &wire["result"]["outcome"]["report"];
        let entries = report["candidates"].as_array().unwrap();
        assert_eq!(entries.len(), 1, "{wire}");
        let entry = &entries[0];
        assert_eq!(entry["outcome"]["kind"], expected, "{wire}");
        assert_eq!(entry["declared_severity"], "error");
        assert_eq!(entry["occurrence"]["kind"], "document_field");
        assert_eq!(entry["occurrence"]["field"], "description");
        assert_document_excerpt(
            &entry["occurrence"]["state"]["span"],
            &text,
            "Use when checking.",
            &revision,
        );
        assert_eq!(entry["answers"].as_array().unwrap().len(), 1);
        let answer = &entry["answers"][0];
        assert_eq!(answer["question"], json!({"kind":"verdict"}));
        assert_eq!(answer["answer"], Value::Null);
        assert_eq!(answer["settlement"], settlement);
        if expected == "finding" {
            assert_eq!(entry["outcome"]["path"], json!([answer["answer_id"]]));
        } else {
            assert_eq!(entry["outcome"]["at"], json!({"kind":"verdict"}));
            assert_eq!(entry["outcome"]["answer"], answer["answer_id"]);
        }
        assert_eq!(report["usage"]["requests"], 0);
    }
}

#[tokio::test]
async fn absent_fields_reach_judgment_but_missing_and_malformed_blocks_settle_unresolved() {
    for (text, state, reason, requests) in [
        ("---\nname: sample\n---\n# Body\n", "absent", None, 1),
        (
            "# No frontmatter\n",
            "block_unavailable",
            Some("missing"),
            0,
        ),
        (
            "---\ndescription: [unterminated\n---\n# Body\n",
            "block_unavailable",
            Some("malformed"),
            0,
        ),
    ] {
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.9,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let (repository, mut input, _, _) = fixture(
            &server.endpoint,
            json!({"kind":"frontmatter_field","field":"description"}),
            None,
        );
        replace_document(&repository, &mut input, text);
        let output = support::run(repository.root.path(), "evaluate_candidates", input).await;
        let wire = candidate_response(&output, 0);
        let report = &wire["result"]["outcome"]["report"];
        let entries = report["candidates"].as_array().unwrap();
        assert_eq!(entries.len(), 1, "{wire}");
        let candidate = &entries[0];
        assert_eq!(candidate["occurrence"]["kind"], "document_field");
        assert_eq!(candidate["occurrence"]["state"]["kind"], state);
        assert_eq!(candidate["occurrence"]["field"], "description");
        if let Some(reason) = reason {
            assert_eq!(candidate["occurrence"]["state"]["reason"], reason);
            assert_eq!(candidate["outcome"]["kind"], "unresolved", "{wire}");
            assert_eq!(
                candidate["outcome"]["missing"]["diagnostic"],
                "frontmatter_unavailable"
            );
            assert!(
                candidate["answers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|answer| answer["answer"].is_null())
            );
        } else {
            assert_eq!(candidate["outcome"]["kind"], "finding");
        }
        let (count, bodies) = server.finish().await;
        assert_eq!(count, requests);
        assert_eq!(report["usage"]["requests"], requests);
        if requests == 1 {
            assert!(bodies[0].to_string().contains("absent"));
        }
    }
}

#[tokio::test]
async fn judged_verdict_meaning_maps_both_polarities_and_keeps_uncertain_native_answers() {
    for (meaning, probability, expected) in [
        ("conforms", 0.9, "dropped"),
        ("conforms", 0.1, "finding"),
        ("conforms", 0.5, "uncertain"),
        ("finding", 0.9, "finding"),
        ("finding", 0.1, "dropped"),
        ("finding", 0.5, "uncertain"),
    ] {
        let server = AnswerServer::start(ReplyMode::Answers {
            probability,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let (repository, input, _, _) = fixture(
            &server.endpoint,
            json!({"kind":"frontmatter_field","field":"description"}),
            None,
        );
        let path = repository.root.path().join("rules.jsonl");
        let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        rule["verdict"]["pass_means"] = json!(meaning);
        rule["verdict"]["uncertain_below"] = json!(0.8);
        std::fs::write(path, rule.to_string()).unwrap();
        let output = support::run(repository.root.path(), "evaluate_candidates", input).await;
        let wire = candidate_response(&output, 0);
        let entry = &wire["result"]["outcome"]["report"]["candidates"][0];
        assert_eq!(entry["outcome"]["kind"], expected, "{wire}");
        let answer = &entry["answers"][0];
        assert_eq!(answer["answer"]["yes_probability"], probability);
        if expected == "finding" {
            assert_eq!(entry["outcome"]["path"], json!([answer["answer_id"]]));
        } else if expected == "dropped" {
            assert_eq!(entry["outcome"]["answer"], answer["answer_id"]);
        } else {
            assert_eq!(answer["settlement"], "uncertain");
        }
        assert_eq!(server.finish().await.0, 1);
    }
}

#[tokio::test]
async fn marked_models_keep_filter_narrowing_two_gates_and_sentence_verdict_per_selected_unit() {
    for narrowed in [true, false] {
        let mut server = transport::controlled::ControlledServer::start().await;
        let (repository, input, fixture_text, _) = fixture(
            &server.endpoint,
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            None,
        );
        let root = repository.root.path();
        let mut set: Value =
            serde_json::from_slice(&std::fs::read(root.join("set.json")).unwrap()).unwrap();
        set.as_object_mut().unwrap().remove("rule_paths");
        set["marker_prefix"] = json!("K");
        set["rule_sources"] = json!([{"markdown":"standard.md","rules":"standard.rules.json"}]);
        std::fs::write(root.join("set.json"), set.to_string()).unwrap();
        let evidence = json!([{"id":"unit","input":"subject","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}}]);
        let mut entry = json!({"name":"candidate.one","target":{"input":"subject","select":{"kind":"markdown_units","mode":"unit","units":["paragraph"]}},"mechanical_pass":{"predicate":{"kind":"regex","field":"/text","pattern":"If|Ordinary|When"}},"gates":[{"kind":"decision","question":{"type":"noul","instructions":"GATE_FIRST"},"evidence":evidence,"threshold":0.8},{"kind":"decision","question":{"type":"noul","instructions":"GATE_SECOND"},"evidence":evidence,"threshold":0.8}],"questions":{"extra":{"type":"noul","instructions":"INFORMATION_ONLY"}},"threshold":0.8,"severity":"error"});
        if narrowed {
            entry["narrowing"] = json!({"question":{"type":"noul","instructions":"NARROW_SUBJECT"},"evidence":evidence,"threshold":0.8});
        }
        std::fs::write(
            root.join("standard.md"),
            "The selected unit meets the standard.<!-- K014 -->\n",
        )
        .unwrap();
        std::fs::write(
            root.join("standard.rules.json"),
            json!({"K014":entry}).to_string(),
        )
        .unwrap();
        let checked = json!({"definition_root":root,"definitions":{"kind":"rule_sets","rule_sets":["set.json"]},"payload_schemas":[],"limits":{"max_file_bytes":65536},"output":"assembled_rules"});
        let printed = support::run_without_credential(root, "check_rules", checked).await;
        let printed = support::response(&printed, 0);
        let definition = &printed["result"]["outcome"]["printed_rules"][0]["definition"];
        assert_eq!(definition["applicability"].as_array().unwrap().len(), 2);
        assert_eq!(definition["verdict"]["pass_means"], "conforms");
        assert_eq!(
            definition["verdict"]["question"]["instructions"],
            "The selected unit meets the standard."
        );
        assert_eq!(
            definition["mechanical_pass"]["predicate"]["pattern"],
            "If|Ordinary|When"
        );
        assert_eq!(
            definition["questions"]["extra"]["instructions"],
            "INFORMATION_ONLY"
        );
        assert_eq!(!definition["narrowing"].is_null(), narrowed);
        let directory = root.to_owned();
        let expected = std::collections::BTreeMap::from([
            (
                fixture_text.find("If café").unwrap(),
                ("finding", "verdict"),
            ),
            (
                fixture_text.find("Ordinary words").unwrap(),
                if narrowed {
                    ("dropped", "narrowing")
                } else {
                    ("finding", "verdict")
                },
            ),
            (
                fixture_text.find("When ready").unwrap(),
                ("dropped", "first"),
            ),
        ]);
        let mut running =
            tokio::spawn(
                async move { support::run(&directory, "evaluate_candidates", input).await },
            );
        let mut observations = Vec::new();
        let mut requests = std::collections::BTreeMap::new();
        let output = loop {
            let request = tokio::select! {output=&mut running=>break output.unwrap(),request=server.next()=>request};
            let body = request.body.to_string();
            let subject = if body.contains("If café") {
                "if"
            } else if body.contains("Ordinary words") {
                "ordinary"
            } else if body.contains("When ready") {
                "when"
            } else {
                panic!("missing selected subject: {body}")
            };
            *requests.entry(subject).or_insert(0usize) += 1;
            assert_eq!(
                ["If café", "Ordinary words", "When ready"]
                    .iter()
                    .filter(|text| body.contains(**text))
                    .count(),
                1
            );
            let mut response =
                transport::response_body(&request.body, 0.9, RouterReply::ReviewHandback);
            for (name, question) in request.body["questions"].as_object().unwrap() {
                let question = question.to_string();
                let stage = if question.contains("NARROW_SUBJECT") {
                    "narrowing"
                } else if question.contains("GATE_FIRST") {
                    "first"
                } else if question.contains("GATE_SECOND") {
                    "second"
                } else if question.contains("INFORMATION_ONLY") {
                    "info"
                } else if question.contains("The selected unit meets the standard.") {
                    "verdict"
                } else {
                    panic!("unexpected question {question}")
                };
                let probability = if (subject == "ordinary" && stage == "narrowing")
                    || (subject == "when" && stage == "first")
                {
                    0.1
                } else if stage == "verdict" {
                    0.3
                } else {
                    0.9
                };
                response["answers"][name]["noul"] = json!(probability);
                observations.push((subject, stage));
            }
            request.respond(200, response);
        };
        let wire = candidate_response(&output, 0);
        let entries = wire["result"]["outcome"]["report"]["candidates"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), 3);
        let mut seen = std::collections::BTreeSet::new();
        for candidate in entries {
            let start = candidate["occurrence"]["occurrence"]["span"]["byte_start"]
                .as_u64()
                .unwrap() as usize;
            assert!(seen.insert(start));
            let (expected_kind, expected_stage) = expected.get(&start).unwrap();
            let answers = candidate["answers"].as_array().unwrap();
            assert_eq!(
                answers
                    .iter()
                    .map(|answer| answer["answer_id"].as_str().unwrap())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                answers.len()
            );
            let outcome = &candidate["outcome"];
            assert_eq!(
                outcome["kind"], *expected_kind,
                "fixture unit at {start}: {wire}"
            );
            if *expected_stage == "narrowing" {
                assert_eq!(outcome["at"], json!({"kind":"narrowing"}));
                assert_eq!(answers.len(), 1);
                assert_eq!(answers[0]["question"], json!({"kind":"narrowing"}));
                assert_eq!(answers[0]["settlement"], "did_not_pass");
                assert_eq!(outcome["answer"], answers[0]["answer_id"]);
            } else if *expected_stage == "verdict" {
                assert_eq!(candidate["declared_severity"], "error");
                let verdict = answers
                    .iter()
                    .find(|answer| answer["question"]["kind"] == "verdict")
                    .unwrap();
                assert_eq!(verdict["settlement"], "did_not_pass");
                let mut expected_questions = Vec::new();
                let mut probabilities = Vec::new();
                if narrowed {
                    expected_questions.push(json!({"kind":"narrowing"}));
                    probabilities.push(0.9);
                }
                expected_questions.extend([
                    json!({"kind":"rule_gate","position":0}),
                    json!({"kind":"rule_gate","position":1}),
                    json!({"kind":"verdict"}),
                    json!({"kind":"informational","name":"extra"}),
                ]);
                probabilities.extend([0.9, 0.9, 0.3, 0.9]);
                assert_eq!(
                    answers
                        .iter()
                        .map(|answer| answer["question"].clone())
                        .collect::<Vec<_>>(),
                    expected_questions
                );
                for (answer, probability) in answers.iter().zip(probabilities) {
                    assert_eq!(answer["answer"]["yes_probability"], probability);
                }
                let expected_path = answers[..answers.len() - 1]
                    .iter()
                    .map(|answer| answer["answer_id"].clone())
                    .collect::<Vec<_>>();
                assert_eq!(outcome["path"], json!(expected_path));
                assert_eq!(expected_path.len(), if narrowed { 4 } else { 3 });
            } else {
                assert_eq!(outcome["at"], json!({"kind":"rule_gate","position":0}));
                assert_eq!(answers.len(), if narrowed { 2 } else { 1 });
                assert!(answers.iter().all(|answer| !matches!(
                    answer["question"]["kind"].as_str(),
                    Some("verdict" | "informational")
                )));
                let rejected = answers
                    .iter()
                    .find(|answer| answer["question"] == json!({"kind":"rule_gate","position":0}))
                    .unwrap();
                assert_eq!(rejected["settlement"], "did_not_pass");
                assert_eq!(outcome["answer"], rejected["answer_id"]);
            }
        }
        assert_eq!(seen, expected.keys().copied().collect());
        assert_eq!(
            requests,
            std::collections::BTreeMap::from([("if", 1), ("ordinary", 1), ("when", 1)])
        );
        let stages = if narrowed {
            vec!["narrowing", "first", "second", "verdict", "info"]
        } else {
            vec!["first", "second", "verdict", "info"]
        };
        for subject in ["if", "ordinary", "when"] {
            assert_eq!(
                observations
                    .iter()
                    .filter(|(owner, _)| *owner == subject)
                    .map(|(_, stage)| *stage)
                    .collect::<Vec<_>>(),
                stages,
                "{subject}"
            );
        }
        assert!(observations.contains(&("if", "verdict")));
        assert!(observations.contains(&("when", "verdict")));
        if narrowed {
            assert!(observations.contains(&("ordinary", "narrowing")));
            assert!(observations.contains(&("ordinary", "first")));
            assert!(observations.contains(&("ordinary", "verdict")));
        } else {
            assert!(observations.contains(&("ordinary", "verdict")));
            assert!(!observations.iter().any(|(_, stage)| *stage == "narrowing"));
        }
        server.finish().await;
    }
}

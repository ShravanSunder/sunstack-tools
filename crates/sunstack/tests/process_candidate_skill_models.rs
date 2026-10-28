#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "support/document_candidate_fixture.rs"]
mod document_fixture;
mod support;
#[path = "support/rule_transport.rs"]
mod transport;
use document_fixture::*;
use serde_json::{Value, json};
use transport::{AnswerServer, ReplyMode, RouterReply};
fn standards() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sunstack-rules/tests/fixtures/skill-standards")
}
#[tokio::test]
async fn three_real_skill_rulesets_check_without_a_reviewed_target_binding_or_key() {
    for (name, count) in [
        ("skill-metadata", 2),
        ("skill-text", 5),
        ("skill-references", 1),
    ] {
        let root = standards();
        let input = json!({"definition_root":root,"definitions":{"kind":"rule_sets","rule_sets":[format!("rules/{name}.ruleset.json")]},"payload_schemas":[],"limits":{"max_file_bytes":1048576},"output":"assembled_rules"});
        let output = support::run_without_credential(&root, "check_rules", input).await;
        let wire = support::response(&output, 0);
        assert_eq!(wire["result"]["outcome"]["checked_rule_count"], count);
        for rule in wire["result"]["outcome"]["printed_rules"]
            .as_array()
            .unwrap()
        {
            assert_eq!(rule["definition"]["verdict"]["pass_means"], "conforms");
            assert_eq!(rule["source"]["kind"], "markdown");
        }
    }
}
#[tokio::test]
async fn unchanged_real_section_and_call_site_entries_select_exact_frozen_units() {
    let text = "---\r\ndescription: Use when checking.\r\n---\r\n# First\r\n\r\nIf café is ready, load alpha.\r\n\r\n- When ready, load one. If more, load two.\r\n\r\n# Second\r\n\r\nOrdinary words remain.\r\n\r\n# Third\r\n\r\nWhen ready, load beta.\r\n";
    for (rules, markdown, prefix, rule_id, count) in [
        (
            "main-path.rules.json",
            "main-path.md",
            "K",
            "branch.condition",
            4,
        ),
        (
            "calls-and-placement.references.rules.json",
            "calls-and-placement.md",
            "R",
            "reference.teaching-depth",
            3,
        ),
    ] {
        let server = AnswerServer::start(ReplyMode::Answers {
            probability: 0.3,
            router: RouterReply::ReviewHandback,
        })
        .await;
        let (repository, mut input, _, _) = fixture(
            &server.endpoint,
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
            None,
        );
        replace_document(&repository, &mut input, text);
        let revision = input["sources"][0]["source"]["revision"]["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        let source = standards();
        let root = repository.root.path();
        std::fs::write(root.join("set.json"),json!({"schema_version":1,"name":"real-entry","description":"Unchanged real skill entry","inputs":{"skill":{"kind":"document"}},"applicability":[],"common_ruleset_instructions":null,"marker_prefix":prefix,"rule_sources":[{"markdown":source.join("references").join(markdown),"rules":source.join("rules").join(rules)}]}).to_string()).unwrap();
        let output = support::run(root, "evaluate_candidates", input).await;
        let wire = candidate_response(&output, 0);
        let entries = wire["result"]["outcome"]["report"]["candidates"]
            .as_array()
            .unwrap();
        assert_eq!(entries.len(), count, "{wire}");
        let mut match_starts = std::collections::BTreeSet::new();
        let mut sections = std::collections::BTreeSet::new();
        let paragraph_raw = "If café is ready, load alpha.\r\n";
        let list_raw = "- When ready, load one. If more, load two.\r\n\r\n";
        let third_raw = "When ready, load beta.\r\n";
        let owners = [
            (paragraph_raw, "If café is ready, load alpha.", "paragraph"),
            (
                list_raw,
                "- When ready, load one. If more, load two.",
                "list_item",
            ),
            (third_raw, "When ready, load beta.", "paragraph"),
        ];
        let calls = std::collections::BTreeMap::from([
            (text.find("If café").unwrap(), (0usize, 0usize, "If")),
            (text.find("When ready, load one").unwrap(), (1, 2, "When")),
            (
                text.find("If more").unwrap(),
                (1, owners[1].1.find("If more").unwrap(), "If"),
            ),
            (text.find("When ready, load beta").unwrap(), (2, 0, "When")),
        ]);
        let section_ranges = [
            (
                text.find("# First").unwrap(),
                text.find("# Second").unwrap(),
                "First\n\nIf café is ready, load alpha.\n\n- When ready, load one. If more, load two.",
            ),
            (
                text.find("# Second").unwrap(),
                text.find("# Third").unwrap(),
                "Second\n\nOrdinary words remain.",
            ),
            (
                text.find("# Third").unwrap(),
                text.len(),
                "Third\n\nWhen ready, load beta.",
            ),
        ];
        for candidate in entries {
            assert_eq!(candidate["rule"]["rule_id"], rule_id);
            assert_eq!(candidate["occurrence"]["kind"], "document_unit");
            assert_eq!(candidate["outcome"]["kind"], "finding");
            assert_eq!(candidate["answers"].as_array().unwrap().len(), 1);
            let occurrence = &candidate["occurrence"]["occurrence"];
            if rule_id == "branch.condition" {
                assert_eq!(occurrence["kind"], "call_site");
                let matched = &occurrence["match_span"];
                let start = matched["byte_start"].as_u64().unwrap() as usize;
                let (owner_index, visible_start, word) = calls.get(&start).unwrap();
                assert_document_span(matched, text, start, start + word.len(), &revision);
                assert!(match_starts.insert(start));
                let (raw, _, kind) = owners[*owner_index];
                assert_eq!(candidate["occurrence"]["unit_kind"], kind);
                assert_document_excerpt(&occurrence["unit_span"], text, raw, &revision);
                assert_eq!(
                    occurrence["visible_range"],
                    json!({"start":visible_start,"end":visible_start+word.len()})
                );
            } else {
                assert_eq!(occurrence["kind"], "unit");
                assert_eq!(candidate["occurrence"]["unit_kind"], "section");
                let span = &occurrence["span"];
                let start = span["byte_start"].as_u64().unwrap() as usize;
                let (_, expected_end, _) = section_ranges
                    .iter()
                    .find(|(expected_start, _, _)| *expected_start == start)
                    .unwrap();
                assert_document_span(span, text, start, *expected_end, &revision);
                assert!(sections.insert(start));
            }
        }
        if rule_id == "branch.condition" {
            let expected = [
                text.find("If café").unwrap(),
                text.find("When ready, load one").unwrap(),
                text.find("If more").unwrap(),
                text.find("When ready, load beta").unwrap(),
            ]
            .into_iter()
            .collect();
            assert_eq!(match_starts, expected);
        } else {
            assert_eq!(sections.len(), 3);
        }
        let (requests, bodies) = server.finish().await;
        assert_eq!(requests, count);
        let mut observed_raw = std::collections::BTreeMap::new();
        for body in bodies {
            let units = request_markdown_units(&body);
            assert_eq!(units.len(), 1);
            let (start, end, visible) = if rule_id == "branch.condition" {
                let (raw, visible, _) = owners
                    .iter()
                    .find(|(raw, _, _)| units[0]["text"] == *raw)
                    .unwrap();
                let start = text.find(raw).unwrap();
                (start, start + raw.len(), *visible)
            } else {
                *section_ranges
                    .iter()
                    .find(|(start, end, _)| units[0]["text"] == text[*start..*end])
                    .unwrap()
            };
            assert_eq!(units[0]["text"], text[start..end]);
            assert_eq!(units[0]["visible_text"], visible);
            *observed_raw.entry(start).or_insert(0usize) += 1;
            for attribution in request_markdown_attributions(&body) {
                assert_document_span(&attribution["span"], text, start, end, &revision);
            }
        }
        let expected_counts = if rule_id == "branch.condition" {
            std::collections::BTreeMap::from([
                (text.find(owners[0].0).unwrap(), 1),
                (text.find(owners[1].0).unwrap(), 2),
                (text.find(owners[2].0).unwrap(), 1),
            ])
        } else {
            section_ranges
                .iter()
                .map(|(start, _, _)| (*start, 1))
                .collect()
        };
        assert_eq!(observed_raw, expected_counts);
    }
}

#[tokio::test]
async fn local_description_length_boundary_counts_three_hundred_unicode_characters() {
    for (length, kind, settlement) in [(300, "dropped", "passed"), (301, "finding", "did_not_pass")]
    {
        let (repository, mut input, _, _) = fixture(
            "http://127.0.0.1:1/v1/systemone",
            json!({"kind":"frontmatter_field","field":"description"}),
            None,
        );
        let value = "é".repeat(length);
        let text = format!("---\r\ndescription: {value}\r\n---\r\n# Body\r\n");
        replace_document(&repository, &mut input, &text);
        let revision = input["sources"][0]["source"]["revision"]["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        let root = repository.root.path();
        let mut rule: Value =
            serde_json::from_slice(&std::fs::read(root.join("rules.jsonl")).unwrap()).unwrap();
        rule["severity"] = json!("error");
        rule["verdict"] = json!({"kind":"mechanical","explanation":"Description is within 300 characters","check":{"kind":"expect","expect":{"matches":"^[\\s\\S]{0,300}$"}}});
        std::fs::write(root.join("rules.jsonl"), rule.to_string()).unwrap();
        let output = support::run_without_credential(root, "evaluate_candidates", input).await;
        let wire = candidate_response(&output, 0);
        let report = &wire["result"]["outcome"]["report"];
        assert_eq!(report["candidates"].as_array().unwrap().len(), 1);
        let candidate = &report["candidates"][0];
        assert_eq!(candidate["outcome"]["kind"], kind, "{wire}");
        assert_eq!(candidate["declared_severity"], "error");
        assert_eq!(candidate["answers"][0]["settlement"], settlement);
        assert!(candidate["answers"][0]["answer"].is_null());
        assert_document_excerpt(
            &candidate["occurrence"]["state"]["span"],
            &text,
            &value,
            &revision,
        );
        assert_eq!(report["usage"]["requests"], 0);
    }
}
#[tokio::test]
async fn equal_valued_frontmatter_fields_and_two_rules_on_one_unit_keep_distinct_identities() {
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
    replace_document(
        &repository,
        &mut input,
        "---\ndescription: same\nname: same\n---\n# Body\n\nSingle subject.\n",
    );
    let path = repository.root.path().join("rules.jsonl");
    let original: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let mut definitions = String::new();
    for (name, select) in [
        (
            "description",
            json!({"kind":"frontmatter_field","field":"description"}),
        ),
        ("name", json!({"kind":"frontmatter_field","field":"name"})),
        (
            "unit.one",
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
        ),
        (
            "unit.two",
            json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
        ),
    ] {
        let mut rule = original.clone();
        rule["rule_id"] = json!(name);
        rule["evidence"][0]["select"] = select;
        definitions.push_str(&rule.to_string());
        definitions.push('\n');
    }
    std::fs::write(path, definitions).unwrap();
    let output = support::run(repository.root.path(), "evaluate_candidates", input).await;
    let wire = candidate_response(&output, 0);
    let entries = wire["result"]["outcome"]["report"]["candidates"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 4, "{wire}");
    let by_rule = entries
        .iter()
        .map(|candidate| (candidate["rule"]["rule_id"].as_str().unwrap(), candidate))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_ne!(
        by_rule["name"]["candidate_id"]["occurrence"],
        by_rule["description"]["candidate_id"]["occurrence"]
    );
    assert_eq!(
        by_rule["unit.one"]["candidate_id"]["occurrence"],
        by_rule["unit.two"]["candidate_id"]["occurrence"]
    );
    assert_ne!(
        by_rule["unit.one"]["candidate_id"],
        by_rule["unit.two"]["candidate_id"]
    );
    assert_eq!(server.finish().await.0, 4);
}

#[tokio::test]
async fn quoted_and_block_fields_send_decoded_values_and_keep_the_native_field_anchor() {
    for (yaml, expected) in [
        ("\"Use when quoted.\"", "Use when quoted."),
        (
            "|-\n  Use when block.\n  Next line.",
            "Use when block.\nNext line.",
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
        let text = format!("---\ndescription: {yaml}\n---\n# Body\n");
        replace_document(&repository, &mut input, &text);
        let output = support::run(repository.root.path(), "evaluate_candidates", input).await;
        let wire = candidate_response(&output, 0);
        let entry = &wire["result"]["outcome"]["report"]["candidates"][0];
        assert_eq!(entry["outcome"]["kind"], "finding");
        assert_eq!(entry["occurrence"]["kind"], "document_field");
        assert_eq!(entry["occurrence"]["field"], "description");
        let (count, bodies) = server.finish().await;
        assert_eq!(count, 1);
        let evidence = bodies[0]["state"]["evidence"].as_object().unwrap();
        let fields = evidence
            .values()
            .filter(|value| value["kind"] == "frontmatter_field")
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0]["result"]["value"], expected);
        assert_eq!(
            fields[0]["result"]["span"],
            entry["occurrence"]["state"]["span"]
        );
        let span = &entry["occurrence"]["state"]["span"];
        let raw = if yaml.starts_with('|') {
            "Use when block.\n  Next line.\n".to_owned()
        } else {
            yaml.to_owned()
        };
        let revision = entry["occurrence"]["source"]["revision"]["commit_id"]
            .as_str()
            .unwrap();
        assert_document_excerpt(span, &text, &raw, revision);
    }
}

#[tokio::test]
async fn local_ast_verdict_uses_original_frozen_document_with_ancestors_and_never_live_worktree_text()
 {
    let (repository, mut input, _, _) = fixture(
        "http://127.0.0.1:1/v1/systemone",
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
        None,
    );
    replace_document(
        &repository,
        &mut input,
        "# First\n\n- BAD nested.\n\nOutside paragraph.\n",
    );
    let frozen_text = "# First\n\n- BAD nested.\n\nOutside paragraph.\n";
    let revision = input["sources"][0]["source"]["revision"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let root = repository.root.path();
    let path = root.join("rules.jsonl");
    let mut rule: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    rule["verdict"] = json!({"kind":"mechanical","explanation":"No bad nested paragraph","check":{"kind":"ast_grep","ast_grep":{"kind":"paragraph","regex":"BAD","inside":{"kind":"list_item"}}}});
    std::fs::write(path, rule.to_string()).unwrap();
    repository.write("guide.md", "# Dirty\n\nNothing bad or nested is live.\n");
    let output = support::run_without_credential(root, "evaluate_candidates", input).await;
    let wire = candidate_response(&output, 0);
    let report = &wire["result"]["outcome"]["report"];
    let candidates = report["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2, "{wire}");
    let expected = std::collections::BTreeMap::from([
        (
            frozen_text.find("BAD nested.").unwrap(),
            ("BAD nested.\n", "finding", "did_not_pass"),
        ),
        (
            frozen_text.find("Outside paragraph.").unwrap(),
            ("Outside paragraph.\n", "dropped", "passed"),
        ),
    ]);
    let mut seen = std::collections::BTreeSet::new();
    for candidate in candidates {
        let span = &candidate["occurrence"]["occurrence"]["span"];
        let start = span["byte_start"].as_u64().unwrap() as usize;
        let (raw, outcome, settlement) = expected.get(&start).unwrap();
        assert!(seen.insert(start));
        assert_document_excerpt(span, frozen_text, raw, &revision);
        assert_eq!(candidate["outcome"]["kind"], *outcome, "{wire}");
        assert_eq!(candidate["answers"][0]["settlement"], *settlement);
    }
    assert_eq!(seen, expected.keys().copied().collect());
    assert_eq!(report["usage"]["requests"], 0);
}

#[tokio::test]
async fn reachable_document_judgments_without_a_key_are_refused_without_dispatch_or_fabricated_answers()
 {
    let server = AnswerServer::start(ReplyMode::Answers {
        probability: 0.9,
        router: RouterReply::ReviewHandback,
    })
    .await;
    let (repository, input, _, _) = fixture(
        &server.endpoint,
        json!({"kind":"markdown_units","mode":"unit","units":["paragraph"]}),
        None,
    );
    let output =
        support::run_without_credential(repository.root.path(), "evaluate_candidates", input).await;
    let wire = candidate_response(&output, 0);
    let report = &wire["result"]["outcome"]["report"];
    assert_eq!(
        report["completeness"],
        json!({"kind":"inconclusive","uncertain":0,"unresolved":0,"not_processed":3})
    );
    assert_eq!(report["candidates"].as_array().unwrap().len(), 3);
    for candidate in report["candidates"].as_array().unwrap() {
        assert_eq!(
            candidate["outcome"],
            json!({"kind":"not_processed","reason":"refused"})
        );
        assert_eq!(candidate["answers"], json!([]));
    }
    assert_eq!(report["usage"]["requests"], 0);
    assert_eq!(server.finish().await.0, 0);
}

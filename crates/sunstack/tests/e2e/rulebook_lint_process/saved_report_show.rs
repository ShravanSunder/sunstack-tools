use super::*;

#[path = "saved_report_show/collisions.rs"]
mod collisions;

// Minimal projection of the frozen R03 0.11.0 report: actual case/result shapes,
// two revisions sharing result 27, and result 68 occurring only in one revision.
const COMPLETED_CASES: &str = r#"{
  "errors": {},
  "not_selected": [],
  "by_rule": {},
  "by_filter": {},
  "cases": [
    {
      "id": "r03-current-typescript-topics",
      "case": {
        "id": "r03-current-typescript-topics",
        "revision": "89dc638e3fe6a865cb7c4a00af9c3c4440549259",
        "set": "development",
        "label_from": "blind-agents",
        "kind": "rule",
        "rule": "skill-description.states-triggers-not-workflow-steps",
        "file": "plugins/sunstack-dev/skills/domain-typescript/SKILL.md",
        "lines": [
          1,
          5
        ],
        "text_sha256": "7ede34407c3d3c57c658a214fe4e69008da66569091c32496a9fc32d1039e723",
        "expected": "pass"
      },
      "outcome": {
        "kind": "right"
      },
      "result_id": 27,
      "bars": [
        {
          "step": "carries_extra",
          "bound": "yes_at_or_above",
          "bar": 0.7,
          "observed": 0.06,
          "crossed": false
        },
        {
          "step": "carries_extra",
          "bound": "no_at_or_below",
          "bar": 0.3,
          "observed": 0.06,
          "crossed": true
        }
      ]
    }
  ],
  "counts": {
    "right": 1
  },
  "bars": [],
  "runs": [
    {
      "revision": "89dc638e3fe6a865cb7c4a00af9c3c4440549259",
      "resolved_revision": "89dc638e3fe6a865cb7c4a00af9c3c4440549259",
      "report": {
        "piece_evidence": {},
        "requests": [],
        "rulebook_files": [],
        "filter_answers": [],
        "filter_skips": [],
        "version": "0.9.0",
        "config": "/fixture/r03/config.json",
        "scope": {
          "kind": "all",
          "widened": null
        },
        "status": {
          "kind": "clean"
        },
        "findings": [],
        "results": [
          {
            "text_sha256": "7ede34407c3d3c57c658a214fe4e69008da66569091c32496a9fc32d1039e723",
            "id": 27,
            "request_ids": [],
            "name": "skill-description.states-triggers-not-workflow-steps",
            "marker": "K005",
            "file": "plugins/sunstack-dev/skills/domain-typescript/SKILL.md",
            "first_line": 1,
            "last_line": 5,
            "outcome": "pass",
            "step": "carries_extra",
            "path": [
              "complete_description",
              "carries_extra"
            ],
            "basis": "judged",
            "answers": {
              "carries_extra": {
                "type": "noul",
                "yes_probability": 0.06
              }
            }
          },
          {
            "text_sha256": "480e4eb675f1a7348c6e5da9126158b07ecb257455a603842b85d1abf31b75ac",
            "id": 68,
            "request_ids": [],
            "name": "skill-description.longer-than-300-characters",
            "marker": "K003",
            "file": "plugins/sunstack-dev/skills/role-lead/SKILL.md",
            "first_line": 1,
            "last_line": 4,
            "outcome": "pass",
            "step": "within_300",
            "path": [
              "within_300"
            ],
            "basis": "mechanical"
          }
        ],
        "uncertain": [],
        "selections": [],
        "file_counts": [],
        "skipped_rules": [],
        "counts": {
          "error": 0,
          "warn": 1
        },
        "exclusions": {
          "off": 0,
          "ignored": 0,
          "not_applicable": 0,
          "outside_change_scope": 0,
          "zero_matches": 0
        },
        "incomplete": [],
        "usage": {
          "checked_files_acquired": 0,
          "model_requests": 0,
          "input_tokens": null,
          "output_tokens": null,
          "cost_usd": null,
          "cost_reported_requests": 0,
          "cost_missing_requests": 0
        },
        "evidence_seen": {},
        "limits": {
          "max_locate_lines": 80,
          "evidence_max_chars": 6000,
          "request_max_chars": 131072,
          "max_files": 2000,
          "max_file_bytes": 1048576,
          "max_total_bytes": 67108864,
          "max_matches": 20000,
          "max_model_requests": 9000,
          "max_input_tokens_per_request": 131072,
          "attempt_timeout_seconds": 60,
          "max_concurrent_requests": 8,
          "max_transcript_bytes": 8388608,
          "max_request_bytes": 262144,
          "max_questions_per_request": 32,
          "retry_count": 2,
          "initial_backoff_ms": 500,
          "max_backoff_ms": 8000,
          "max_response_bytes": 1048576
        },
        "diagnostics": []
      }
    },
    {
      "revision": "9f7b2e283e95dc849f4e271a5287e3ae21959ca9",
      "resolved_revision": "9f7b2e283e95dc849f4e271a5287e3ae21959ca9",
      "report": {
        "piece_evidence": {},
        "requests": [],
        "rulebook_files": [],
        "filter_answers": [],
        "filter_skips": [],
        "version": "0.9.0",
        "config": "/fixture/r03/config.json",
        "scope": {
          "kind": "all",
          "widened": null
        },
        "status": {
          "kind": "findings"
        },
        "findings": [],
        "results": [
          {
            "text_sha256": "5e90500a28b197597b49cd4bb3b54508b6c8915b0acea7358cb6e5139a953cde",
            "id": 27,
            "request_ids": [],
            "name": "skill-description.longer-than-225-characters",
            "marker": "K002",
            "file": "plugins/sunstack-dev/skills/ops-observability-stack/SKILL.md",
            "first_line": 1,
            "last_line": 4,
            "outcome": "pass",
            "step": "within_225",
            "path": [
              "within_225"
            ],
            "basis": "mechanical"
          }
        ],
        "uncertain": [],
        "selections": [],
        "file_counts": [],
        "skipped_rules": [],
        "counts": {
          "error": 4,
          "warn": 5
        },
        "exclusions": {
          "off": 0,
          "ignored": 0,
          "not_applicable": 0,
          "outside_change_scope": 0,
          "zero_matches": 0
        },
        "incomplete": [],
        "usage": {
          "checked_files_acquired": 0,
          "model_requests": 0,
          "input_tokens": null,
          "output_tokens": null,
          "cost_usd": null,
          "cost_reported_requests": 0,
          "cost_missing_requests": 0
        },
        "evidence_seen": {},
        "limits": {
          "max_locate_lines": 80,
          "evidence_max_chars": 6000,
          "request_max_chars": 131072,
          "max_files": 2000,
          "max_file_bytes": 1048576,
          "max_total_bytes": 67108864,
          "max_matches": 20000,
          "max_model_requests": 9000,
          "max_input_tokens_per_request": 131072,
          "attempt_timeout_seconds": 60,
          "max_concurrent_requests": 8,
          "max_transcript_bytes": 8388608,
          "max_request_bytes": 262144,
          "max_questions_per_request": 32,
          "retry_count": 2,
          "initial_backoff_ms": 500,
          "max_backoff_ms": 8000,
          "max_response_bytes": 1048576
        },
        "diagnostics": []
      }
    }
  ]
}"#;

fn completed_cases() -> Value {
    serde_json::from_str(COMPLETED_CASES).unwrap()
}

async fn show_saved(root: &Path, selector: &str) -> std::process::Output {
    tokio::time::timeout(
        Duration::from_secs(10),
        support::isolated_command(root)
            .env_remove("OPENROUTER_API_KEY")
            .args(["lint", "show", "saved.json", selector])
            .output(),
    )
    .await
    .unwrap()
    .unwrap()
}

fn successful_detail(output: &std::process::Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test]
async fn completed_cases_show_unique_result_and_exact_case_from_saved_bytes_only() {
    // Arrange: no configuration, source files or credential exists here.
    let root = tempfile::tempdir().unwrap();
    save(&root.path().join("saved.json"), &completed_cases());

    // Act: numeric result and string case IDs reach the saved JSON reader.
    let numeric = successful_detail(&show_saved(root.path(), "68").await);
    let case = successful_detail(&show_saved(root.path(), "r03-current-typescript-topics").await);

    // Assert: ordinary detail and the existing prepared-case shape are preserved.
    assert_eq!(numeric["result"]["id"], 68);
    assert_eq!(case["case"]["id"], "r03-current-typescript-topics");
    assert_eq!(case["revision"], "89dc638e3fe6a865cb7c4a00af9c3c4440549259");
    assert_eq!(case["detail"]["result"]["id"], 27);
    assert!(numeric.get("runs").is_none());
    assert!(case.get("runs").is_none());
}

#[tokio::test]
async fn completed_cases_refuse_ambiguous_ids_with_every_candidate() {
    let root = tempfile::tempdir().unwrap();
    save(&root.path().join("saved.json"), &completed_cases());

    let output = show_saved(root.path(), "27").await;

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let message = String::from_utf8_lossy(&output.stderr);
    for expected in [
        "ambiguous",
        "completed cases report",
        "27",
        "89dc638e3fe6a865cb7c4a00af9c3c4440549259",
        "9f7b2e283e95dc849f4e271a5287e3ae21959ca9",
        "r03-current-typescript-topics",
        "no case id",
    ] {
        assert!(message.contains(expected), "missing {expected}: {message}");
    }
    assert!(!message.contains("missing field"), "{message}");
}

#[tokio::test]
async fn prepared_cases_keep_ordinals_and_add_exact_case_ids_including_reason_only_cases() {
    let root = tempfile::tempdir().unwrap();
    let mut report = completed_cases();
    report["mode"] = json!("prepare");
    report["status"] = json!({"kind":"prepared"});
    let mut reason_only = report["cases"][0].clone();
    reason_only["id"] = json!("stale-case");
    reason_only["case"]["id"] = json!("stale-case");
    reason_only["result_id"] = Value::Null;
    reason_only["outcome"] = json!({"kind":"stale"});
    report["cases"].as_array_mut().unwrap().push(reason_only);
    save(&root.path().join("saved.json"), &report);

    let ordinal = successful_detail(&show_saved(root.path(), "1").await);
    let exact = successful_detail(&show_saved(root.path(), "r03-current-typescript-topics").await);
    let reason = successful_detail(&show_saved(root.path(), "stale-case").await);

    assert_eq!(ordinal, exact);
    assert_eq!(ordinal["detail"]["result"]["id"], 27);
    assert_eq!(reason["case"]["outcome"]["kind"], "stale");
    assert!(reason.get("detail").is_none());
    let unavailable = show_saved(root.path(), "68").await;
    assert_eq!(unavailable.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unavailable.stderr).contains("unknown case ordinal 68"));
}

#[tokio::test]
async fn saved_report_refusals_name_kind_and_reason_without_raw_serde_errors() {
    let root = tempfile::tempdir().unwrap();
    let mut malformed_lint = completed_cases()["runs"][0]["report"].clone();
    malformed_lint.as_object_mut().unwrap().remove("results");
    let mut malformed_cases = completed_cases();
    malformed_cases["runs"][0]["report"]["results"] = json!("bad result collection");
    for (report, selector, expected) in [
        (completed_cases(), "999", "unknown result id 999"),
        (
            completed_cases(),
            "absent-case",
            "unknown case id absent-case",
        ),
        (malformed_lint, "27", "malformed lint report"),
        (malformed_cases, "27", "malformed completed cases report"),
        (
            json!({"version":"0.11.0", "pieces":[], "usage":{}}),
            "1",
            "unsupported saved report kind",
        ),
    ] {
        save(&root.path().join("saved.json"), &report);
        let output = show_saved(root.path(), selector).await;
        assert_eq!(output.status.code(), Some(2));
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(message.contains(expected), "expected {expected}: {message}");
        for raw in [
            "missing field",
            "invalid type",
            "at line",
            "expected struct",
        ] {
            assert!(!message.contains(raw), "raw serde error: {message}");
        }
    }
    std::fs::write(root.path().join("saved.json"), "{bad JSON").unwrap();
    let output = show_saved(root.path(), "1").await;
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("malformed saved report JSON"));
}

#[tokio::test]
async fn ordinary_lint_report_keeps_numeric_detail_and_names_unknown_selectors() {
    let root = tempfile::tempdir().unwrap();
    save(
        &root.path().join("saved.json"),
        &completed_cases()["runs"][0]["report"],
    );

    let detail = successful_detail(&show_saved(root.path(), "27").await);

    assert_eq!(detail["result"]["id"], 27);
    assert!(detail.get("case").is_none());
    let output = show_saved(root.path(), "case-name").await;
    assert_eq!(output.status.code(), Some(2));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("lint report"), "{message}");
    assert!(message.contains("numeric result id"), "{message}");
}

#[tokio::test]
async fn prepared_numeric_case_ids_fall_back_after_valid_ordinals() {
    // Arrange: numeric case IDs are legal, but prepared ordinals keep priority.
    let root = tempfile::tempdir().unwrap();
    let mut report = completed_cases();
    report["mode"] = json!("prepare");
    report["status"] = json!({"kind":"prepared"});
    let mut numeric_case = report["cases"][0].clone();
    numeric_case["id"] = json!("1");
    numeric_case["case"]["id"] = json!("1");
    numeric_case["result_id"] = Value::Null;
    numeric_case["outcome"] = json!({"kind":"stale"});
    report["cases"]
        .as_array_mut()
        .unwrap()
        .push(numeric_case.clone());
    numeric_case["id"] = json!("68");
    numeric_case["case"]["id"] = json!("68");
    report["cases"].as_array_mut().unwrap().push(numeric_case);
    save(&root.path().join("saved.json"), &report);

    let ordinal = successful_detail(&show_saved(root.path(), "1").await);
    let exact_fallback = successful_detail(&show_saved(root.path(), "68").await);

    assert_eq!(ordinal["case"]["id"], "r03-current-typescript-topics");
    assert_eq!(exact_fallback["case"]["id"], "68");
    assert!(exact_fallback.get("detail").is_none());
}

#[tokio::test]
async fn completed_case_without_matching_result_shows_its_recorded_outcome() {
    let root = tempfile::tempdir().unwrap();
    let mut report = completed_cases();
    report["cases"][0]["result_id"] = Value::Null;
    report["cases"][0]["outcome"] = json!({"kind":"no_matching_piece"});
    save(&root.path().join("saved.json"), &report);

    let detail = successful_detail(&show_saved(root.path(), "r03-current-typescript-topics").await);

    assert_eq!(detail["case"]["outcome"]["kind"], "no_matching_piece");
    assert!(detail.get("detail").is_none());
}

#[tokio::test]
async fn duplicate_case_ids_refuse_each_candidate_instead_of_choosing_the_first() {
    for prepared in [false, true] {
        // Arrange: the same exact case ID points to two different saved results.
        let root = tempfile::tempdir().unwrap();
        let mut report = completed_cases();
        let mut duplicate = report["cases"][0].clone();
        duplicate["result_id"] = json!(68);
        report["cases"].as_array_mut().unwrap().push(duplicate);
        if prepared {
            report["mode"] = json!("prepare");
            report["status"] = json!({"kind":"prepared"});
        }
        save(&root.path().join("saved.json"), &report);

        // Act: exact case lookup cannot choose which result the owner meant.
        let output = show_saved(root.path(), "r03-current-typescript-topics").await;

        // Assert: every candidate is named, while prepared ordinals remain usable.
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let message = String::from_utf8_lossy(&output.stderr);
        for expected in [
            "ambiguous case id r03-current-typescript-topics",
            "89dc638e3fe6a865cb7c4a00af9c3c4440549259",
            "result id 27",
            "result id 68",
        ] {
            assert!(message.contains(expected), "missing {expected}: {message}");
        }
        if prepared {
            let ordinal = successful_detail(&show_saved(root.path(), "1").await);
            assert_eq!(ordinal["detail"]["result"]["id"], 27);
        }
    }
}

#[tokio::test]
async fn duplicate_case_revision_runs_refuse_instead_of_choosing_the_first() {
    for prepared in [false, true] {
        // Arrange: two nested runs claim to own evidence for the selected revision.
        let root = tempfile::tempdir().unwrap();
        let mut report = completed_cases();
        let mut duplicate = report["runs"][0].clone();
        duplicate["report"]["results"][0]["outcome"] = json!("finding");
        report["runs"].as_array_mut().unwrap().push(duplicate);
        if prepared {
            report["mode"] = json!("prepare");
            report["status"] = json!({"kind":"prepared"});
        }
        save(&root.path().join("saved.json"), &report);
        let selectors = if prepared {
            vec!["r03-current-typescript-topics", "1"]
        } else {
            vec!["r03-current-typescript-topics"]
        };

        for selector in selectors {
            // Act: selecting the case still leaves its revision evidence ambiguous.
            let output = show_saved(root.path(), selector).await;

            // Assert: refuse without emitting one arbitrary saved result.
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            let message = String::from_utf8_lossy(&output.stderr);
            for expected in [
                "ambiguous case revision evidence",
                "89dc638e3fe6a865cb7c4a00af9c3c4440549259",
                "case id r03-current-typescript-topics",
                "result id 27",
            ] {
                assert!(message.contains(expected), "missing {expected}: {message}");
            }
        }
    }
}

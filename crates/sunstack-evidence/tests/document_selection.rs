#![allow(clippy::unwrap_used, clippy::expect_used)]
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;
#[tokio::test]
async fn real_markdown_paragraph_and_ast_ranges() {
    let root = tempfile::tempdir().unwrap();
    let text = "U001: wrapped\ncontinuation\n\n- U002: list\n\n```\nU003: example\n```\n\n> U004: quote\n\n# U005: heading\n\n`U006: code`\n";
    std::fs::write(root.path().join("skill.md"), text).unwrap();
    let binding = RepositoryBinding {
        key: RepositoryId::new("repo").unwrap(),
        root: root.path().into(),
        input_paths: vec![QueryPath::Glob {
            pattern: "**".into(),
        }],
        change_scope: ChangeScope::Full,
    };
    let doc = capture_document(
        binding.clone(),
        "skill.md".into(),
        10000,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let result = resolve_paragraph(&doc, &TargetId::new("U001").unwrap()).unwrap();
    let SelectionContent::Paragraph { text, span } = result else {
        unreachable!()
    };
    assert_eq!(text, "U001: wrapped\ncontinuation");
    assert_eq!(&doc.text[span.byte_start..span.byte_end], text);
    assert!(resolve_paragraph(&doc, &TargetId::new("U002").unwrap()).is_ok());
    for id in ["U003", "U004", "U005", "U006", "U999"] {
        assert_eq!(
            resolve_paragraph(&doc, &TargetId::new(id).unwrap())
                .unwrap_err()
                .code,
            EvidenceCode::MissingTarget
        );
    }
    std::fs::write(root.path().join("code.rs"), "fn target() { call(42); }\n").unwrap();
    let code = capture_document(binding, "code.rs".into(), 10000, CancellationToken::new())
        .await
        .unwrap();
    let SelectionContent::Matches { matches } = select_document(
        &code,
        &EvidenceSelection::AstPattern {
            language: "rust".into(),
            pattern: "call($ARG)".into(),
        },
        10,
    )
    .unwrap() else {
        unreachable!()
    };
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].text, "call(42)");
    let argument = &matches[0].captures["ARG"];
    assert_eq!(&code.text[argument.byte_start..argument.byte_end], "42");
}
#[tokio::test]
async fn duplicate_deleted_literal_and_regex_targets_use_frozen_source_ranges() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("skill.md");
    std::fs::write(&path, "U001: wrapped\ncontinuation\n\nU002: two\n").unwrap();
    let binding = RepositoryBinding {
        key: RepositoryId::new("repo").unwrap(),
        root: root.path().into(),
        input_paths: vec![QueryPath::Exact {
            path: "skill.md".into(),
        }],
        change_scope: ChangeScope::Full,
    };
    let before = capture_document(
        binding.clone(),
        "skill.md".into(),
        10000,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    for select in [
        EvidenceSelection::LiteralText {
            text: "continuation".into(),
        },
        EvidenceSelection::RegexText {
            pattern: "(?P<word>continuation)".into(),
        },
    ] {
        let SelectionContent::Matches { matches } = select_document(&before, &select, 10).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].span.start_line, 2);
        assert_eq!(matches[0].span.start_column, 1);
        assert_eq!(
            &before.text[matches[0].span.byte_start..matches[0].span.byte_end],
            "continuation"
        );
    }
    std::fs::write(&path, "U002: two\n").unwrap();
    let current = capture_document(
        binding.clone(),
        "skill.md".into(),
        10000,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(
        resolve_document_target(&current, Some(&before), &TargetId::new("U001").unwrap()),
        TargetResolution::Deleted
    );
    assert!(resolve_paragraph(&before, &TargetId::new("U001").unwrap()).is_ok());
    std::fs::write(&path, "U001: one\n\nU001: duplicate\n").unwrap();
    let duplicated = capture_document(binding, "skill.md".into(), 10000, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        resolve_document_target(&duplicated, None, &TargetId::new("U001").unwrap()),
        TargetResolution::Ambiguous
    );
}

async fn assert_paragraph_case(markdown: &str, target_id: &str, expected: Option<&str>) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("paragraph.md"), markdown).unwrap();
    let binding = RepositoryBinding {
        key: RepositoryId::new("paragraph_case").unwrap(),
        root: root.path().into(),
        input_paths: vec![QueryPath::Exact {
            path: "paragraph.md".into(),
        }],
        change_scope: ChangeScope::Full,
    };
    let document = capture_document(
        binding,
        "paragraph.md".into(),
        10000,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let result = resolve_paragraph(&document, &TargetId::new(target_id).unwrap());
    if let Some(expected) = expected {
        let SelectionContent::Paragraph { text, span } = result.unwrap() else {
            unreachable!()
        };
        assert_eq!(text, expected);
        assert_eq!(&markdown[span.byte_start..span.byte_end], expected);
        let start = markdown.find(expected).unwrap();
        assert_eq!(
            (span.byte_start, span.byte_end),
            (start, start + expected.len())
        );
    } else {
        assert_eq!(result.unwrap_err().code, EvidenceCode::MissingTarget);
    }
}
macro_rules! paragraph_regression {
    ($name:ident, $markdown:expr, $target:expr, $expected:expr) => {
        #[tokio::test]
        async fn $name() {
            assert_paragraph_case($markdown, $target, $expected).await;
        }
    };
}
paragraph_regression!(
    paragraph_control_ordinary_wrapped,
    "U001: first\ncontinuation\n",
    "U001",
    Some("U001: first\ncontinuation")
);
paragraph_regression!(
    paragraph_regression_tight_list_wrapped,
    "- U002: first\n  continuation\n",
    "U002",
    Some("U002: first\n  continuation")
);
paragraph_regression!(
    paragraph_regression_tight_list_emphasis,
    "- U003: first *emphasis* tail\n",
    "U003",
    Some("U003: first *emphasis* tail")
);
paragraph_regression!(
    paragraph_regression_tight_list_link,
    "- U004: first [label](https://example.com) tail\n",
    "U004",
    Some("U004: first [label](https://example.com) tail")
);
paragraph_regression!(
    paragraph_regression_tight_list_inline_code,
    "- U005: first `code` tail\n",
    "U005",
    Some("U005: first `code` tail")
);
paragraph_regression!(
    paragraph_control_loose_list_wrapped,
    "- U006: first\n  continuation\n\n- next item\n",
    "U006",
    Some("U006: first\n  continuation")
);
paragraph_regression!(
    paragraph_control_inline_code_id,
    "- `U007: example`\n",
    "U007",
    None
);
paragraph_regression!(
    paragraph_regression_list_heading_excluded,
    "- # U008: heading\n",
    "U008",
    None
);
paragraph_regression!(
    paragraph_regression_list_fence_excluded,
    "- ```\n  U009: example\n  ```\n",
    "U009",
    None
);
paragraph_regression!(
    paragraph_control_ordinary_emphasis,
    "U010: first *emphasis* tail\n",
    "U010",
    Some("U010: first *emphasis* tail")
);

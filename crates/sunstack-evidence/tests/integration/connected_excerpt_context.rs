#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[path = "../support/mod.rs"]
mod support;
use sha2::{Digest, Sha256};
use sunstack_evidence::*;
use support::connected_inputs::*;
use support::*;
use tokio_util::sync::CancellationToken;

const BEFORE_TEXT: &str = "// before\nimport { helper as previous } from './helper';\nfunction beforeCaller(value: number): number { return previous(value); }\n";
const AFTER_TEXT: &str = "// after has a different width\n\nimport { helper as current } from './helper';\nfunction afterCaller(value: number): number { return current(value); }\n";
const BEFORE_IMPORT: &str = "import { helper as previous } from './helper';";
const AFTER_IMPORT: &str = "import { helper as current } from './helper';";

fn fixture() -> GitFixture {
    let fixture = GitFixture::new();
    fixture.write("caller.ts", BEFORE_TEXT);
    fixture.write(
        "helper.ts",
        "export function helper(value: number): number { return value; }\n",
    );
    fixture.stage("caller.ts");
    fixture.stage("helper.ts");
    fixture.commit(1000);
    fixture.write("caller.ts", AFTER_TEXT);
    fixture
}
fn input(fixture: &GitFixture, sides: SourceSides) -> EvidencePlanInput {
    let mut select = related(RelationDirection::Callees);
    if let EvidenceSelection::RelatedUnits { source_sides, .. } = &mut select {
        *source_sides = sides;
    }
    plan(
        vec![fixture.binding(ChangeScope::Uncommitted {
            layers: vec![DirtyLayer::Unstaged],
        })],
        select,
        EvidenceRequirement::Required,
    )
}
fn assert_context(readiness: &SelectionReadiness, fixture: &GitFixture, before: bool) {
    let SelectionReadiness::Ready {
        content: SelectionContent::RelatedGroup { group },
        ..
    } = readiness
    else {
        panic!("required context must be ready: {readiness:?}")
    };
    let ContextGroupOutcome::Complete { group, .. } = group.as_ref() else {
        panic!("context must be complete")
    };
    let (text, import, signature, line, revision, side) = if before {
        (
            BEFORE_TEXT,
            BEFORE_IMPORT,
            "function beforeCaller(value: number): number",
            2,
            "index",
            "before",
        )
    } else {
        (
            AFTER_TEXT,
            AFTER_IMPORT,
            "function afterCaller(value: number): number",
            3,
            "worktree",
            "after",
        )
    };
    let excerpt = group
        .excerpts
        .iter()
        .find(|excerpt| excerpt.text.starts_with(signature))
        .unwrap();
    let ExcerptContext::Code {
        enclosing_signature,
        import_context,
    } = &excerpt.context
    else {
        panic!("code context missing")
    };
    assert_eq!(enclosing_signature, signature);
    assert_eq!(import_context.len(), 1);
    let span = &import_context[0];
    let start = text.find(import).unwrap();
    assert_eq!(
        (span.byte_start, span.byte_end),
        (start, start + import.len())
    );
    assert_eq!(
        (
            span.start_line,
            span.start_column,
            span.end_line,
            span.end_column
        ),
        (line, 1, line, import.len() + 1)
    );
    assert_eq!(&text[span.byte_start..span.byte_end], import);
    let SourceRef::RepositoryFile {
        repository,
        path,
        revision: source_revision,
        content_identity,
    } = &span.source
    else {
        panic!("repository source missing")
    };
    assert_eq!(repository.as_str(), "fixture");
    assert_eq!(path.to_str(), Some("caller.ts"));
    assert_eq!(source_revision, &excerpt.unit.revision);
    let expected_identity = if before {
        fixture
            .repository
            .index()
            .unwrap()
            .get_path(std::path::Path::new("caller.ts"), 0)
            .unwrap()
            .id
            .to_string()
    } else {
        format!("{:x}", Sha256::digest(AFTER_TEXT.as_bytes()))
    };
    assert_eq!(content_identity, &expected_identity);
    if before {
        assert!(matches!(
            source_revision,
            SourceRevision::Index {
                side: SourceSide::Before,
                ..
            }
        ));
    } else {
        assert!(
            matches!(source_revision, SourceRevision::Worktree { snapshot, side: SourceSide::After } if snapshot == &excerpt.unit.snapshot)
        );
    }
    let wire = serde_json::to_value(readiness).unwrap();
    let wire_excerpt = wire["content"]["group"]["group"]["excerpts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| {
            value["text"]
                .as_str()
                .is_some_and(|text| text.starts_with(signature))
        })
        .unwrap();
    let context = &wire_excerpt["context"];
    assert_eq!(context["kind"], "code");
    assert_eq!(context["enclosing_signature"], signature);
    assert_eq!(context["import_context"].as_array().unwrap().len(), 1);
    let wire_span = &context["import_context"][0];
    assert_eq!(wire_span["byte_start"], start);
    assert_eq!(wire_span["byte_end"], start + import.len());
    assert_eq!(wire_span["source"]["kind"], "repository_file");
    assert_eq!(wire_span["source"]["repository"], "fixture");
    assert_eq!(wire_span["source"]["path"], "caller.ts");
    assert_eq!(wire_span["source"]["revision"]["kind"], revision);
    assert_eq!(wire_span["source"]["revision"]["side"], side);
    assert_eq!(wire_span["source"]["content_identity"], expected_identity);
}
#[tokio::test]
async fn public_preparation_keeps_literal_signatures_and_versioned_import_spans() {
    let fixture = fixture();
    for (sides, before) in [
        (SourceSides::BeforeOnly, true),
        (SourceSides::AfterOnly, false),
    ] {
        let outcome = prepare_evidence(
            input(&fixture, sides),
            AdapterRegistry::builtins().unwrap(),
            CancellationToken::new(),
        )
        .await;
        let EvidencePlanOutcome::Prepared { selections, .. } = outcome else {
            panic!("public preparation failed: {outcome:?}")
        };
        assert_context(&selections[0].readiness, &fixture, before);
    }
}
#[tokio::test]
async fn public_session_keeps_import_context_and_signature_after_live_source_rewrite() {
    let fixture = fixture();
    let input = input(&fixture, SourceSides::AfterOnly);
    let declaration = input.declarations[0].clone();
    let mut session = EvidenceSession::open(
        input,
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let first = session.select(&declaration).await;
    assert_context(&first.readiness, &fixture, false);
    fixture.write(
        "caller.ts",
        "import { changed } from './missing'; function replacement() { changed(); }",
    );
    let repeated = session.select(&declaration).await;
    assert_context(&repeated.readiness, &fixture, false);
    assert_eq!(
        serde_json::to_value(&first.readiness).unwrap(),
        serde_json::to_value(&repeated.readiness).unwrap()
    );
    assert_eq!(
        session.close().measurements.files_read,
        4,
        "frozen contexts must not reread the changed live file"
    );
}

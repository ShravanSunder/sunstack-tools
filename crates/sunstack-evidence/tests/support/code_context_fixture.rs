#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use crate::support::*;
use serde_json::json;
use sunstack_evidence::*;
use tokio_util::sync::CancellationToken;
pub fn repository(deleted: bool) -> (GitFixture, String, String) {
    let fixture = GitFixture::new();
    let before = format!(
        "mod helper; mod caller; use crate::helper::callee;\nconst TOP: &str = \"TOP_MATCH\";\npub fn touched() {{\n unflagged(\"old\");\n flagged(\"UNCHANGED_MATCH\");\n{} callee(); missing_name();\n}}\n",
        if deleted {
            " flagged(\"DELETED_MATCH\");\n"
        } else {
            ""
        }
    );
    fixture.write("src/lib.rs", &before);
    fixture.write(
        "src/helper.rs",
        "pub fn callee() { let fact = \"COMMITTED_CALLEE\"; }\n",
    );
    fixture.write(
        "src/caller.rs",
        "pub fn caller() { crate::touched(); let fact = \"COMMITTED_CALLER\"; }\n",
    );
    for path in ["src/lib.rs", "src/helper.rs", "src/caller.rs"] {
        fixture.stage(path);
    }
    let base = fixture.commit(1000).to_string();
    fixture.write(
        "src/lib.rs",
        &before
            .replace("unflagged(\"old\")", "flagged(\"CHANGED_MATCH\")")
            .replace(" flagged(\"DELETED_MATCH\");\n", ""),
    );
    fixture.stage("src/lib.rs");
    let head = fixture.commit(1100).to_string();
    fixture.write(
        "src/caller.rs",
        "pub fn caller() { crate::touched(); let fact = \"DIRTY_NOT_AT_REVISION\"; }\n",
    );
    (fixture, base, head)
}
pub fn frozen_text(fixture: &GitFixture, revision: &str, path: &str) -> String {
    let commit = fixture
        .repository
        .find_commit(git2::Oid::from_str(revision).unwrap())
        .unwrap();
    let tree = commit.tree().unwrap();
    let entry = tree.get_path(std::path::Path::new(path)).unwrap();
    let blob = fixture.repository.find_blob(entry.id()).unwrap();
    String::from_utf8(blob.content().to_vec()).unwrap()
}
pub fn exact_span(
    span: &SourceSpan,
    text: &str,
    snippet: &str,
    path: &str,
    revision: &str,
    side: SourceSide,
) {
    let start = text.find(snippet).unwrap();
    let end = start + snippet.len();
    let point = |offset: usize| {
        let prefix = &text[..offset];
        (
            prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
            prefix.rsplit('\n').next().unwrap().chars().count() + 1,
        )
    };
    assert_eq!((span.byte_start, span.byte_end), (start, end));
    assert_eq!((span.start_line, span.start_column), point(start));
    assert_eq!((span.end_line, span.end_column), point(end));
    let SourceRef::RepositoryFile {
        repository,
        revision: actual,
        path: actual_path,
        ..
    } = &span.source
    else {
        panic!("not a repository span")
    };
    assert_eq!(repository.as_str(), "fixture");
    assert_eq!(actual_path, std::path::Path::new(path));
    assert_eq!(
        *actual,
        SourceRevision::Commit {
            commit_id: GitObjectId::new(revision).unwrap(),
            side
        }
    );
}
pub async fn session(fixture: &GitFixture) -> EvidenceSession {
    EvidenceSession::open(
        EvidencePlanInput {
            repositories: vec![fixture.binding(ChangeScope::Full {})],
            sources: Default::default(),
            payload_schemas: vec![],
            cursors: vec![],
            declarations: vec![],
            native_queries: vec![],
            package_bindings: vec![],
            limits: InvocationLimits {
                max_files: 100.try_into().unwrap(),
                max_file_bytes: 100000.try_into().unwrap(),
                max_total_bytes: 1000000.try_into().unwrap(),
                max_matches: 1000.try_into().unwrap(),
                max_transcript_bytes: 100000.try_into().unwrap(),
                include_ignored: false,
            },
            discovery: EvidenceDiscovery::Bounded {
                limits: discovery_limits(),
            },
        },
        AdapterRegistry::builtins().unwrap(),
        CancellationToken::new(),
    )
    .await
    .unwrap()
}
pub fn pass(pattern: &str) -> MechanicalPass {
    serde_json::from_value(json!({"predicate":{"kind":"ast","language":"rust","pattern":pattern}}))
        .unwrap()
}
pub fn code(head: &str, paths: Vec<QueryPath>) -> SourceInput {
    SourceInput::Code {
        repository: RepositoryId::new("fixture").unwrap(),
        revision: RevisionHead::Commit {
            revision: head.into(),
        },
        paths,
    }
}

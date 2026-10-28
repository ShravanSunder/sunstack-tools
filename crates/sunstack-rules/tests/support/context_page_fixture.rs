//! Native fixtures for the pure context projection contract.
#![allow(dead_code)]
use sunstack_evidence::*;
use sunstack_rules::{ContextPage, ContextSettings, DiagnosticList, TruncationSettings};

pub(super) fn scalar_cost(page: &ContextPage) -> Result<usize, DiagnosticList> {
    let rendered = serde_json::to_string(&serde_json::json!({"page":page.text})).unwrap();
    let empty = serde_json::to_string(&serde_json::json!({"page":""})).unwrap();
    Ok(rendered.chars().count() - empty.chars().count())
}
pub(super) fn unit(name: &str, text: String, side: SourceSide) -> ContextUnit {
    let revision = SourceRevision::Commit {
        commit_id: GitObjectId::new(if side == SourceSide::Before {
            "a".repeat(40)
        } else {
            "b".repeat(40)
        })
        .unwrap(),
        side,
    };
    let span = SourceSpan {
        source: SourceRef::RepositoryFile {
            repository: RepositoryId::new("repo").unwrap(),
            revision: revision.clone(),
            path: format!("{name}.rs").into(),
            content_identity: content_digest(text.as_bytes()),
        },
        byte_start: 0,
        byte_end: text.len(),
        start_line: 10,
        start_column: 1,
        end_line: 10 + text.lines().count() - 1,
        end_column: 2,
    };
    ContextUnit {
        piece_id: PieceId::new(name).unwrap(),
        unit: SourceUnit {
            key: SourceUnitKey {
                snapshot: SnapshotId::new("snapshot").unwrap(),
                repository: RepositoryId::new("repo").unwrap(),
                revision,
                path: format!("{name}.rs").into(),
                byte_start: 0,
                byte_end: text.len(),
            },
            kind: SourceUnitKind::Function {
                signature: format!("fn {name}() {{"),
            },
            span,
            content_identity: content_digest(text.as_bytes()),
            syntax_quality: SyntaxQuality::Parsed,
        },
        text,
    }
}
pub(super) fn large_function(name: &str, head: &str, line_prefix: &str) -> ContextUnit {
    let body = (1..=100)
        .map(|n| format!("{line_prefix}{n:03} {}", "x".repeat(400)))
        .collect::<Vec<_>>()
        .join("\n");
    unit(
        name,
        format!("fn {name}() {{\n{head}\n{body}\nTAIL\n}}"),
        SourceSide::After,
    )
}
pub(super) fn truncation(head_lines: usize, tail_lines: usize) -> ContextSettings {
    ContextSettings {
        truncation: TruncationSettings {
            head_lines,
            tail_lines,
        },
    }
}
pub(super) fn context(units: Vec<ContextUnit>) -> CandidateContext {
    CandidateContext {
        context_id: CodeContextId::new("context").unwrap(),
        input_id: SourceInputId::new("input").unwrap(),
        repository: RepositoryId::new("repo").unwrap(),
        matched: CodeExcerpt {
            span: units[0].unit.span.clone(),
            text: units[0].text.clone(),
        },
        changes: vec![],
        units,
        redactions: vec![],
        related: vec![],
        imports: vec![],
        coverage: DiscoveryCoverage::CompleteWithinDeclaredScope {
            query_bounds: QueryBounds {
                repository_ids: vec![],
                requested_hops: 1,
                windows: vec![],
            },
        },
    }
}
pub(super) fn relation(
    id: &str,
    seed: &ContextUnit,
    target: &ContextUnit,
    uncertainties: Vec<RelationshipUncertainty>,
) -> RelatedPiece {
    RelatedPiece::Resolved {
        piece_id: PieceId::new(id).unwrap(),
        seed: seed.unit.key.clone(),
        connection: RelationDirection::Callees,
        unit: target.unit.key.clone(),
        facts: vec![RelationshipFact::StaticCall {
            span: seed.unit.span.clone(),
        }],
        uncertainties,
        contradictions: vec![],
    }
}
pub(super) fn positioned_match(context: &mut CandidateContext, first: usize, last: usize) {
    let unit = &context.units[0];
    let lines = unit.text.split_inclusive('\n').collect::<Vec<_>>();
    let byte_start = lines
        .iter()
        .take(first)
        .map(|line| line.len())
        .sum::<usize>();
    let byte_end = lines
        .iter()
        .take(last + 1)
        .map(|line| line.len())
        .sum::<usize>();
    context.matched = CodeExcerpt {
        span: SourceSpan {
            byte_start,
            byte_end,
            start_line: unit.unit.span.start_line + first,
            end_line: unit.unit.span.start_line + last + 1,
            end_column: 1,
            ..unit.unit.span.clone()
        },
        text: unit.text[byte_start..byte_end].into(),
    };
}

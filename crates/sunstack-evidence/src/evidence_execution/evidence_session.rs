use crate::{evidence_execution::evidence_preparation::Preparation, *};
use std::time::Instant;
use tokio_util::sync::CancellationToken;

/// One native invocation's lazy captures and cumulative limits. It has no provider or rules dependency.
pub struct EvidenceSession {
    preparation: Preparation,
    started: Instant,
}
pub struct EvidenceSessionSummary {
    pub captures: Vec<CaptureCoverage>,
    pub change_selections: Vec<ResolvedChangeSelection>,
    pub cursors: Vec<ReturnedCursor>,
    pub measurements: EvidenceMeasurements,
}
impl EvidenceSession {
    pub async fn open(
        input: EvidencePlanInput,
        registry: AdapterRegistry,
        cancellation: CancellationToken,
    ) -> EvidenceResult<Self> {
        let started = Instant::now();
        let preparation = Preparation::open(input, registry, cancellation).await?;
        Ok(Self {
            preparation,
            started,
        })
    }
    pub async fn select(&mut self, declaration: &EvidenceDeclaration) -> SelectionResult {
        let readiness = match self.preparation.select(declaration).await {
            Ok(readiness) => readiness,
            Err(error) => SelectionReadiness::Refused {
                diagnostics: error.diagnostics(),
            },
        };
        SelectionResult {
            id: declaration.id.clone(),
            readiness,
        }
    }
    pub(crate) async fn select_native(&mut self, query: &NativeQuery) -> SelectionResult {
        let readiness = match self.preparation.native_select(query).await {
            Ok(readiness) => readiness,
            Err(error) => SelectionReadiness::Refused {
                diagnostics: error.diagnostics(),
            },
        };
        SelectionResult {
            id: query.id.clone(),
            readiness,
        }
    }
    pub async fn compare_change_sides(
        &mut self,
        subject: ChangeComparisonSubject,
        level: ComparisonLevel,
    ) -> ChangeComparison {
        self.preparation.compare_change_sides(subject, level).await
    }
    /// Inspect a selected raw unit on its already captured full Markdown tree.
    pub async fn markdown_matches<M: ast_grep_core::Matcher + Send + 'static>(
        &mut self,
        source: &SourceRef,
        unit: MarkdownByteRange,
        matcher: M,
    ) -> EvidenceResult<Vec<SourceSpan>> {
        self.preparation
            .markdown_matches(source, unit, matcher)
            .await
    }
    pub fn measurements(&self) -> &EvidenceMeasurements {
        self.preparation.measurements()
    }
    pub fn close(self) -> EvidenceSessionSummary {
        self.preparation.close(self.started)
    }
}

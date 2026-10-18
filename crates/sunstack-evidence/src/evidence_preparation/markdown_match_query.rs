use super::*;

impl Preparation {
    pub(crate) async fn markdown_matches<M: ast_grep_core::Matcher + Send + 'static>(
        &mut self,
        source: &SourceRef,
        unit: MarkdownByteRange,
        matcher: M,
    ) -> EvidenceResult<Vec<SourceSpan>> {
        if self.cancellation.is_cancelled() {
            return Err(EvidenceCode::MissingContext.into());
        }
        // A local check may inspect only a target whose canonical selection
        // already acquired these exact bytes. It cannot initiate acquisition.
        let document = self
            .document_cache
            .values()
            .find(|document| &document.source == source)
            .cloned()
            .ok_or(EvidenceCode::SourceUnavailable)?;
        if document.text.get(unit.start..unit.end).is_none() {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        let max_matches = self.input.limits.max_matches.get();
        let scanned = document.text.len();
        let source = document.source.clone();
        let (spans, limited) = tokio::task::spawn_blocking(move || {
            MarkdownDocument::parse(&document.text).matching_spans(matcher, &unit, max_matches)
        })
        .await
        .map_err(|_| EvidenceCode::InvalidSelector)?;
        self.measurements.files_parsed += 1;
        self.measurements.bytes_scanned += scanned;
        if limited {
            return Err(EvidenceCode::SelectorLimit.into());
        }
        Ok(spans
            .into_iter()
            .map(|span| SourceSpan {
                source: source.clone(),
                byte_start: span.byte_start,
                byte_end: span.byte_end,
                start_line: span.start_line,
                start_column: span.start_column,
                end_line: span.end_line,
                end_column: span.end_column,
            })
            .collect())
    }
}

use crate::{markdown_offset_projection::NormalizedMarkdown, *};
use std::{collections::BTreeMap, ops::Range};

pub(crate) fn select_frontmatter(
    document: &CapturedDocument,
    field: &str,
) -> EvidenceResult<SelectionContent> {
    if field.is_empty() {
        return Err(EvidenceCode::InvalidSelector.into());
    }
    let normalized = NormalizedMarkdown::new(&document.text);
    let unavailable = |reason| {
        let mut diagnostic = Diagnostic::new(EvidenceCode::FrontmatterUnavailable);
        diagnostic.location = DiagnosticLocation::Source {
            source: document.source.clone(),
            span: None,
        };
        FrontmatterFieldContent::BlockUnavailable {
            source: document.source.clone(),
            field: field.into(),
            reason,
            diagnostics: DiagnosticList {
                items: vec![diagnostic],
                ..DiagnosticList::default()
            },
        }
    };
    let result = match header_ranges(normalized.as_str()) {
        Err(reason) => unavailable(reason),
        Ok((body, header)) => {
            type Fields = BTreeMap<String, serde_saphyr::Spanned<serde_json::Value>>;
            // The maintained decoder enforces its default depth/alias/event budgets,
            // rejects duplicate keys, and has no include or filesystem feature enabled.
            match serde_saphyr::from_str::<Option<Fields>>(&normalized.as_str()[body.clone()]) {
                Err(_) => unavailable(FrontmatterUnavailableReason::Malformed),
                Ok(fields) => match fields.unwrap_or_default().remove(field) {
                    None => FrontmatterFieldContent::Absent {
                        source: document.source.clone(),
                        field: field.into(),
                        frontmatter_span: source_span(
                            &document.source,
                            &document.text,
                            normalized.original_range(header),
                        ),
                    },
                    Some(value) => {
                        let location = value.referenced.span();
                        let range = location.byte_offset().zip(location.byte_len()).and_then(
                            |(start, len)| {
                                let start = usize::try_from(start).ok()?;
                                let end = start.checked_add(usize::try_from(len).ok()?)?;
                                (end <= body.len()).then_some(body.start + start..body.start + end)
                            },
                        );
                        match (range, serde_json::from_value::<StructuredJson>(value.value)) {
                            (Some(range), Ok(value)) => FrontmatterFieldContent::Present {
                                source: document.source.clone(),
                                field: field.into(),
                                value,
                                span: source_span(
                                    &document.source,
                                    &document.text,
                                    normalized.original_range(range),
                                ),
                            },
                            _ => unavailable(FrontmatterUnavailableReason::Malformed),
                        }
                    }
                },
            }
        }
    };
    Ok(SelectionContent::FrontmatterField {
        result: Box::new(result),
    })
}

fn header_ranges(text: &str) -> Result<(Range<usize>, Range<usize>), FrontmatterUnavailableReason> {
    let mut lines = text.split_inclusive('\n');
    let delimiter = |line: &str| line.trim_end_matches(['\n', ' ', '\t']) == "---";
    let Some(first) = lines.next().filter(|line| delimiter(line)) else {
        return Err(FrontmatterUnavailableReason::Missing);
    };
    let body_start = first.len();
    let mut offset = body_start;
    for line in lines {
        if delimiter(line) {
            return Ok((body_start..offset, 0..offset + line.len()));
        }
        offset += line.len();
    }
    Err(FrontmatterUnavailableReason::Malformed)
}

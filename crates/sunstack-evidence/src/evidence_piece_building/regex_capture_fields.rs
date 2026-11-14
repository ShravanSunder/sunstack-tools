use super::*;
use std::collections::BTreeSet;

/// Capture raw field strings; derived asking text remains literal rather than HTML markup.
pub(super) fn capture_list(
    pattern: &str,
    input: Option<&FieldValue>,
    named: bool,
    file: &MatchedFile,
    own_range: &MatchRange,
    contents: Option<&BTreeMap<PathBuf, Result<String, CheckedReadError>>>,
) -> FieldValue {
    let own_span = [source_span(file, own_range)];
    let inputs = match input {
        None if !named => vec![(&file.text[own_range.start..own_range.end], &own_span[..])],
        Some(FieldValue::Text { value, spans }) => vec![(value.as_str(), spans.as_slice())],
        Some(
            FieldValue::Texts { values, spans, .. } | FieldValue::Document { values, spans, .. },
        ) if values.len() == spans.len() => values
            .iter()
            .zip(spans)
            .map(|(value, span)| (value.as_str(), std::slice::from_ref(span)))
            .collect(),
        Some(FieldValue::Absent) => vec![],
        Some(FieldValue::Unavailable { reason }) => {
            return FieldValue::Unavailable { reason: *reason };
        }
        _ => return unavailable(),
    };
    let Ok(regex) = Regex::new(pattern) else {
        return unavailable();
    };
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    let mut spans = Vec::new();
    for (text, context) in inputs {
        for captures in regex.captures_iter(text) {
            let selected = if regex.captures_len() == 1 {
                captures.get(0).filter(|value| !value.is_empty())
            } else {
                captures
                    .iter()
                    .skip(1)
                    .flatten()
                    .find(|value| !value.is_empty())
            };
            let Some(selected) = selected else {
                continue;
            };
            if !seen.insert(selected.as_str().to_owned()) {
                continue;
            }
            let Some(span) = capture_span(text, context, selected.range(), file, contents) else {
                return unavailable();
            };
            values.push(selected.as_str().to_owned());
            spans.push(span);
        }
    }
    FieldValue::Texts {
        cut: input.is_some_and(FieldValue::is_cut),
        values,
        spans,
    }
}

fn unavailable() -> FieldValue {
    FieldValue::Unavailable {
        reason: FieldUnavailable::CaptureSourceUnavailable,
    }
}

/// Exact byte witnesses for raw strings; transformed strings retain their input context.
fn capture_span(
    value: &str,
    context: &[SourceSpan],
    selected: std::ops::Range<usize>,
    file: &MatchedFile,
    contents: Option<&BTreeMap<PathBuf, Result<String, CheckedReadError>>>,
) -> Option<SourceSpan> {
    let first = context.first()?;
    let original = if first.source == file.source {
        Some(file.text.as_str())
    } else if let SourceRef::Document { path, .. } = &first.source {
        contents
            .and_then(|contents| contents.get(path))
            .and_then(|text| text.as_ref().ok())
            .map(String::as_str)
    } else {
        None
    };
    if let Some(original) = original {
        let raw = context
            .iter()
            .map(|span| {
                (span.source == first.source)
                    .then(|| original.get(span.byte_start..span.byte_end))
                    .flatten()
            })
            .collect::<Option<Vec<_>>>()
            .map(|parts| parts.concat());
        if raw
            .as_ref()
            .is_some_and(|raw| raw == value || raw.trim_end_matches(['\r', '\n']) == value)
        {
            let mut offset = 0;
            let mut start = None;
            let mut end = None;
            for span in context {
                let length = span.byte_end - span.byte_start;
                if selected.start >= offset && selected.start < offset + length {
                    start = Some(span.byte_start + selected.start - offset);
                }
                if selected.end > offset && selected.end <= offset + length {
                    end = Some(span.byte_start + selected.end - offset);
                }
                offset += length;
            }
            if let (Some(start), Some(end)) = (start, end) {
                return Some(crate::source_acquisition::source_capture::source_span(
                    &first.source,
                    original,
                    start..end,
                ));
            }
        }
    }
    // YAML decoding or heading projection can change bytes. Do not invent exact offsets.
    let mut span = first.clone();
    if let Some(last) = context.last().filter(|last| last.source == first.source) {
        span.byte_end = last.byte_end;
        span.end_line = last.end_line;
        span.end_column = last.end_column;
    }
    Some(span)
}

#[cfg(test)]
#[path = "regex_capture_fields_test.rs"]
mod tests;

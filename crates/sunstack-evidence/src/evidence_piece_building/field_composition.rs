use super::*;
pub(super) fn source_span(file: &MatchedFile, range: &MatchRange) -> SourceSpan {
    crate::source_acquisition::source_capture::source_span(
        &file.source,
        &file.text,
        range.start..range.end,
    )
}

fn first_line_range(file: &MatchedFile, range: &MatchRange) -> MatchRange {
    let length = file.text[range.start..range.end]
        .find('\n')
        .map_or(range.end - range.start, |n| n + 1);
    MatchRange {
        start: range.start,
        end: range.start + length,
    }
}

pub(super) fn local_fields(
    definition: &EvidenceKindDefinition,
    file: &MatchedFile,
    range: &MatchRange,
    match_position: usize,
    pieces: &[EvidencePiece],
) -> BTreeMap<FieldName, FieldValue> {
    let text = &file.text[range.start..range.end];
    let mut taken = Vec::new();
    let mut fields = BTreeMap::new();
    for (name, source) in &definition.fields {
        let value = match source {
            FieldSource::FirstLine {} => {
                let first = first_line_range(file, range);
                taken.push(first.clone());
                FieldValue::Text {
                    value: file.text[first.start..first.end]
                        .trim_end_matches(['\r', '\n'])
                        .into(),
                    spans: vec![source_span(file, &first)],
                }
            }
            FieldSource::Capture { regex } => {
                match Regex::new(regex).ok().and_then(|regex| {
                    regex.captures(text).and_then(|capture| {
                        let whole = capture.get(0)?;
                        let group = capture
                            .iter()
                            .skip(1)
                            .flatten()
                            .find(|value| !value.as_str().is_empty())?;
                        Some((whole.range(), group.range()))
                    })
                }) {
                    Some((whole, group)) => {
                        taken.push(MatchRange {
                            start: range.start + whole.start,
                            end: range.start + whole.end,
                        });
                        let selected = MatchRange {
                            start: range.start + group.start,
                            end: range.start + group.end,
                        };
                        FieldValue::Text {
                            value: file.text[selected.start..selected.end].into(),
                            spans: vec![source_span(file, &selected)],
                        }
                    }
                    None => FieldValue::Absent,
                }
            }
            FieldSource::Inside { of }
            | FieldSource::Enclosing { of }
            | FieldSource::SameFile { of } => {
                let mut selected = source_ranges(of, file, pieces)
                    .into_iter()
                    .filter(|candidate| match source {
                        FieldSource::Inside { .. } => match_within(candidate, range),
                        FieldSource::Enclosing { .. } => match_within(range, candidate),
                        _ => true,
                    })
                    .collect::<Vec<_>>();
                if matches!(source, FieldSource::Enclosing { .. }) {
                    selected.sort_by_key(|span| (span.start, std::cmp::Reverse(span.end)));
                }
                let spans = selected
                    .iter()
                    .map(|candidate| {
                        if matches!(source, FieldSource::Inside { .. }) {
                            taken.push(candidate.clone());
                            candidate.clone()
                        } else {
                            first_line_range(file, candidate)
                        }
                    })
                    .collect::<Vec<_>>();
                FieldValue::Texts {
                    values: spans
                        .iter()
                        .map(|span| {
                            let text = &file.text[span.start..span.end];
                            if matches!(source, FieldSource::Inside { .. }) {
                                text.into()
                            } else {
                                text.trim_end_matches(['\r', '\n']).into()
                            }
                        })
                        .collect(),
                    spans: spans.iter().map(|span| source_span(file, span)).collect(),
                }
            }
            FieldSource::Next { rule, count } | FieldSource::Previous { rule, count } => {
                neighbour_field(
                    definition,
                    file,
                    match_position,
                    rule,
                    *count,
                    matches!(source, FieldSource::Previous { .. }),
                )
            }
            FieldSource::Yaml { key } => yaml_field(file, range, key),
            FieldSource::Remainder { .. } | FieldSource::Target { .. } => continue,
        };
        fields.insert(name.clone(), value);
    }
    for (name, source) in &definition.fields {
        let FieldSource::Remainder { exclude } = source else {
            continue;
        };
        let mut field_taken = taken.clone();
        if let Some(of) = exclude {
            field_taken.extend(
                source_ranges(of, file, pieces)
                    .into_iter()
                    .filter(|candidate| {
                        match_within(candidate, range)
                            && (candidate.start != range.start || candidate.end != range.end)
                    }),
            );
        }
        fields.insert(name.clone(), remainder_field(file, range, field_taken));
    }
    fields
}

fn remainder_field(
    file: &MatchedFile,
    range: &MatchRange,
    mut taken: Vec<MatchRange>,
) -> FieldValue {
    taken.sort_by_key(|span| (span.start, span.end));
    let mut cursor = range.start;
    let mut value = String::new();
    let mut spans = Vec::new();
    for removed in taken {
        if removed.start > cursor {
            value.push_str(&file.text[cursor..removed.start]);
            spans.push(source_span(
                file,
                &MatchRange {
                    start: cursor,
                    end: removed.start,
                },
            ));
        }
        cursor = cursor.max(removed.end);
    }
    if cursor < range.end {
        value.push_str(&file.text[cursor..range.end]);
        spans.push(source_span(
            file,
            &MatchRange {
                start: cursor,
                end: range.end,
            },
        ));
    }
    FieldValue::Text { value, spans }
}

fn yaml_field(file: &MatchedFile, range: &MatchRange, key: &str) -> FieldValue {
    let now = chrono::Utc::now();
    let document = CapturedDocument {
        interval: CaptureInterval {
            started_at: now,
            completed_at: now,
        },
        source: file.source.clone(),
        text: file.text[range.start..range.end].into(),
    };
    match crate::document_evidence::markdown_frontmatter::select_frontmatter(&document, key) {
        Ok(SelectionContent::FrontmatterField { result }) => match *result {
            FrontmatterFieldContent::Present {
                value: StructuredJson::String(value),
                mut span,
                ..
            } => {
                span.byte_start += range.start;
                span.byte_end += range.start;
                let span = source_span(
                    file,
                    &MatchRange {
                        start: span.byte_start,
                        end: span.byte_end,
                    },
                );
                FieldValue::Text {
                    value,
                    spans: vec![span],
                }
            }
            FrontmatterFieldContent::Present { .. } => FieldValue::Unavailable {
                reason: FieldUnavailable::NotAString,
            },
            FrontmatterFieldContent::Absent { .. } => FieldValue::Absent,
            FrontmatterFieldContent::BlockUnavailable { .. } => FieldValue::Unavailable {
                reason: FieldUnavailable::FrontmatterMalformed,
            },
        },
        _ => FieldValue::Unavailable {
            reason: FieldUnavailable::FrontmatterMalformed,
        },
    }
}

fn neighbour_field(
    definition: &EvidenceKindDefinition,
    file: &MatchedFile,
    match_position: usize,
    rule: &crate::EvidenceRuleId,
    count: crate::NeighbourCount,
    previous: bool,
) -> FieldValue {
    let entry = file
        .index
        .match_nodes
        .get(&definition.rule)
        .and_then(|nodes| nodes.get(match_position))
        .and_then(|node| file.index.parents.get(node));
    let mut selected = Vec::new();
    if let Some(entry) = entry {
        let positions: Box<dyn Iterator<Item = usize>> = if previous {
            Box::new((0..entry.position).rev())
        } else {
            Box::new(entry.position + 1..entry.children.len())
        };
        for position in positions {
            let child = &entry.children[position];
            if child.continuation {
                continue;
            }
            if !file
                .index
                .match_nodes
                .get(rule)
                .is_some_and(|nodes| nodes.contains(&child.node))
            {
                break;
            }
            selected.push(child.range.clone());
            if selected.len() == count.get() {
                break;
            }
        }
    }
    selected.sort_by_key(|range| (range.start, range.end));
    FieldValue::Texts {
        values: selected
            .iter()
            .map(|range| file.text[range.start..range.end].into())
            .collect(),
        spans: selected
            .iter()
            .map(|range| source_span(file, range))
            .collect(),
    }
}

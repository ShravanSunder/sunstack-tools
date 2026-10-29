use crate::{
    CaptureInterval, CapturedDocument, CheckedFileSource, CheckedReadError, EvidenceKindDefinition,
    EvidenceKindName, EvidencePiece, EvidenceProject, EvidenceTargetResolution, FieldName,
    FieldSource, FieldUnavailable, FieldValue, FileMatchIndex, FrontmatterFieldContent, LoadTarget,
    MatchRange, PieceLocation, RepositoryId, SelectionContent, SourceRef, SourceSpan,
    StructuredJson, content_digest, match_within, project_model_view,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceFileFailure {
    pub file: PathBuf,
    pub reason: String,
}
#[derive(Debug, Clone, Default)]
pub struct EvidenceCatalog {
    pub pieces: Vec<EvidencePiece>,
    pub matches: BTreeMap<PathBuf, FileMatchIndex>,
    pub failures: Vec<EvidenceFileFailure>,
}
#[derive(Debug, Clone, Copy)]
pub struct PieceBuildLimits {
    pub max_file_bytes: usize,
    pub max_total_bytes: usize,
    pub max_matches: usize,
    pub evidence_max_chars: usize,
}

struct MatchedFile {
    text: String,
    index: FileMatchIndex,
    source: SourceRef,
}

/// Acquire and match files once, then compose fields from the immutable match indexes.
pub fn build_pieces(
    project: &EvidenceProject,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    files: &[PathBuf],
    source: &mut dyn CheckedFileSource,
    limits: PieceBuildLimits,
) -> EvidenceCatalog {
    let mut catalog = EvidenceCatalog::default();
    let mut matched = BTreeMap::new();
    let mut paths = files.to_vec();
    paths.sort();
    paths.dedup();
    let mut total_bytes = 0usize;
    for path in paths {
        let result = source.read_file(&path, limits.max_file_bytes);
        match result {
            Err(error) => catalog.failures.push(EvidenceFileFailure {
                file: path,
                reason: error.to_string(),
            }),
            Ok(text) => {
                total_bytes = total_bytes.saturating_add(text.len());
                if total_bytes > limits.max_total_bytes {
                    catalog.failures.push(EvidenceFileFailure {
                        file: path,
                        reason: "max_total_bytes".into(),
                    });
                    continue;
                }
                let index = project.match_file(&path, &text, limits.max_matches);
                if index.limited {
                    catalog.failures.push(EvidenceFileFailure {
                        file: path.clone(),
                        reason: "max_matches".into(),
                    });
                }
                let Ok(repository) = RepositoryId::new("checked_repository") else {
                    continue;
                };
                let source_ref = SourceRef::Document {
                    repository,
                    path: path.clone(),
                    content_identity: content_digest(text.as_bytes()),
                };
                catalog.matches.insert(path.clone(), index.clone());
                matched.insert(
                    path,
                    MatchedFile {
                        text,
                        index,
                        source: source_ref,
                    },
                );
            }
        }
    }
    // Prebuild all target-free fields. Admission guarantees as_kind only refers to these kinds.
    let mut base_fields = BTreeMap::new();
    for (kind, definition) in kinds {
        for (path, file) in &matched {
            for range in file
                .index
                .by_rule
                .get(&definition.rule)
                .into_iter()
                .flatten()
            {
                let fields = local_fields(definition, file, range);
                base_fields.insert((kind.clone(), path.clone(), range.start), fields);
            }
        }
    }
    let mut openings = BTreeMap::<PathBuf, Result<String, CheckedReadError>>::new();
    for (kind, definition) in kinds {
        for (path, file) in &matched {
            for range in file
                .index
                .by_rule
                .get(&definition.rule)
                .into_iter()
                .flatten()
            {
                let mut fields = base_fields
                    .get(&(kind.clone(), path.clone(), range.start))
                    .cloned()
                    .unwrap_or_default();
                for (field, field_source) in &definition.fields {
                    if let FieldSource::Target {
                        regex,
                        resolve,
                        opening_chars,
                        as_kind,
                    } = field_source
                    {
                        let targets = Regex::new(regex)
                            .ok()
                            .into_iter()
                            .flat_map(|pattern| {
                                pattern
                                    .captures_iter(&file.text[range.start..range.end])
                                    .filter_map(|capture| {
                                        capture
                                            .iter()
                                            .skip(1)
                                            .flatten()
                                            .find(|value| !value.as_str().is_empty())
                                            .map(|value| value.as_str().to_owned())
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .map(|name| {
                                let use_path = match resolve {
                                    EvidenceTargetResolution::Path => true,
                                    EvidenceTargetResolution::Skill => false,
                                    EvidenceTargetResolution::PathOrSkill => name.ends_with(".md"),
                                };
                                let candidates = if use_path {
                                    vec![]
                                } else {
                                    matched
                                        .keys()
                                        .filter(|candidate| {
                                            candidate.file_name().is_some_and(|n| n == "SKILL.md")
                                                && candidate
                                                    .parent()
                                                    .and_then(Path::file_name)
                                                    .is_some_and(|n| n == name.as_str())
                                        })
                                        .cloned()
                                        .collect::<Vec<_>>()
                                };
                                let target_path = if use_path {
                                    path.parent().unwrap_or(Path::new("")).join(&name)
                                } else {
                                    candidates
                                        .first()
                                        .cloned()
                                        .unwrap_or_else(|| PathBuf::from(&name).join("SKILL.md"))
                                };
                                let mut target = LoadTarget {
                                    name,
                                    path: target_path.clone(),
                                    exists: false,
                                    candidates: vec![],
                                    opening: None,
                                    fields: None,
                                };
                                if !use_path && candidates.len() != 1 {
                                    target.candidates = candidates;
                                    return Ok(target);
                                }
                                let Some(resolved) = source.resolve_file(&target_path) else {
                                    return Ok(target);
                                };
                                target.path = resolved.clone();
                                if use_path {
                                    let content = if let Some(file) = matched.get(&resolved) {
                                        Ok(file.text.clone())
                                    } else {
                                        openings
                                            .entry(resolved.clone())
                                            .or_insert_with(|| {
                                                source.read_file(&resolved, limits.max_file_bytes)
                                            })
                                            .clone()
                                    };
                                    match content {
                                        Ok(text) => {
                                            target.exists = true;
                                            target.opening =
                                                Some(text.chars().take(*opening_chars).collect());
                                        }
                                        Err(_) => return Err(FieldUnavailable::Unreadable),
                                    }
                                } else {
                                    target.exists = true;
                                    if let Some(as_kind) = as_kind {
                                        target.fields = base_fields.iter().find_map(
                                            |((candidate_kind, candidate_file, _), fields)| {
                                                (candidate_kind == as_kind
                                                    && candidate_file == &resolved)
                                                    .then(|| fields.clone())
                                            },
                                        );
                                    }
                                }
                                Ok(target)
                            })
                            .collect::<Result<Vec<_>, FieldUnavailable>>();
                        fields.insert(
                            field.clone(),
                            match targets {
                                Ok(values) => FieldValue::Targets { values },
                                Err(reason) => FieldValue::Unavailable { reason },
                            },
                        );
                    }
                }
                let span = source_span(file, range);
                let location = PieceLocation {
                    file: path.clone(),
                    first_line: span.start_line,
                    last_line: file.text.as_bytes()[..range.end.saturating_sub(1).max(range.start)]
                        .iter()
                        .filter(|byte| **byte == b'\n')
                        .count()
                        + 1,
                    span,
                };
                let max_chars = definition
                    .max_chars
                    .map_or(limits.evidence_max_chars, |n| n.get() as usize);
                let model_view = project_model_view(&fields, max_chars);
                catalog.pieces.push(EvidencePiece {
                    kind: kind.clone(),
                    location,
                    text: file.text[range.start..range.end].into(),
                    fields,
                    model_view,
                });
            }
        }
    }
    catalog
}

fn source_span(file: &MatchedFile, range: &MatchRange) -> SourceSpan {
    crate::source_capture::source_span(&file.source, &file.text, range.start..range.end)
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

fn local_fields(
    definition: &EvidenceKindDefinition,
    file: &MatchedFile,
    range: &MatchRange,
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
            FieldSource::Inside { rule }
            | FieldSource::Enclosing { rule }
            | FieldSource::SameFile { rule } => {
                let mut selected = file
                    .index
                    .by_rule
                    .get(rule)
                    .into_iter()
                    .flatten()
                    .filter(|candidate| match source {
                        FieldSource::Inside { .. } => match_within(candidate, range),
                        FieldSource::Enclosing { .. } => match_within(range, candidate),
                        _ => true,
                    })
                    .cloned()
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
            FieldSource::Yaml { key } => yaml_field(file, range, key),
            FieldSource::Remainder {} | FieldSource::Target { .. } => continue,
        };
        fields.insert(name.clone(), value);
    }
    taken.sort_by_key(|span| (span.start, span.end));
    let mut cursor = range.start;
    let mut remainder = String::new();
    let mut spans = Vec::new();
    for taken in taken {
        if taken.start > cursor {
            remainder.push_str(&file.text[cursor..taken.start]);
            spans.push(source_span(
                file,
                &MatchRange {
                    start: cursor,
                    end: taken.start,
                },
            ));
        }
        cursor = cursor.max(taken.end);
    }
    if cursor < range.end {
        remainder.push_str(&file.text[cursor..range.end]);
        spans.push(source_span(
            file,
            &MatchRange {
                start: cursor,
                end: range.end,
            },
        ));
    }
    for (name, source) in &definition.fields {
        if matches!(source, FieldSource::Remainder {}) {
            fields.insert(
                name.clone(),
                FieldValue::Text {
                    value: remainder.clone(),
                    spans: spans.clone(),
                },
            );
        }
    }
    fields
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
    match crate::markdown_frontmatter::select_frontmatter(&document, key) {
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

//! Primitive selectors over captured bytes, shared by native seeds and change-set evidence.
use crate::*;
use std::{collections::BTreeMap, path::Path, sync::Arc};

#[derive(Clone, Debug)]
pub struct SourceUnitSelection {
    pub units: Vec<SelectedSourceUnit>,
    pub cutoffs: Vec<DiscoveryCutoff>,
    pub matches_considered: usize,
    pub files_parsed: usize,
    pub bytes_scanned: usize,
}
#[derive(Clone, Debug)]
pub struct ChangeSetSelection {
    pub content: SelectionContent,
    pub cutoffs: Vec<DiscoveryCutoff>,
    pub matches_considered: usize,
    pub files_parsed: usize,
    pub bytes_scanned: usize,
}

pub(crate) fn primitive_selector(
    selector: &EvidenceSelector,
) -> EvidenceResult<(&RepositoryId, Vec<QueryPath>, EvidenceSelection)> {
    match selector {
        EvidenceSelector::Paragraph {
            repository_id,
            source_path,
            target_id,
        } => Ok((
            repository_id,
            vec![QueryPath::Exact {
                path: source_path.clone(),
            }],
            EvidenceSelection::Paragraph {
                target_id: target_id.clone(),
            },
        )),
        EvidenceSelector::FileText {
            repository_id,
            source_path,
        } => Ok((
            repository_id,
            vec![QueryPath::Exact {
                path: source_path.clone(),
            }],
            EvidenceSelection::WholeDocument {},
        )),
        EvidenceSelector::LiteralText {
            repository_id,
            paths,
            text,
        } => Ok((
            repository_id,
            paths.clone(),
            EvidenceSelection::LiteralText { text: text.clone() },
        )),
        EvidenceSelector::RegexText {
            repository_id,
            paths,
            pattern,
        } => Ok((
            repository_id,
            paths.clone(),
            EvidenceSelection::RegexText {
                pattern: pattern.clone(),
            },
        )),
        EvidenceSelector::AstPattern {
            repository_id,
            paths,
            language,
            pattern,
        } => Ok((
            repository_id,
            paths.clone(),
            EvidenceSelection::AstPattern {
                language: language.clone(),
                pattern: pattern.clone(),
            },
        )),
        EvidenceSelector::RelatedUnits { .. } => Err(EvidenceCode::InvalidSelector.into()),
    }
}
pub(crate) fn matches_paths(paths: &[QueryPath], path: &Path) -> EvidenceResult<bool> {
    for query in paths {
        match query {
            QueryPath::Exact { path: exact }
                if exact
                    .components()
                    .filter(|component| !matches!(component, std::path::Component::CurDir))
                    .eq(path.components().filter(|component| {
                        !matches!(component, std::path::Component::CurDir)
                    })) =>
            {
                return Ok(true);
            }
            QueryPath::Glob { pattern } => {
                let matcher = globset::GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .build()
                    .map_err(|_| EvidenceCode::InvalidSelector)?
                    .compile_matcher();
                if matcher.is_match(path) {
                    return Ok(true);
                }
            }
            QueryPath::Exact { .. } => {}
        }
    }
    Ok(false)
}
pub(crate) fn select_frozen_file(
    file: &FrozenRepositoryFile,
    selection: &EvidenceSelection,
    max_matches: usize,
) -> EvidenceResult<(SelectionContent, bool)> {
    let now = chrono::Utc::now();
    crate::document_selection::select_document_bounded(
        &CapturedDocument {
            interval: CaptureInterval {
                started_at: now,
                completed_at: now,
            },
            source: file.source.clone(),
            text: Arc::from(file.text.as_str()),
        },
        selection,
        max_matches,
    )
}
pub fn select_source_units(
    captures: &[CapturedChangeSelection],
    query: &SourceUnitQuery,
    max_matches: usize,
) -> EvidenceResult<SourceUnitSelection> {
    let mut selected = BTreeMap::new();
    let mut cutoffs = Vec::new();
    let mut matched = 0usize;
    let mut files_parsed = 0;
    let mut bytes_scanned = 0;
    match &query.seeds {
        SourceUnitSeeds::ChangedUnits { repository_ids } => {
            if repository_ids.is_empty() {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            for repository in repository_ids {
                let capture = captures
                    .iter()
                    .find(|capture| &capture.repository == repository)
                    .ok_or(EvidenceCode::UnknownRepository)?;
                let mapping = crate::source_units::map_changed_source_units(
                    capture,
                    &query.languages,
                    &query.unit_kinds,
                    query.source_sides,
                )?;
                files_parsed += mapping.files_parsed;
                bytes_scanned += mapping.bytes_scanned;
                for unit in mapping.units {
                    selected.insert(unit.unit.key.clone(), unit);
                }
            }
        }
        SourceUnitSeeds::SelectorUnits { selectors } => {
            if selectors.is_empty() {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            'selectors: for selector in selectors {
                let (repository, paths, selection) = primitive_selector(selector)?;
                let capture = captures
                    .iter()
                    .find(|capture| &capture.repository == repository)
                    .ok_or(EvidenceCode::UnknownRepository)?;
                let mut located_before = false;
                let mut located_after = false;
                for file in &capture.sources {
                    let SourceRef::RepositoryFile { path, .. } = &file.source else {
                        continue;
                    };
                    if !matches_paths(&paths, path)?
                        || !crate::source_units::source_language(path)
                            .is_some_and(|language| query.languages.contains(&language))
                    {
                        continue;
                    }
                    if (query.source_sides == SourceSides::AfterOnly
                        && file.side == SourceSide::Before)
                        || (query.source_sides == SourceSides::BeforeOnly
                            && file.side == SourceSide::After)
                    {
                        continue;
                    }
                    files_parsed += 1;
                    bytes_scanned += file.text.len();
                    let (content, mut limited) = match select_frozen_file(
                        file,
                        &selection,
                        max_matches.saturating_sub(matched),
                    ) {
                        Err(error) if error.code == EvidenceCode::MissingTarget => continue,
                        other => other?,
                    };
                    let mut spans = match content {
                        SelectionContent::DocumentText { span, .. }
                        | SelectionContent::Paragraph { span, .. } => vec![span],
                        SelectionContent::Matches { matches } => {
                            matches.into_iter().map(|found| found.span).collect()
                        }
                        _ => return Err(EvidenceCode::InvalidSelector.into()),
                    };
                    if spans.len() > max_matches.saturating_sub(matched) {
                        limited = true;
                        spans.truncate(max_matches.saturating_sub(matched));
                    }
                    matched += spans.len();
                    for unit in extract_source_units(file, &capture.snapshot)? {
                        if query
                            .unit_kinds
                            .contains(&crate::source_units::unit_kind_filter(&unit.unit.kind))
                            && spans.iter().any(|span| {
                                span.byte_start < unit.unit.key.byte_end
                                    && unit.unit.key.byte_start < span.byte_end
                            })
                        {
                            located_before |= file.side == SourceSide::Before;
                            located_after |= file.side == SourceSide::After;
                            selected.insert(unit.unit.key.clone(), unit);
                        }
                    }
                    if limited {
                        cutoffs.push(DiscoveryCutoff {
                            limit: DiscoveryLimit::Matches,
                            frontier: selected.keys().cloned().collect(),
                        });
                        break 'selectors;
                    }
                }
                if query.source_sides == SourceSides::PairedRequired
                    && (!located_before || !located_after)
                {
                    return Err(EvidenceCode::RequiredGroupIncomplete.into());
                }
            }
        }
    }
    Ok(SourceUnitSelection {
        units: selected.into_values().collect(),
        cutoffs,
        matches_considered: matched,
        files_parsed,
        bytes_scanned,
    })
}

pub fn select_change_set(
    capture: &CapturedChangeSelection,
    selection: &EvidenceSelection,
    max_matches: usize,
) -> EvidenceResult<ChangeSetSelection> {
    if let EvidenceSelection::ChangedUnits {
        languages,
        unit_kinds,
        source_sides,
    } = selection
    {
        let mapping = crate::source_units::map_changed_source_units(
            capture,
            languages,
            unit_kinds,
            *source_sides,
        )?;
        return Ok(ChangeSetSelection {
            content: SelectionContent::ChangedUnits {
                units: mapping.units,
            },
            cutoffs: vec![],
            matches_considered: 0,
            files_parsed: mapping.files_parsed,
            bytes_scanned: mapping.bytes_scanned,
        });
    }
    let (paths, sides, primitive) = match selection {
        EvidenceSelection::ChangeSetLiteralText { paths, text, sides } => (
            paths,
            sides,
            EvidenceSelection::LiteralText { text: text.clone() },
        ),
        EvidenceSelection::ChangeSetRegexText {
            paths,
            pattern,
            sides,
        } => (
            paths,
            sides,
            EvidenceSelection::RegexText {
                pattern: pattern.clone(),
            },
        ),
        EvidenceSelection::ChangeSetAstPattern {
            paths,
            language,
            pattern,
            sides,
        } => (
            paths,
            sides,
            EvidenceSelection::AstPattern {
                language: language.clone(),
                pattern: pattern.clone(),
            },
        ),
        _ => return Err(EvidenceCode::InvalidSelector.into()),
    };
    if sides.is_empty() {
        return Err(EvidenceCode::InvalidSelector.into());
    }
    // A Full union member supplies context, not extra delta matches.
    let mut files: Vec<(&FrozenRepositoryFile, Vec<&ChangeSpan>)> = Vec::new();
    for member in &capture.members {
        for change in &member.changes {
            for (side, spans) in [
                (&change.before, &change.before_spans),
                (&change.after, &change.after_spans),
            ] {
                if let CapturedFileSide::Present { file } = side {
                    if let Some((_, accumulated)) =
                        files.iter_mut().find(|(known, _)| *known == file)
                    {
                        accumulated.extend(spans);
                    } else {
                        files.push((file, spans.iter().collect()));
                    }
                }
            }
        }
    }
    let mut all_matches = Vec::new();
    let mut cutoffs = Vec::new();
    let mut files_parsed = 0;
    let mut bytes_scanned = 0;
    let mut matches_considered = 0;
    for (file, spans) in files {
        let (whole_side, changed_side) = match file.side {
            SourceSide::Before => (
                sides.contains(&MatchSide::BeforeFile),
                sides.contains(&MatchSide::Removed),
            ),
            SourceSide::After => (
                sides.contains(&MatchSide::AfterFile),
                sides.contains(&MatchSide::Added),
            ),
        };
        if !whole_side && !changed_side {
            continue;
        }
        let SourceRef::RepositoryFile { path, .. } = &file.source else {
            continue;
        };
        if !matches_paths(paths, path)? {
            continue;
        }
        files_parsed += 1;
        bytes_scanned += file.text.len();
        let (content, limited) = select_frozen_file(
            file,
            &primitive,
            max_matches.saturating_sub(matches_considered),
        )?;
        if let SelectionContent::Matches { matches } = content {
            matches_considered += matches.len();
            all_matches.extend(matches.into_iter().filter(|found| {
                whole_side
                    || spans.iter().any(|span| {
                        span.byte_start < span.byte_end
                            && span.byte_start < found.span.byte_end
                            && found.span.byte_start < span.byte_end
                    })
            }));
        }
        if limited {
            cutoffs.push(DiscoveryCutoff {
                limit: DiscoveryLimit::Matches,
                frontier: vec![],
            });
            break;
        }
    }
    Ok(ChangeSetSelection {
        content: SelectionContent::Matches {
            matches: all_matches,
        },
        cutoffs,
        matches_considered,
        files_parsed,
        bytes_scanned,
    })
}

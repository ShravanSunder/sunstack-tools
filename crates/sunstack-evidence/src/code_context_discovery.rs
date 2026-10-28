use crate::*;
use ast_grep_core::Pattern;
use ast_grep_language::{LanguageExt, SupportLang};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    time::Instant,
};

impl CandidateSource for CodeContextIndex {
    type DiscoveredMatch = DiscoveredCodeMatch;
    fn discover(
        &self,
        pass: Option<&MechanicalPass>,
        samples: usize,
    ) -> EvidenceResult<Discovery<Self::DiscoveredMatch>> {
        if matches!(self.input, SourceInput::Diff { .. })
            && pass.and_then(|pass| pass.source_sides) == Some(SourceSides::PairedRequired)
            && let Err(error) = changed_source_units(
                self.capture.as_ref(),
                &[
                    SourceLanguage::Rust,
                    SourceLanguage::Typescript,
                    SourceLanguage::Tsx,
                    SourceLanguage::Markdown,
                ],
                &[
                    UnitKindFilter::Function,
                    UnitKindFilter::Method,
                    UnitKindFilter::Class,
                    UnitKindFilter::Section,
                    UnitKindFilter::Paragraph,
                ],
                SourceSides::PairedRequired,
            )
        {
            return Ok(Discovery {
                completed: false,
                coverage: Some(DiscoveryCoverage::Unavailable {
                    diagnostics: error.diagnostics(),
                }),
                ..Discovery::default()
            });
        }
        let mut result = Discovery::default();
        let mut dropped = Vec::new();
        let eligible = self.eligible_units();
        let touched = self.touched_units();
        let started = Instant::now();
        'files: for file in &self.capture.sources {
            if self.cancellation.is_cancelled()
                || started.elapsed().as_millis() >= self.limits.max_local_time_ms.get() as u128
            {
                result.completed = false;
                break;
            }
            let SourceRef::RepositoryFile { path, .. } = &file.source else {
                continue;
            };
            if let SourceInput::Code { paths, .. } = &self.input {
                let filter = RepositoryBinding {
                    key: self.repository.clone(),
                    root: std::path::PathBuf::new(),
                    input_paths: paths.clone(),
                    change_scope: ChangeScope::Full {},
                };
                if !path_is_allowed(&filter, path)? {
                    continue;
                }
            } else if !self.units.iter().any(|unit| {
                eligible.contains(&unit.unit.key) && unit.unit.span.source == file.source
            }) && !self
                .changes
                .iter()
                .flat_map(|hunk| [hunk.before.as_ref(), hunk.after.as_ref()])
                .flatten()
                .any(|change| change.span.source == file.source)
            {
                continue;
            }
            let ranges = match pass {
                Some(pass) => predicate_ranges(&pass.predicate, file, usize::MAX)?,
                None => std::iter::once(0..file.text.len()).collect(),
            };
            let mut kept = Vec::new();
            for range in ranges {
                let span = source_span(&file.source, &file.text, range.clone());
                let unit = self
                    .containing_unit(&span)
                    .map(|unit| unit.unit.key.clone());
                let Some((part, hunk)) =
                    self.classify(&span, unit.as_ref(), pass, &eligible, &touched)
                else {
                    continue;
                };
                if result.matches.len() >= self.limits.max_matches.get() {
                    result.completed = false;
                    break 'files;
                }
                let occurrence = match self.input {
                    SourceInput::Diff { .. } => SourceOccurrence::ContextMatch {
                        input_id: self.input_id.clone(),
                        part,
                        unit,
                        span,
                        hunk,
                    },
                    _ => SourceOccurrence::CodeMatch {
                        input_id: self.input_id.clone(),
                        unit,
                        span,
                    },
                };
                let context = self.candidate_context(&occurrence)?;
                kept.push(range);
                result.matches.push(DiscoveredCodeMatch {
                    occurrence,
                    context,
                });
            }
            for unit in self
                .units
                .iter()
                .filter(|unit| unit.unit.span.source == file.source)
            {
                if kept.iter().any(|range| {
                    unit.unit.key.byte_start <= range.start && range.end <= unit.unit.key.byte_end
                }) {
                    continue;
                }
                let span = unit.unit.span.clone();
                let key = Some(unit.unit.key.clone());
                if let Some((part, hunk)) =
                    self.classify(&span, key.as_ref(), pass, &eligible, &touched)
                {
                    let occurrence = match self.input {
                        SourceInput::Diff { .. } => SourceOccurrence::ContextMatch {
                            input_id: self.input_id.clone(),
                            part,
                            unit: key,
                            span,
                            hunk,
                        },
                        _ => SourceOccurrence::CodeMatch {
                            input_id: self.input_id.clone(),
                            unit: key,
                            span,
                        },
                    };
                    dropped.push(DiscoveredCodeMatch {
                        context: self.candidate_context(&occurrence)?,
                        occurrence,
                    });
                }
            }
        }
        // Exact occurrence identity collapses duplicate views, never two spans in one function.
        let mut unique = BTreeMap::new();
        for item in result.matches {
            unique
                .entry(item.occurrence.occurrence_identity()?.as_str().to_owned())
                .or_insert(item);
        }
        result.matches = unique.into_values().collect();
        let count = samples.min(dropped.len());
        for position in 0..count {
            result
                .rejected_sample
                .push(dropped[position * dropped.len() / count].clone());
        }
        result.coverage = Some(self.coverage.clone());
        if !result.completed {
            let (query_bounds, mut cutoffs) = match &self.coverage {
                DiscoveryCoverage::CompleteWithinDeclaredScope { query_bounds } => {
                    (query_bounds.clone(), vec![])
                }
                DiscoveryCoverage::Limited {
                    query_bounds,
                    cutoffs,
                } => (query_bounds.clone(), cutoffs.clone()),
                DiscoveryCoverage::Unavailable { .. } => return Ok(result),
            };
            cutoffs.push(DiscoveryCutoff {
                limit: if self.cancellation.is_cancelled()
                    || started.elapsed().as_millis() >= self.limits.max_local_time_ms.get() as u128
                {
                    DiscoveryLimit::LocalTime
                } else {
                    DiscoveryLimit::Matches
                },
                frontier: vec![],
            });
            let coverage = DiscoveryCoverage::Limited {
                query_bounds,
                cutoffs,
            };
            for item in &mut result.matches {
                item.context.coverage = coverage.clone();
            }
            result.coverage = Some(coverage);
        }
        Ok(result)
    }
}
impl CodeContextIndex {
    fn classify(
        &self,
        span: &SourceSpan,
        unit: Option<&SourceUnitKey>,
        pass: Option<&MechanicalPass>,
        eligible: &BTreeSet<SourceUnitKey>,
        touched: &BTreeSet<SourceUnitKey>,
    ) -> Option<(ContextPart, Option<HunkId>)> {
        if !matches!(self.input, SourceInput::Diff { .. }) {
            return Some((ContextPart::TouchedUnit, None));
        }
        let before = matches!(
            &span.source,
            SourceRef::RepositoryFile {
                revision: SourceRevision::Commit {
                    side: SourceSide::Before,
                    ..
                } | SourceRevision::Index {
                    side: SourceSide::Before,
                    ..
                } | SourceRevision::Worktree {
                    side: SourceSide::Before,
                    ..
                },
                ..
            }
        );
        let changed = self.changes.iter().find(|hunk| {
            [hunk.before.as_ref(), hunk.after.as_ref()]
                .into_iter()
                .flatten()
                .any(|change| crate::code_context_view::same_source_overlap(&change.span, span))
        });
        match pass.and_then(|pass| pass.source_sides) {
            Some(SourceSides::AfterOnly) if before => return None,
            Some(SourceSides::BeforeOnly) if !before => return None,
            None if before && changed.is_none() => return None,
            _ => {}
        }
        let part = if changed.is_some() {
            ContextPart::Change
        } else if self.units.iter().any(|parent| {
            touched.contains(&parent.unit.key)
                && parent.unit.span.source == span.source
                && parent.unit.key.byte_start <= span.byte_start
                && span.byte_end <= parent.unit.key.byte_end
        }) {
            ContextPart::TouchedUnit
        } else if unit.is_some_and(|unit| eligible.contains(unit)) {
            ContextPart::Related
        } else {
            return None;
        };
        if pass
            .and_then(|pass| pass.context_parts.as_ref())
            .is_some_and(|parts| !parts.contains(&part))
        {
            return None;
        }
        Some((part, changed.map(|hunk| hunk.hunk_id.clone())))
    }
}
fn predicate_ranges(
    predicate: &MechanicalPredicate,
    file: &FrozenRepositoryFile,
    limit: usize,
) -> EvidenceResult<Vec<Range<usize>>> {
    match predicate {
        MechanicalPredicate::Ast { language, pattern } => {
            let SourceRef::RepositoryFile { path, .. } = &file.source else {
                return Err(EvidenceCode::InvalidSelector.into());
            };
            let expected = match language {
                CodePatternLanguage::Rust => SourceLanguage::Rust,
                CodePatternLanguage::Typescript => SourceLanguage::Typescript,
                CodePatternLanguage::Tsx => SourceLanguage::Tsx,
            };
            if crate::source_units::source_language(path) != Some(expected) {
                return Ok(vec![]);
            }
            let parser: SupportLang = language
                .as_str()
                .parse()
                .map_err(|_| EvidenceCode::UnsupportedLanguage)?;
            let pattern =
                Pattern::try_new(pattern, parser).map_err(|_| EvidenceCode::InvalidSelector)?;
            let tree = parser.ast_grep(&file.text);
            Ok(tree
                .root()
                .find_all(pattern)
                .take(limit)
                .map(|found| found.range())
                .collect())
        }
        MechanicalPredicate::Regex { field, pattern } if field.as_str() == "/text" => {
            Ok(regex::Regex::new(pattern)
                .map_err(|_| EvidenceCode::InvalidSelector)?
                .find_iter(&file.text)
                .take(limit)
                .map(|matched| matched.range())
                .collect())
        }
        MechanicalPredicate::Literal { field, text } if field.as_str() == "/text" => {
            if text.is_empty() {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            Ok(file
                .text
                .match_indices(text)
                .take(limit)
                .map(|(start, matched)| start..start + matched.len())
                .collect())
        }
        MechanicalPredicate::Any { predicates } | MechanicalPredicate::All { predicates } => {
            if predicates.is_empty() {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            let require_all = matches!(predicate, MechanicalPredicate::All { .. });
            let mut combined: Option<BTreeSet<(usize, usize)>> = None;
            for predicate in predicates {
                let found = predicate_ranges(predicate, file, limit)?
                    .into_iter()
                    .map(|range| (range.start, range.end))
                    .collect::<BTreeSet<_>>();
                combined = Some(match combined {
                    None => found,
                    Some(previous) if require_all => {
                        previous.intersection(&found).copied().collect()
                    }
                    Some(previous) => previous.union(&found).copied().collect(),
                });
            }
            Ok(combined
                .unwrap_or_default()
                .into_iter()
                .map(|(start, end)| start..end)
                .collect())
        }
        _ => Ok(vec![]),
    }
}

impl MechanicalPass {
    pub fn validate(&self) -> EvidenceResult<()> {
        fn validate(predicate: &MechanicalPredicate, path: &mut Vec<String>) -> EvidenceResult<()> {
            let invalid = || EvidenceError {
                code: EvidenceCode::InvalidSelector,
                field_path: path.clone(),
            };
            match predicate {
                MechanicalPredicate::Ast { language, pattern } => {
                    let language: SupportLang = language.as_str().parse().map_err(|_| invalid())?;
                    Pattern::try_new(pattern, language).map_err(|_| invalid())?;
                }
                MechanicalPredicate::Regex { pattern, .. } => {
                    regex::Regex::new(pattern).map_err(|_| invalid())?;
                }
                MechanicalPredicate::Literal { text, .. } if text.is_empty() => {
                    return Err(invalid());
                }
                MechanicalPredicate::Any { predicates }
                | MechanicalPredicate::All { predicates } => {
                    if predicates.is_empty() {
                        return Err(invalid());
                    }
                    for (position, predicate) in predicates.iter().enumerate() {
                        path.extend(["predicates".into(), position.to_string()]);
                        validate(predicate, path)?;
                        path.pop();
                        path.pop();
                    }
                }
                _ => {}
            }
            Ok(())
        }
        validate(
            &self.predicate,
            &mut vec!["mechanical_pass".into(), "predicate".into()],
        )
    }
}

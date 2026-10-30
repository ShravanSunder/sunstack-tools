use crate::{
    BuildStep, CandidateId, CandidateKey, CaptureInterval, CapturedDocument, CheckedFileSource,
    CheckedReadError, ChosenCandidates, EvidenceKindDefinition, EvidenceKindName, EvidenceKindPlan,
    EvidencePiece, EvidenceProject, EvidenceTargetResolution, FieldName, FieldSource,
    FieldUnavailable, FieldValue, FileCount, FileMatchIndex, FrontmatterFieldContent, KindCount,
    LoadTarget, MatchRange, MatchSource, PieceLocation, RepositoryId, SelectionCandidate,
    SelectionContent, SelectionCount, SharedView, SourceRef, SourceSpan, StructuredJson,
    content_digest, match_within, project_model_view,
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
    pub file_counts: Vec<FileCount>,
}
#[derive(Debug, Clone, Copy)]
pub struct PieceBuildLimits {
    pub max_files: usize,
    pub max_file_bytes: usize,
    pub max_total_bytes: usize,
    pub max_matches: usize,
    pub evidence_max_chars: usize,
}
#[derive(Clone)]
struct MatchedFile {
    text: String,
    index: FileMatchIndex,
    source: SourceRef,
}
/// Charge each resolved content acquisition once, including target openings and frozen text.
fn read_cached_content(
    source: &mut dyn CheckedFileSource,
    contents: &mut BTreeMap<PathBuf, Result<String, CheckedReadError>>,
    total_bytes: &mut usize,
    path: &Path,
    limits: PieceBuildLimits,
) -> Result<String, CheckedReadError> {
    let path = source
        .resolve_file(path)
        .ok_or(CheckedReadError::NotContained)?;
    if let Some(content) = contents.get(&path) {
        return content.clone();
    }
    let remaining = limits.max_total_bytes.saturating_sub(*total_bytes);
    let content = if source.checked_files_acquired() >= limits.max_files {
        Err(CheckedReadError::FileCountExceeded)
    } else {
        match source.read_file(&path, limits.max_file_bytes.min(remaining)) {
            Ok(text) => {
                *total_bytes = total_bytes.saturating_add(text.len());
                Ok(text)
            }
            Err(CheckedReadError::TooLarge) if remaining < limits.max_file_bytes => {
                Err(CheckedReadError::TotalBytesExceeded)
            }
            Err(error) => Err(error),
        }
    };
    contents.insert(path, content.clone());
    content
}

pub struct EvidenceBuild {
    kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    plan: crate::EvidenceKindPlan,
    position: usize,
    matched: BTreeMap<PathBuf, MatchedFile>,
    contents: BTreeMap<PathBuf, Result<String, CheckedReadError>>,
    total_bytes: usize,
    limits: PieceBuildLimits,
    catalog: EvidenceCatalog,
    pending: Option<(EvidenceKindName, Vec<EvidencePiece>)>,
}
impl EvidenceBuild {
    pub fn start(
        project: &EvidenceProject,
        kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
        files: &[PathBuf],
        source: &mut dyn CheckedFileSource,
        limits: PieceBuildLimits,
    ) -> Result<Self, Vec<crate::KindPlanRefusal>> {
        let plan = EvidenceKindPlan::derive(kinds)?;
        let mut build = Self {
            kinds: kinds.clone(),
            plan,
            position: 0,
            matched: BTreeMap::new(),
            contents: BTreeMap::new(),
            total_bytes: 0,
            limits,
            catalog: EvidenceCatalog::default(),
            pending: None,
        };
        let mut paths = files.to_vec();
        paths.sort();
        paths.dedup();
        let mut remaining = limits.max_matches;
        for path in paths {
            match read_cached_content(
                source,
                &mut build.contents,
                &mut build.total_bytes,
                &path,
                limits,
            ) {
                Err(error) => build.catalog.failures.push(EvidenceFileFailure {
                    file: path,
                    reason: error.to_string(),
                }),
                Ok(text) => {
                    let index = project.match_file(&path, &text, remaining);
                    remaining = remaining
                        .saturating_sub(index.by_rule.values().map(Vec::len).sum::<usize>());
                    if index.limited {
                        build.catalog.failures.push(EvidenceFileFailure {
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
                    let counts = kinds
                        .iter()
                        .map(|(name, kind)| {
                            (
                                name.clone(),
                                KindCount {
                                    matches: index.by_rule.get(&kind.rule).map_or(0, Vec::len),
                                    limited: index.limited,
                                    selection: kind
                                        .select
                                        .as_ref()
                                        .map(|_| SelectionCount::default()),
                                    ..KindCount::default()
                                },
                            )
                        })
                        .collect();
                    build.catalog.file_counts.push(FileCount {
                        file: path.clone(),
                        kinds: counts,
                    });
                    build.catalog.matches.insert(path.clone(), index.clone());
                    build.matched.insert(
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
        Ok(build)
    }

    /// Resolve target identities from scanned matches without opening their content.
    pub fn target_paths(
        &self,
        source: &dyn CheckedFileSource,
        repository_root: &Path,
    ) -> BTreeMap<PathBuf, Vec<PathBuf>> {
        let mut targets = BTreeMap::<PathBuf, Vec<PathBuf>>::new();
        for (path, file) in &self.matched {
            for definition in self.kinds.values() {
                for range in file
                    .index
                    .by_rule
                    .get(&definition.rule)
                    .into_iter()
                    .flatten()
                {
                    for field in definition.fields.values() {
                        let FieldSource::Target { regex, resolve, .. } = field else {
                            continue;
                        };
                        let Ok(pattern) = Regex::new(regex) else {
                            continue;
                        };
                        for capture in pattern.captures_iter(&file.text[range.start..range.end]) {
                            let Some(name) = capture
                                .iter()
                                .skip(1)
                                .flatten()
                                .find(|value| !value.as_str().is_empty())
                                .map(|value| value.as_str())
                            else {
                                continue;
                            };
                            let use_path = match resolve {
                                EvidenceTargetResolution::Path => true,
                                EvidenceTargetResolution::Skill => false,
                                EvidenceTargetResolution::PathOrSkill => name.ends_with(".md"),
                            };
                            let paths = if use_path {
                                vec![path.parent().unwrap_or(Path::new("")).join(name)]
                            } else {
                                self.matched
                                    .keys()
                                    .filter(|candidate| {
                                        candidate.file_name().is_some_and(|part| part == "SKILL.md")
                                            && candidate
                                                .parent()
                                                .and_then(Path::file_name)
                                                .is_some_and(|part| part == name)
                                    })
                                    .cloned()
                                    .collect()
                            };
                            for target in paths {
                                if let Some(resolved) = source.resolve_file(&target).or_else(|| {
                                    crate::checked_file_source::contained_target_identity(
                                        repository_root,
                                        &target,
                                    )
                                }) {
                                    targets.entry(path.clone()).or_default().push(resolved);
                                }
                            }
                        }
                    }
                }
            }
        }
        targets
    }
    pub fn advance(
        &mut self,
        chosen: ChosenCandidates,
        source: &mut dyn CheckedFileSource,
    ) -> BuildStep {
        if let Some((kind, pieces)) = self.pending.take() {
            let selected = pieces
                .into_iter()
                .filter(|piece| {
                    chosen.0.contains(&CandidateId {
                        kind: kind.clone(),
                        file: piece.location.file.clone(),
                        span: piece.location.span.clone(),
                    })
                })
                .collect();
            self.finalize(&kind, selected, source);
        }
        while let Some(kind) = self.plan.order.get(self.position).cloned() {
            self.position += 1;
            let definition = &self.kinds[&kind];
            let mut pieces = Vec::new();
            for (path, file) in &self.matched {
                for range in file
                    .index
                    .by_rule
                    .get(&definition.rule)
                    .into_iter()
                    .flatten()
                {
                    if let Some(parent) = &definition.within
                        && !self.catalog.pieces.iter().any(|piece| {
                            &piece.kind == parent
                                && piece.location.file == *path
                                && match_within(
                                    range,
                                    &MatchRange {
                                        start: piece.location.span.byte_start,
                                        end: piece.location.span.byte_end,
                                    },
                                )
                        })
                    {
                        continue;
                    }
                    let fields = local_fields(definition, file, range, &self.catalog.pieces);
                    let span = source_span(file, range);
                    let location = PieceLocation {
                        file: path.clone(),
                        first_line: span.start_line,
                        last_line: file.text.as_bytes()
                            [..range.end.saturating_sub(1).max(range.start)]
                            .iter()
                            .filter(|b| **b == b'\n')
                            .count()
                            + 1,
                        span,
                    };
                    pieces.push(EvidencePiece {
                        kind: kind.clone(),
                        location,
                        text: file.text[range.start..range.end].into(),
                        model_view: project_model_view(
                            &fields,
                            definition
                                .max_chars
                                .map_or(self.limits.evidence_max_chars, |n| n.get() as usize),
                        ),
                        fields,
                    });
                }
            }
            if let Some(selection) = &definition.select {
                // AST nodes may share a byte range; E13 identifies the candidate
                // by kind, file and range, before keys, counts and pending pieces.
                let mut identities = std::collections::BTreeSet::new();
                pieces.retain(|piece| {
                    identities.insert(CandidateId {
                        kind: kind.clone(),
                        file: piece.location.file.clone(),
                        span: piece.location.span.clone(),
                    })
                });
                let max_chars = definition
                    .max_chars
                    .map_or(self.limits.evidence_max_chars, |n| n.get() as usize);
                let mut shared = Vec::new();
                let mut candidates = Vec::new();
                let mut per_file = BTreeMap::<PathBuf, usize>::new();
                for piece in &pieces {
                    let position = per_file.entry(piece.location.file.clone()).or_default();
                    if *position == 0 {
                        let fields = piece
                            .fields
                            .iter()
                            .filter(|(name, _)| selection.shared.contains(name))
                            .map(|(name, value)| (name.clone(), value.clone()))
                            .collect();
                        shared.push(SharedView {
                            file: piece.location.file.clone(),
                            kind: kind.clone(),
                            view: crate::selection_model_view(&fields, max_chars),
                            fields,
                        });
                    }
                    let fields = piece
                        .fields
                        .iter()
                        .filter(|(name, _)| {
                            !selection.shared.contains(name)
                                && !matches!(
                                    definition.fields.get(*name),
                                    Some(FieldSource::Target { .. })
                                )
                        })
                        .map(|(name, value)| (name.clone(), value.clone()))
                        .collect();
                    candidates.push(SelectionCandidate {
                        id: CandidateId {
                            kind: kind.clone(),
                            file: piece.location.file.clone(),
                            span: piece.location.span.clone(),
                        },
                        key: CandidateKey::at(*position),
                        location: piece.location.clone(),
                        first_line: piece.text.lines().next().unwrap_or("").into(),
                        selection_view: crate::selection_model_view(&fields, max_chars),
                        fields,
                    });
                    *position += 1;
                }
                for count in &mut self.catalog.file_counts {
                    if let Some(selection) = count
                        .kinds
                        .get_mut(&kind)
                        .and_then(|count| count.selection.as_mut())
                    {
                        selection.candidates = per_file.get(&count.file).copied().unwrap_or(0);
                    }
                }
                self.pending = Some((kind.clone(), pieces));
                return BuildStep::NeedsSelection {
                    kind,
                    candidates,
                    shared,
                };
            }
            self.finalize(&kind, pieces, source);
        }
        let mut catalog = self.catalog.clone();
        catalog.pieces.sort_by(|left, right| {
            (
                &left.kind,
                &left.location.file,
                left.location.span.byte_start,
            )
                .cmp(&(
                    &right.kind,
                    &right.location.file,
                    right.location.span.byte_start,
                ))
        });
        for (file, content) in &self.contents {
            if let Err(
                error
                @ (CheckedReadError::TotalBytesExceeded | CheckedReadError::FileCountExceeded),
            ) = content
                && !catalog.failures.iter().any(|failure| failure.file == *file)
            {
                catalog.failures.push(EvidenceFileFailure {
                    file: file.clone(),
                    reason: error.to_string(),
                });
            }
        }
        BuildStep::Done(catalog)
    }
    fn finalize(
        &mut self,
        kind: &EvidenceKindName,
        mut pieces: Vec<EvidencePiece>,
        source: &mut dyn CheckedFileSource,
    ) {
        let definition = &self.kinds[kind];
        for piece in &mut pieces {
            for (field, field_source) in &definition.fields {
                let FieldSource::Target {
                    regex,
                    resolve,
                    opening_chars,
                    as_kind,
                } = field_source
                else {
                    continue;
                };
                let values = Regex::new(regex)
                    .ok()
                    .into_iter()
                    .flat_map(|pattern| {
                        pattern
                            .captures_iter(&piece.text)
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
                            self.matched
                                .keys()
                                .filter(|candidate| {
                                    candidate.file_name().is_some_and(|name| name == "SKILL.md")
                                        && candidate
                                            .parent()
                                            .and_then(Path::file_name)
                                            .is_some_and(|part| part == name.as_str())
                                })
                                .cloned()
                                .collect::<Vec<_>>()
                        };
                        let path = if use_path {
                            piece
                                .location
                                .file
                                .parent()
                                .unwrap_or(Path::new(""))
                                .join(&name)
                        } else {
                            candidates
                                .first()
                                .cloned()
                                .unwrap_or_else(|| PathBuf::from(&name).join("SKILL.md"))
                        };
                        let mut target = LoadTarget {
                            name,
                            path: path.clone(),
                            exists: false,
                            candidates: vec![],
                            opening: None,
                            fields: None,
                        };
                        if !use_path && candidates.len() != 1 {
                            target.candidates = candidates;
                            return Ok(target);
                        }
                        let Some(resolved) = source.resolve_file(&path) else {
                            return Ok(target);
                        };
                        target.path = resolved.clone();
                        if use_path {
                            let content = if let Some(file) = self.matched.get(&resolved) {
                                Ok(file.text.clone())
                            } else {
                                read_cached_content(
                                    source,
                                    &mut self.contents,
                                    &mut self.total_bytes,
                                    &resolved,
                                    self.limits,
                                )
                            };
                            let text = content.map_err(|_| FieldUnavailable::Unreadable)?;
                            target.exists = true;
                            target.opening = Some(text.chars().take(*opening_chars).collect());
                        } else {
                            target.exists = true;
                            if let Some(as_kind) = as_kind {
                                target.fields = self
                                    .catalog
                                    .pieces
                                    .iter()
                                    .find(|candidate| {
                                        candidate.kind == *as_kind
                                            && candidate.location.file == resolved
                                    })
                                    .map(|candidate| candidate.fields.clone());
                            }
                        }
                        Ok(target)
                    })
                    .collect::<Result<Vec<_>, FieldUnavailable>>();
                piece.fields.insert(
                    field.clone(),
                    match values {
                        Ok(values) => FieldValue::Targets { values },
                        Err(reason) => FieldValue::Unavailable { reason },
                    },
                );
            }
            piece.model_view = project_model_view(
                &piece.fields,
                definition
                    .max_chars
                    .map_or(self.limits.evidence_max_chars, |n| n.get() as usize),
            );
        }
        for count in &mut self.catalog.file_counts {
            if let Some(kind_count) = count.kinds.get_mut(kind) {
                kind_count.pieces = pieces
                    .iter()
                    .filter(|piece| piece.location.file == count.file)
                    .count();
            }
        }
        self.catalog.pieces.extend(pieces);
    }
}
pub fn build_pieces(
    project: &EvidenceProject,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    files: &[PathBuf],
    source: &mut dyn CheckedFileSource,
    limits: PieceBuildLimits,
) -> EvidenceCatalog {
    match EvidenceBuild::start(project, kinds, files, source, limits) {
        Ok(mut build) => loop {
            match build.advance(ChosenCandidates::default(), source) {
                BuildStep::Done(catalog) => return catalog,
                BuildStep::NeedsSelection { .. } => {}
            }
        },
        Err(errors) => EvidenceCatalog {
            failures: errors
                .into_iter()
                .map(|error| EvidenceFileFailure {
                    file: PathBuf::from("sunstack.lint.json"),
                    reason: error.to_string(),
                })
                .collect(),
            ..EvidenceCatalog::default()
        },
    }
}
fn source_ranges(
    of: &MatchSource,
    file: &MatchedFile,
    pieces: &[EvidencePiece],
) -> Vec<MatchRange> {
    match of {
        MatchSource::Rule { rule } => file.index.by_rule.get(rule).cloned().unwrap_or_default(),
        MatchSource::Kind { kind } => pieces
            .iter()
            .filter(|piece| piece.kind == *kind && piece.location.span.source == file.source)
            .map(|piece| MatchRange {
                start: piece.location.span.byte_start,
                end: piece.location.span.byte_end,
            })
            .collect(),
    }
}

mod field_composition;
use field_composition::{local_fields, source_span};

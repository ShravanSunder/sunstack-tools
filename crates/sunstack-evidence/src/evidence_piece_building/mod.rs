use crate::{
    BuildStep, CandidateId, CandidateKey, CaptureInterval, CapturedDocument, CheckedFileSource,
    CheckedReadError, ChosenCandidates, EvidenceKindDefinition, EvidenceKindName, EvidenceKindPlan,
    EvidencePiece, EvidenceProject, FieldName, FieldSource, FieldUnavailable, FieldValue,
    FileCount, FileMatchIndex, FrontmatterFieldContent, KindCount, LoadTarget, MatchRange,
    MatchSource, PieceLocation, RepositoryId, SelectionCandidate, SelectionContent, SelectionCount,
    SharedView, SourceRef, SourceSpan, StructuredJson, content_digest, evidence_kind_language,
    match_within,
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
    pub piece_text_hashes: BTreeMap<(PathBuf, usize, usize), String>,
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
    native_units: BTreeMap<EvidenceKindName, NativeMatchedKind>,
}
/// Charge each resolved content acquisition once, including target openings and frozen text.
fn read_cached_content(
    source: &mut dyn CheckedFileSource,
    contents: &mut BTreeMap<PathBuf, Result<String, CheckedReadError>>,
    total_bytes: &mut usize,
    path: &Path,
    limits: PieceBuildLimits,
) -> Result<String, CheckedReadError> {
    let path = source.resolve_file(path)?;
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
    project: EvidenceProject,
    kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    plan: crate::EvidenceKindPlan,
    position: usize,
    matched: BTreeMap<PathBuf, MatchedFile>,
    contents: BTreeMap<PathBuf, Result<String, CheckedReadError>>,
    total_bytes: usize,
    limits: PieceBuildLimits,
    catalog: EvidenceCatalog,
    pending: Option<(EvidenceKindName, Vec<EvidencePiece>)>,
    unresolved_selections: std::collections::BTreeSet<CandidateId>,
}
impl EvidenceBuild {
    /// Keep unanswered candidates out of selection resolution while independent work advances.
    pub fn defer_selection(
        &mut self,
        unresolved: &std::collections::BTreeSet<CandidateId>,
    ) -> Vec<crate::DeferredEvidenceKind> {
        let Some((root, pending)) = &mut self.pending else {
            return vec![];
        };
        let root = root.clone();
        self.unresolved_selections
            .extend(unresolved.iter().cloned());
        let files = unresolved
            .iter()
            .map(|candidate| candidate.file.clone())
            .collect::<std::collections::BTreeSet<_>>();
        pending.retain(|piece| {
            !unresolved.contains(&CandidateId {
                kind: root.clone(),
                file: piece.location.file.clone(),
                span: piece.location.span.clone(),
            })
        });
        let mut dependent = std::collections::BTreeSet::from([root]);
        for kind in &self.plan.order {
            if self
                .plan
                .depends_on
                .get(kind)
                .is_some_and(|parents| parents.iter().any(|parent| dependent.contains(parent)))
            {
                dependent.insert(kind.clone());
            }
        }
        dependent
            .into_iter()
            .flat_map(|kind| {
                files
                    .iter()
                    .cloned()
                    .map(move |file| crate::DeferredEvidenceKind {
                        kind: kind.clone(),
                        file,
                    })
            })
            .collect()
    }
    pub fn start(
        project: &EvidenceProject,
        kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
        files: &[PathBuf],
        source: &mut dyn CheckedFileSource,
        limits: PieceBuildLimits,
    ) -> Result<Self, Vec<crate::KindPlanRefusal>> {
        let plan = EvidenceKindPlan::derive(kinds)?;
        let mut build = Self {
            project: project.clone(),
            kinds: kinds.clone(),
            plan,
            position: 0,
            matched: BTreeMap::new(),
            contents: BTreeMap::new(),
            total_bytes: 0,
            limits,
            catalog: EvidenceCatalog::default(),
            pending: None,
            unresolved_selections: Default::default(),
        };
        build.scan_files(files, source);
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
            for (kind, definition) in &self.kinds {
                for range in &file.kind_ranges(kind, definition) {
                    for field in definition.fields.values() {
                        if let FieldSource::Document { path: document, .. } = field {
                            let target = path.parent().unwrap_or(Path::new("")).join(document);
                            if let Some(resolved) = source.resolve_file(&target).ok().or_else(|| crate::source_acquisition::checked_file_source::contained_target_identity(repository_root, &target)) {
                                targets.entry(path.clone()).or_default().push(resolved);
                            }
                            continue;
                        }
                        let FieldSource::Target {
                            regex,
                            resolve,
                            exclude,
                            ..
                        } = field
                        else {
                            continue;
                        };
                        let Ok(pattern) = Regex::new(regex) else {
                            continue;
                        };
                        for name in target_names(&pattern, file, range, exclude.as_ref()) {
                            if name.is_empty() {
                                continue;
                            }
                            let use_path = TargetReference::uses_path(&name, resolve);
                            let paths = if use_path {
                                let syntax = TargetReferenceSyntax::for_language(
                                    evidence_kind_language(&self.project, kind, definition),
                                );
                                let Ok(reference) = TargetReference::parse(&name, syntax) else {
                                    continue;
                                };
                                vec![if reference.file.as_os_str().is_empty() {
                                    path.clone()
                                } else {
                                    path.parent().unwrap_or(Path::new("")).join(reference.file)
                                }]
                            } else {
                                self.matched
                                    .keys()
                                    .filter(|candidate| {
                                        candidate.file_name().is_some_and(|part| part == "SKILL.md")
                                            && candidate
                                                .parent()
                                                .and_then(Path::file_name)
                                                .is_some_and(|part| part == name.as_str())
                                    })
                                    .cloned()
                                    .collect()
                            };
                            for target in paths {
                                if let Some(resolved) = source.resolve_file(&target).ok().or_else(|| {
                                    crate::source_acquisition::checked_file_source::contained_target_identity(
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
            let mut pieces = self.pieces_for_kind(&kind, &self.matched.keys().cloned().collect());
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
                            view: self
                                .project_selection_fields(&kind, definition, &fields, max_chars),
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
                        selection_view: self
                            .project_selection_fields(&kind, definition, &fields, max_chars),
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
        BuildStep::Done(self.finished_catalog())
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

fn own_text_ranges(
    range: &MatchRange,
    exclude: Option<&crate::TargetExclude>,
    index: &FileMatchIndex,
) -> Vec<MatchRange> {
    let mut removed = exclude
        .into_iter()
        .flat_map(|exclude| index.by_rule.get(&exclude.rule))
        .flatten()
        .filter(|candidate| match_within(candidate, range) && *candidate != range)
        .cloned()
        .collect::<Vec<_>>();
    removed.sort_by_key(|span| (span.start, span.end));
    let mut ranges = Vec::new();
    let mut cursor = range.start;
    for span in removed {
        if cursor < span.start {
            ranges.push(MatchRange {
                start: cursor,
                end: span.start,
            });
        }
        cursor = cursor.max(span.end);
    }
    if cursor < range.end {
        ranges.push(MatchRange {
            start: cursor,
            end: range.end,
        });
    }
    ranges
}

fn target_names(
    pattern: &Regex,
    file: &MatchedFile,
    range: &MatchRange,
    exclude: Option<&crate::TargetExclude>,
) -> Vec<String> {
    own_text_ranges(range, exclude, &file.index)
        .into_iter()
        .flat_map(|range| {
            pattern
                .captures_iter(&file.text[range.start..range.end])
                .filter_map(|capture| {
                    capture
                        .iter()
                        .skip(1)
                        .flatten()
                        .find(|value| !value.as_str().is_empty())
                        .or_else(|| capture.iter().skip(1).flatten().next())
                        .map(|value| value.as_str().to_owned())
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

mod field_composition;
mod section_prefix_fields;
mod section_scoped_fields;
use field_composition::{local_fields, source_span};

mod document_fields;
mod target_loading;
mod target_reference;
use crate::document_evidence::html_content::is_html_path;
use target_reference::{TargetReference, TargetReferenceSyntax};

mod context_loading;
mod native_unit_fields;
use native_unit_fields::{NativeMatchedKind, collect_native_units};
mod deferred_fields;

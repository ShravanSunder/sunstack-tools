use super::{Preparation, connected_sources::RepositoryView};
use crate::*;
use std::collections::BTreeSet;

impl Preparation {
    pub(crate) async fn compare_change_sides(
        &mut self,
        subject: ChangeComparisonSubject,
        level: ComparisonLevel,
    ) -> ChangeComparison {
        match self.compare_subject(subject, level).await {
            Ok(comparison) => comparison,
            Err(error) if error.code == EvidenceCode::RevisionUnavailable => {
                ChangeComparison::BeforeUnknown {
                    reason: BeforeUnknownReason::BaselineUnavailable,
                }
            }
            Err(error) => ChangeComparison::Refused {
                diagnostics: error.diagnostics(),
            },
        }
    }
    async fn compare_subject(
        &mut self,
        subject: ChangeComparisonSubject,
        level: ComparisonLevel,
    ) -> EvidenceResult<ChangeComparison> {
        if self.cancellation.is_cancelled() {
            return Err(EvidenceCode::MissingContext.into());
        }
        let (source, declaration) = match subject {
            ChangeComparisonSubject::DefinitionFile { path } => {
                if level != ComparisonLevel::Path || !path.is_absolute() {
                    return Err(EvidenceCode::InvalidSelector.into());
                }
                let bindings = self.input.repositories.clone();
                let located = tokio::task::spawn_blocking(move || -> EvidenceResult<_> {
                    for binding in bindings {
                        if !binding.change_scope.has_change_selection() {
                            continue;
                        }
                        let root = binding
                            .root
                            .canonicalize()
                            .map_err(|_| EvidenceCode::SourceUnavailable)?;
                        if let Ok(relative) = path.strip_prefix(&root)
                            && path_is_allowed(&binding, relative)?
                        {
                            return Ok(Some((binding.key, relative.to_path_buf())));
                        }
                    }
                    Ok(None)
                })
                .await
                .map_err(|_| EvidenceCode::SourceUnavailable)??;
                return match located {
                    Some((repository, path)) => {
                        self.compare_repository_path(&repository, path, None, ComparisonLevel::Path)
                            .await
                    }
                    None => Ok(ChangeComparison::NoBeforeSide {
                        reason: NoBeforeSideReason::OutsideChangeScopePaths,
                    }),
                };
            }
            ChangeComparisonSubject::Declaration { declaration } => {
                crate::evidence_admission::admit_declaration(&declaration, &self.input.sources)?;
                let source = self
                    .input
                    .sources
                    .get(&declaration.input)
                    .cloned()
                    .ok_or(EvidenceCode::InvalidSelector)?;
                (source, Some(declaration))
            }
            ChangeComparisonSubject::Input { input } => {
                let source = self
                    .input
                    .sources
                    .get(&input)
                    .cloned()
                    .ok_or(EvidenceCode::InvalidSelector)?;
                let declaration = EvidenceDeclaration {
                    id: DeclarationId::new("comparison")?,
                    input,
                    requirement: EvidenceRequirement::Required,
                    select: EvidenceSelection::WholeDocument {},
                    max_items: None,
                    max_bytes: None,
                };
                (source, Some(declaration))
            }
        };
        match source {
            SourceSpec::Conversation { .. } => Ok(ChangeComparison::NoBeforeSide {
                reason: NoBeforeSideReason::ConversationInput,
            }),
            SourceSpec::Payload { .. } => Ok(ChangeComparison::NoBeforeSide {
                reason: NoBeforeSideReason::PayloadInput,
            }),
            SourceSpec::Document { repository, path } => {
                self.compare_repository_path(&repository, path, declaration, level)
                    .await
            }
            SourceSpec::ChangeSet { repository } => {
                self.compare_change_set(&repository, declaration, level)
                    .await
            }
        }
    }
    async fn comparison_capture(
        &mut self,
        repository: &RepositoryId,
    ) -> EvidenceResult<Result<std::sync::Arc<CapturedChangeSelection>, ChangeComparison>> {
        let binding = self
            .input
            .repositories
            .iter()
            .find(|binding| &binding.key == repository)
            .ok_or(EvidenceCode::UnknownRepository)?;
        if !binding.change_scope.has_change_selection() {
            return Ok(Err(ChangeComparison::NoBeforeSide {
                reason: NoBeforeSideReason::FullScope,
            }));
        }
        let capture = self
            .repository_capture(repository, RepositoryView::ChangeSelection)
            .await?;
        match &capture.coverage {
            DiscoveryCoverage::Limited { cutoffs, .. }
                if cutoffs
                    .iter()
                    .all(|cutoff| cutoff.limit == DiscoveryLimit::HistoryCommits) =>
            {
                return Ok(Err(ChangeComparison::BeforeUnknown {
                    reason: BeforeUnknownReason::HistoryIncomplete,
                }));
            }
            DiscoveryCoverage::Limited { .. } => return Err(EvidenceCode::DiscoveryLimit.into()),
            DiscoveryCoverage::Unavailable { diagnostics } => {
                return Ok(Err(ChangeComparison::Refused {
                    diagnostics: diagnostics.clone(),
                }));
            }
            DiscoveryCoverage::CompleteWithinDeclaredScope { .. } => {}
        }
        if capture
            .diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == EvidenceCode::HistoryIncomplete)
        {
            return Ok(Err(ChangeComparison::BeforeUnknown {
                reason: BeforeUnknownReason::HistoryIncomplete,
            }));
        }
        Ok(Ok(capture))
    }
    async fn compare_repository_path(
        &mut self,
        repository: &RepositoryId,
        path: std::path::PathBuf,
        declaration: Option<EvidenceDeclaration>,
        level: ComparisonLevel,
    ) -> EvidenceResult<ChangeComparison> {
        let capture = match self.comparison_capture(repository).await? {
            Ok(capture) => capture,
            Err(comparison) => return Ok(comparison),
        };
        let mut differences = ComparisonDifferences::default();
        for member in &capture.members {
            for change in member.changes.iter().filter(|change| change.path == path) {
                if level == ComparisonLevel::Path {
                    differences.record(
                        member.scope_position,
                        side_present(&change.before),
                        side_present(&change.after),
                    );
                } else {
                    let declaration = declaration.as_ref().ok_or(EvidenceCode::InvalidSelector)?;
                    let before = self
                        .compare_selected_content(&change.before, declaration)
                        .await?;
                    let after = self
                        .compare_selected_content(&change.after, declaration)
                        .await?;
                    if before != after {
                        differences.record(
                            member.scope_position,
                            before.is_some(),
                            after.is_some(),
                        );
                    }
                }
            }
        }
        Ok(differences.finish(level))
    }
    async fn compare_selected_content(
        &mut self,
        side: &CapturedFileSide,
        declaration: &EvidenceDeclaration,
    ) -> EvidenceResult<Option<Vec<String>>> {
        let CapturedFileSide::Present { file } = side else {
            return Ok(None);
        };
        let limits = self.discovery_limits()?;
        let remaining = limits
            .max_matches
            .get()
            .saturating_sub(self.discovery_matches);
        let file = file.clone();
        let declaration = declaration.clone();
        let started = std::time::Instant::now();
        let result = tokio::task::spawn_blocking(move || {
            selected_fingerprint(&file, &declaration, remaining)
        })
        .await
        .map_err(|_| EvidenceCode::InvalidSelector)??;
        self.discovery_elapsed += started.elapsed();
        self.discovery_matches += result.matches_considered;
        self.measurements.files_parsed += 1;
        self.measurements.bytes_scanned += result.bytes_scanned;
        if self.discovery_elapsed.as_millis() >= limits.max_local_time_ms.get() as u128 {
            return Err(EvidenceCode::DiscoveryLimit.into());
        }
        Ok(result.content)
    }
    async fn compare_change_set(
        &mut self,
        repository: &RepositoryId,
        declaration: Option<EvidenceDeclaration>,
        level: ComparisonLevel,
    ) -> EvidenceResult<ChangeComparison> {
        let capture = match self.comparison_capture(repository).await? {
            Ok(capture) => capture,
            Err(comparison) => return Ok(comparison),
        };
        let mut differences = ComparisonDifferences::default();
        for member in &capture.members {
            for change in &member.changes {
                if let Some(declaration) = &declaration {
                    let paths = match &declaration.select {
                        EvidenceSelection::ChangeSetLiteralText { paths, .. }
                        | EvidenceSelection::ChangeSetRegexText { paths, .. }
                        | EvidenceSelection::ChangeSetAstPattern { paths, .. } => Some(paths),
                        _ => None,
                    };
                    if let Some(paths) = paths
                        && !crate::frozen_queries::matches_paths(paths, &change.path)?
                    {
                        continue;
                    }
                }
                differences.record(
                    member.scope_position,
                    side_present(&change.before),
                    side_present(&change.after),
                );
            }
        }
        let path_result = differences.finish(level);
        if level == ComparisonLevel::Path || matches!(path_result, ChangeComparison::Unchanged {}) {
            return Ok(path_result);
        }
        if let Some(declaration) = declaration {
            if matches!(declaration.select, EvidenceSelection::WholeDocument {}) {
                return Ok(path_result);
            }
            if let EvidenceSelection::RelatedUnits {
                languages,
                unit_kinds,
                source_sides,
                ..
            } = &declaration.select
            {
                let languages = languages
                    .iter()
                    .map(|language| match language {
                        GraphLanguage::Rust => SourceLanguage::Rust,
                        GraphLanguage::Typescript => SourceLanguage::Typescript,
                        GraphLanguage::Tsx => SourceLanguage::Tsx,
                    })
                    .collect::<Vec<_>>();
                let capture_for_mapping = capture.clone();
                let unit_kinds = unit_kinds.clone();
                let source_sides = *source_sides;
                let mapping = tokio::task::spawn_blocking(move || {
                    crate::source_units::map_changed_source_units(
                        &capture_for_mapping,
                        &languages,
                        &unit_kinds,
                        source_sides,
                    )
                })
                .await
                .map_err(|_| EvidenceCode::InvalidSelector)??;
                self.measurements.files_parsed += mapping.files_parsed;
                self.measurements.bytes_scanned += mapping.bytes_scanned;
                if mapping.units.is_empty() {
                    return Ok(ChangeComparison::Unchanged {});
                }
            }
            let selected = self.select(&declaration).await?;
            match selected {
                SelectionReadiness::Refused { diagnostics } => {
                    return Ok(ChangeComparison::Refused { diagnostics });
                }
                SelectionReadiness::Ready {
                    content, coverage, ..
                } => {
                    if let SelectionCoverage::ChangeSet {
                        discovery: DiscoveryCoverage::Limited { .. },
                        ..
                    } = coverage.as_ref()
                    {
                        return Err(EvidenceCode::DiscoveryLimit.into());
                    }
                    if !content_nonempty(&content) {
                        return Ok(ChangeComparison::Unchanged {});
                    }
                }
            }
        }
        let members = capture
            .members
            .iter()
            .filter(|member| !member.changes.is_empty())
            .map(|member| member.scope_position)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(ChangeComparison::Changed { members })
    }
}
#[derive(Default)]
struct ComparisonDifferences {
    members: BTreeSet<usize>,
    added: usize,
    deleted: usize,
    modified: usize,
}
impl ComparisonDifferences {
    fn record(&mut self, member: usize, before: bool, after: bool) {
        if !before && !after {
            return;
        }
        self.members.insert(member);
        match (before, after) {
            (false, true) => self.added += 1,
            (true, false) => self.deleted += 1,
            (true, true) => self.modified += 1,
            (false, false) => {}
        }
    }
    fn finish(self, level: ComparisonLevel) -> ChangeComparison {
        if self.members.is_empty() {
            return ChangeComparison::Unchanged {};
        }
        let members = self.members.into_iter().collect();
        if self.modified == 0 && self.deleted == 0 {
            ChangeComparison::Added { members }
        } else if self.modified == 0 && self.added == 0 {
            ChangeComparison::Deleted { members }
        } else if level == ComparisonLevel::Path {
            ChangeComparison::PathChanged { members }
        } else {
            ChangeComparison::Changed { members }
        }
    }
}
fn side_present(side: &CapturedFileSide) -> bool {
    matches!(side, CapturedFileSide::Present { .. })
}
struct SelectionFingerprint {
    content: Option<Vec<String>>,
    matches_considered: usize,
    bytes_scanned: usize,
}
fn selected_fingerprint(
    file: &FrozenRepositoryFile,
    declaration: &EvidenceDeclaration,
    remaining: usize,
) -> EvidenceResult<SelectionFingerprint> {
    let (content, limited) =
        match crate::frozen_queries::select_frozen_file(file, &declaration.select, remaining) {
            Ok(content) => content,
            Err(error) if error.code == EvidenceCode::MissingTarget => {
                return Ok(SelectionFingerprint {
                    content: None,
                    matches_considered: 0,
                    bytes_scanned: file.text.len(),
                });
            }
            Err(error) => return Err(error),
        };
    if limited {
        return Err(EvidenceCode::SelectorLimit.into());
    }
    let serialized_bytes = serde_json::to_vec(&content)
        .map_err(|_| EvidenceCode::SelectorLimit)?
        .len();
    if declaration
        .max_bytes
        .is_some_and(|limit| serialized_bytes > limit.get())
    {
        return Err(EvidenceCode::SelectorLimit.into());
    }
    let (content, matches_considered) = match content {
        SelectionContent::Paragraph { text, .. } | SelectionContent::DocumentText { text, .. } => {
            (Some(vec![text]), 0)
        }
        SelectionContent::Matches { matches } => {
            if declaration
                .max_items
                .is_some_and(|limit| matches.len() > limit.get())
            {
                return Err(EvidenceCode::SelectorLimit.into());
            }
            let count = matches.len();
            let mut values = Vec::new();
            for matched in matches {
                let captures = matched
                    .captures
                    .iter()
                    .map(|(name, span)| {
                        file.text
                            .get(span.byte_start..span.byte_end)
                            .map(|text| (name.clone(), text.to_owned()))
                            .ok_or(EvidenceCode::InvalidSelector)
                    })
                    .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
                values.push(
                    serde_json::to_string(&(matched.text, captures))
                        .map_err(|_| EvidenceCode::InvalidSelector)?,
                );
            }
            values.sort();
            ((!values.is_empty()).then_some(values), count)
        }
        SelectionContent::ConversationRecords { .. }
        | SelectionContent::PayloadValue { .. }
        | SelectionContent::ChangedUnits { .. }
        | SelectionContent::RelatedGroup { .. }
        | SelectionContent::RelatedGroups { .. } => {
            return Err(EvidenceCode::InvalidSelector.into());
        }
    };
    Ok(SelectionFingerprint {
        content,
        matches_considered,
        bytes_scanned: file.text.len(),
    })
}
fn content_nonempty(content: &SelectionContent) -> bool {
    match content {
        SelectionContent::Matches { matches } => !matches.is_empty(),
        SelectionContent::ChangedUnits { units } => !units.is_empty(),
        SelectionContent::RelatedGroup { group } => match group.as_ref() {
            ContextGroupOutcome::Complete { group, .. }
            | ContextGroupOutcome::Partial { group, .. } => !group.units.is_empty(),
            ContextGroupOutcome::Unavailable { .. } => false,
        },
        SelectionContent::RelatedGroups { groups } => !groups.is_empty(),
        SelectionContent::Paragraph { .. }
        | SelectionContent::DocumentText { .. }
        | SelectionContent::ConversationRecords { .. }
        | SelectionContent::PayloadValue { .. } => true,
    }
}

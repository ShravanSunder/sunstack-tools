use crate::*;
use std::path::{Component, Path};

pub(crate) fn admit_connected_plan(input: &EvidencePlanInput) -> EvidenceResult<()> {
    for binding in &input.repositories {
        binding.change_scope.validate()?;
    }
    for source in input.sources.values() {
        if let SourceSpec::ChangeSet { repository } = source {
            repository_binding(input, repository)?;
            if matches!(input.discovery, EvidenceDiscovery::None) {
                return Err(EvidenceCode::UnsupportedRelationshipQuery.into());
            }
        }
    }
    for query in &input.native_queries {
        if let EvidenceSelector::RelatedUnits { query, .. } = &query.selector {
            if matches!(input.discovery, EvidenceDiscovery::None) {
                return Err(EvidenceCode::UnsupportedRelationshipQuery.into());
            }
            if query.repository_ids.is_empty()
                || query.directions.is_empty()
                || query.source_units.languages.is_empty()
                || query.source_units.unit_kinds.is_empty()
            {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            for repository in &query.repository_ids {
                repository_binding(input, repository)?;
            }
            match &query.source_units.seeds {
                SourceUnitSeeds::ChangedUnits { repository_ids } => {
                    if repository_ids.is_empty() {
                        return Err(EvidenceCode::InvalidSelector.into());
                    }
                    for repository in repository_ids {
                        repository_binding(input, repository)?;
                        if !query.repository_ids.contains(repository) {
                            return Err(EvidenceCode::InvalidSelector.into());
                        }
                    }
                }
                SourceUnitSeeds::SelectorUnits { selectors } => {
                    if selectors.is_empty() {
                        return Err(EvidenceCode::InvalidSelector.into());
                    }
                    for selector in selectors {
                        let (repository, paths, selection) =
                            crate::frozen_queries::primitive_selector(selector)?;
                        let binding = repository_binding(input, repository)?;
                        if !query.repository_ids.contains(repository) {
                            return Err(EvidenceCode::InvalidSelector.into());
                        }
                        validate_paths(&paths)?;
                        for path in &paths {
                            if let QueryPath::Exact { path } = path
                                && !path_is_allowed(binding, path)?
                            {
                                return Err(EvidenceCode::PathOutsideRoot.into());
                            }
                        }
                        validate_primitive(&selection)?;
                    }
                }
            }
        }
    }
    for binding in &input.package_bindings {
        repository_binding(input, &binding.importing_repository_id)?;
        repository_binding(input, &binding.target_repository_id)?;
        if binding.module_prefix.trim().is_empty()
            || !safe_relative_path(&binding.target_path_prefix)
            || !matches!(binding.language.as_str(), "rust" | "typescript" | "tsx")
        {
            return Err(EvidenceCode::InvalidRepositoryBinding.into());
        }
    }
    Ok(())
}
fn repository_binding<'input>(
    input: &'input EvidencePlanInput,
    repository: &RepositoryId,
) -> EvidenceResult<&'input RepositoryBinding> {
    input
        .repositories
        .iter()
        .find(|binding| &binding.key == repository)
        .ok_or_else(|| EvidenceCode::UnknownRepository.into())
}
pub(crate) fn admit_connected_selection(selection: &EvidenceSelection) -> EvidenceResult<()> {
    match selection {
        EvidenceSelection::ChangeSetLiteralText { paths, text } => {
            validate_paths(paths)?;
            validate_primitive(&EvidenceSelection::LiteralText { text: text.clone() })?;
        }
        EvidenceSelection::ChangeSetRegexText { paths, pattern } => {
            validate_paths(paths)?;
            validate_primitive(&EvidenceSelection::RegexText {
                pattern: pattern.clone(),
            })?;
        }
        EvidenceSelection::ChangeSetAstPattern {
            paths,
            language,
            pattern,
        } => {
            validate_paths(paths)?;
            validate_primitive(&EvidenceSelection::AstPattern {
                language: language.clone(),
                pattern: pattern.clone(),
            })?;
        }
        EvidenceSelection::ChangedUnits {
            languages,
            unit_kinds,
            ..
        } if languages.is_empty() || unit_kinds.is_empty() => {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        EvidenceSelection::RelatedUnits {
            languages,
            unit_kinds,
            directions,
            chunk_settings,
            ..
        } => {
            if languages.is_empty()
                || unit_kinds.is_empty()
                || directions.is_empty()
                || chunk_settings.grouping != ChunkGrouping::SingleGroup
                || unit_kinds
                    .iter()
                    .any(|kind| matches!(kind, UnitKindFilter::Paragraph | UnitKindFilter::Section))
            {
                return Err(EvidenceCode::InvalidContextGroup.into());
            }
        }
        _ => {}
    }
    Ok(())
}
fn validate_paths(paths: &[QueryPath]) -> EvidenceResult<()> {
    if paths.is_empty() {
        return Err(EvidenceCode::InvalidSelector.into());
    }
    for path in paths {
        match path {
            QueryPath::Exact { path } if !safe_relative_path(path) => {
                return Err(EvidenceCode::PathOutsideRoot.into());
            }
            QueryPath::Glob { pattern } => {
                globset::GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .build()
                    .map_err(|_| EvidenceCode::InvalidSelector)?;
            }
            _ => {}
        }
    }
    Ok(())
}
fn safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
}
fn validate_primitive(selection: &EvidenceSelection) -> EvidenceResult<()> {
    match selection {
        EvidenceSelection::LiteralText { text } if text.is_empty() => {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        EvidenceSelection::RegexText { pattern } => {
            regex::Regex::new(pattern).map_err(|_| EvidenceCode::InvalidSelector)?;
        }
        EvidenceSelection::AstPattern { language, pattern } => {
            let language: ast_grep_language::SupportLang = language
                .parse()
                .map_err(|_| EvidenceCode::UnsupportedLanguage)?;
            ast_grep_core::Pattern::try_new(pattern, language)
                .map_err(|_| EvidenceCode::InvalidSelector)?;
        }
        _ => {}
    }
    Ok(())
}

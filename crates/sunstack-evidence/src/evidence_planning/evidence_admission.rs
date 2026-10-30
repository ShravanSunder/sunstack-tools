use crate::*;
use std::collections::BTreeSet;

pub(crate) fn admission(
    input: &EvidencePlanInput,
    registry: &AdapterRegistry,
) -> EvidenceResult<()> {
    crate::repository_evidence::connected_admission::admit_connected_plan(input)?;
    let mut keys = BTreeSet::new();
    let mut roots = BTreeSet::new();
    for binding in &input.repositories {
        if !keys.insert(&binding.key) || binding.input_paths.is_empty() {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        let root = binding
            .root
            .canonicalize()
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        if !roots.insert(root) {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        for allowed in &binding.input_paths {
            if let QueryPath::Glob { pattern } = allowed {
                globset::GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .build()
                    .map_err(|_| EvidenceCode::InvalidSelector)?;
            }
        }
    }
    for source in input.sources.values() {
        match source {
            SourceSpec::Conversation { format, .. } => {
                registry.adapter(format)?;
            }
            SourceSpec::Document { repository, path } => {
                let binding = input
                    .repositories
                    .iter()
                    .find(|binding| &binding.key == repository)
                    .ok_or(EvidenceCode::InvalidSelector)?;
                if !path_is_allowed(binding, path)? {
                    return Err(EvidenceCode::PathOutsideRoot.into());
                }
            }
            SourceSpec::ChangeSet { .. } => {}
            SourceSpec::Payload { .. } => {}
        }
    }
    let mut ids = BTreeSet::new();
    for declaration in &input.declarations {
        if !ids.insert(&declaration.id) {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        admit_evidence_declaration(declaration, &input.sources)?;
    }
    let mut cursors = BTreeSet::new();
    for cursor in &input.cursors {
        let canonical = cursor
            .transcript_path
            .canonicalize()
            .map_err(|_| EvidenceCode::InvalidCursor)?;
        if !cursors.insert((canonical.clone(),cursor.format.clone())) || !input.sources.values().any(|source|matches!(source,SourceSpec::Conversation{transcript_path,format,..} if format==&cursor.format && transcript_path.canonicalize().ok().as_ref()==Some(&canonical))) {return Err(EvidenceCode::InvalidCursor.into());}
        MonitoringCursor::decode(&cursor.cursor)?;
    }
    for query in &input.native_queries {
        if !ids.insert(&query.id) {
            return Err(EvidenceCode::InvalidSelector.into());
        }
    }
    Ok(())
}
fn valid_pointer(pointer: &str) -> bool {
    if pointer.is_empty() {
        return true;
    }
    if !pointer.starts_with('/') {
        return false;
    }
    let mut chars = pointer.chars();
    while let Some(character) = chars.next() {
        if character == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}

/// Validate a selector against its bound source without acquiring that source.
pub fn admit_evidence_declaration(
    declaration: &EvidenceDeclaration,
    sources: &std::collections::BTreeMap<SourceName, SourceSpec>,
) -> EvidenceResult<()> {
    let source = sources
        .get(&declaration.input)
        .ok_or(EvidenceCode::InvalidSelector)?;
    let compatible = matches!(
        (source, &declaration.select),
        (
            SourceSpec::Document { .. },
            EvidenceSelection::Paragraph { .. }
                | EvidenceSelection::MarkdownUnits(_)
                | EvidenceSelection::FrontmatterField { .. }
                | EvidenceSelection::WholeDocument {}
                | EvidenceSelection::LiteralText { .. }
                | EvidenceSelection::RegexText { .. }
                | EvidenceSelection::AstPattern { .. }
        ) | (
            SourceSpec::Conversation { .. },
            EvidenceSelection::Conversation { .. }
        ) | (
            SourceSpec::ChangeSet { .. },
            EvidenceSelection::ChangeSetLiteralText { .. }
                | EvidenceSelection::ChangeSetRegexText { .. }
                | EvidenceSelection::ChangeSetAstPattern { .. }
                | EvidenceSelection::ChangedUnits { .. }
                | EvidenceSelection::RelatedUnits { .. }
        ) | (
            SourceSpec::Payload { .. },
            EvidenceSelection::WholePayload {} | EvidenceSelection::PayloadField { .. }
        )
    );
    if !compatible {
        return Err(EvidenceCode::InvalidSelector.into());
    }
    crate::repository_evidence::connected_admission::admit_connected_selection(
        &declaration.select,
    )?;
    if let Some(selection) = declaration.select.conversation() {
        selection.validate()?;
    }
    match &declaration.select {
        EvidenceSelection::FrontmatterField { field } if field.is_empty() => {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        EvidenceSelection::MarkdownUnits(selection) => selection.validate()?,
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
        EvidenceSelection::PayloadField { pointer } if !valid_pointer(pointer) => {
            return Err(EvidenceCode::InvalidPointer.into());
        }
        _ => {}
    }
    Ok(())
}

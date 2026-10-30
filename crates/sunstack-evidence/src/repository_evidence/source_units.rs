use crate::*;
use ast_grep_language::{LanguageExt, SupportLang};
use pulldown_cmark::{Event, Parser, Tag};
use std::{collections::BTreeMap, ops::Range, path::Path};

pub fn extract_source_units(
    file: &FrozenRepositoryFile,
    snapshot: &SnapshotId,
) -> EvidenceResult<Vec<SelectedSourceUnit>> {
    let SourceRef::RepositoryFile { path, .. } = &file.source else {
        return Err(EvidenceCode::InvalidSelector.into());
    };
    let Some(language) = source_language(path) else {
        return Ok(Vec::new());
    };
    if language == SourceLanguage::Markdown {
        return markdown_units(file, snapshot);
    }
    let parser: SupportLang = match language {
        SourceLanguage::Rust => "rust",
        SourceLanguage::Typescript => "typescript",
        SourceLanguage::Tsx => "tsx",
        SourceLanguage::Markdown => unreachable!(),
    }
    .parse()
    .map_err(|_| EvidenceCode::UnsupportedLanguage)?;
    let tree = parser.ast_grep(&file.text);
    let mut units = Vec::new();
    for node in tree.root().dfs() {
        let node_kind = node.kind();
        let kind = match node_kind.as_ref() {
            "function_item"
                if node
                    .ancestors()
                    .any(|parent| matches!(parent.kind().as_ref(), "impl_item" | "trait_item")) =>
            {
                UnitKindFilter::Method
            }
            "function_item" | "function_declaration" | "generator_function_declaration" => {
                UnitKindFilter::Function
            }
            "method_definition" | "function_signature_item" => UnitKindFilter::Method,
            "class_declaration" | "abstract_class_declaration" => UnitKindFilter::Class,
            _ => continue,
        };
        let range = node.range();
        let signature_end = node
            .field("body")
            .map_or(range.end, |body| body.range().start);
        let signature = file.text[range.start..signature_end].trim_end().to_string();
        let kind = match kind {
            UnitKindFilter::Function => SourceUnitKind::Function { signature },
            UnitKindFilter::Method => SourceUnitKind::Method { signature },
            UnitKindFilter::Class => SourceUnitKind::Class { signature },
            _ => unreachable!(),
        };
        let quality = if node
            .dfs()
            .any(|descendant| descendant.is_error() || descendant.is_missing())
        {
            SyntaxQuality::RecoveredWithErrors
        } else {
            SyntaxQuality::Parsed
        };
        units.push(make_unit(file, snapshot, range, kind, quality)?);
    }
    Ok(units)
}

pub fn changed_source_units(
    capture: &CapturedChangeSelection,
    languages: &[SourceLanguage],
    unit_kinds: &[UnitKindFilter],
    sides: SourceSides,
) -> EvidenceResult<Vec<SelectedSourceUnit>> {
    Ok(map_changed_source_units(capture, languages, unit_kinds, sides)?.units)
}
pub(crate) struct UnitMapping {
    pub units: Vec<SelectedSourceUnit>,
    pub files_parsed: usize,
    pub bytes_scanned: usize,
}
pub(crate) fn map_changed_source_units(
    capture: &CapturedChangeSelection,
    languages: &[SourceLanguage],
    unit_kinds: &[UnitKindFilter],
    sides: SourceSides,
) -> EvidenceResult<UnitMapping> {
    if !capture.scope.has_change_selection() {
        return Err(EvidenceCode::NoChangeSelection.into());
    }
    if languages.is_empty() || unit_kinds.is_empty() {
        return Err(EvidenceCode::InvalidSelector.into());
    }
    let mut selected = BTreeMap::new();
    let mut files_parsed = 0;
    let mut bytes_scanned = 0;
    for member in &capture.members {
        for change in &member.changes {
            if !source_language(&change.path).is_some_and(|language| languages.contains(&language))
            {
                continue;
            }
            let mut paired = Vec::new();
            for (side, spans) in [
                (&change.before, &change.before_spans),
                (&change.after, &change.after_spans),
            ] {
                let CapturedFileSide::Present { file } = side else {
                    paired.push(Vec::new());
                    continue;
                };
                let units = extract_source_units(file, &capture.snapshot)?
                    .into_iter()
                    .filter(|selected| unit_kinds.contains(&unit_kind_filter(&selected.unit.kind)))
                    .filter(|selected| {
                        spans
                            .iter()
                            .any(|span| intersects_unit(span, &selected.unit.key))
                    })
                    .collect::<Vec<_>>();
                files_parsed += 1;
                bytes_scanned += file.text.len();
                paired.push(units);
            }
            if sides == SourceSides::PairedRequired {
                for (units, other) in [(&paired[0], &paired[1]), (&paired[1], &paired[0])] {
                    if units.iter().any(|unit| {
                        !other
                            .iter()
                            .any(|candidate| candidate.unit.kind == unit.unit.kind)
                    }) {
                        return Err(EvidenceCode::RequiredGroupIncomplete.into());
                    }
                }
            }
            for (index, units) in paired.into_iter().enumerate() {
                if (index == 0 && sides == SourceSides::AfterOnly)
                    || (index == 1 && sides == SourceSides::BeforeOnly)
                {
                    continue;
                }
                for unit in units {
                    selected.insert(unit.unit.key.clone(), unit);
                }
            }
        }
    }
    Ok(UnitMapping {
        units: selected.into_values().collect(),
        files_parsed,
        bytes_scanned,
    })
}

pub(crate) fn source_language(path: &Path) -> Option<SourceLanguage> {
    match path.extension()?.to_str()? {
        "rs" => Some(SourceLanguage::Rust),
        "ts" | "mts" | "cts" => Some(SourceLanguage::Typescript),
        "tsx" => Some(SourceLanguage::Tsx),
        "md" | "markdown" => Some(SourceLanguage::Markdown),
        _ => None,
    }
}
pub(crate) fn unit_kind_filter(kind: &SourceUnitKind) -> UnitKindFilter {
    match kind {
        SourceUnitKind::Function { .. } => UnitKindFilter::Function,
        SourceUnitKind::Method { .. } => UnitKindFilter::Method,
        SourceUnitKind::Class { .. } => UnitKindFilter::Class,
        SourceUnitKind::Paragraph => UnitKindFilter::Paragraph,
        SourceUnitKind::Section => UnitKindFilter::Section,
    }
}
fn intersects_unit(span: &ChangeSpan, unit: &SourceUnitKey) -> bool {
    if span.byte_start == span.byte_end {
        unit.byte_start < span.byte_start && span.byte_start < unit.byte_end
    } else {
        span.byte_start < unit.byte_end && unit.byte_start < span.byte_end
    }
}
fn make_unit(
    file: &FrozenRepositoryFile,
    snapshot: &SnapshotId,
    range: Range<usize>,
    kind: SourceUnitKind,
    syntax_quality: SyntaxQuality,
) -> EvidenceResult<SelectedSourceUnit> {
    let SourceRef::RepositoryFile {
        repository,
        revision,
        path,
        content_identity,
    } = &file.source
    else {
        return Err(EvidenceCode::InvalidSelector.into());
    };
    Ok(SelectedSourceUnit {
        unit: SourceUnit {
            key: SourceUnitKey {
                snapshot: snapshot.clone(),
                repository: repository.clone(),
                revision: revision.clone(),
                path: path.clone(),
                byte_start: range.start,
                byte_end: range.end,
            },
            kind,
            span: source_span(&file.source, &file.text, range.clone()),
            content_identity: content_identity.clone(),
            syntax_quality,
        },
        text: file.text[range].into(),
    })
}
fn markdown_units(
    file: &FrozenRepositoryFile,
    snapshot: &SnapshotId,
) -> EvidenceResult<Vec<SelectedSourceUnit>> {
    let mut units = crate::document_evidence::document_selection::paragraph_ranges(&file.text)
        .into_iter()
        .map(|range| {
            make_unit(
                file,
                snapshot,
                range,
                SourceUnitKind::Paragraph,
                SyntaxQuality::Parsed,
            )
        })
        .collect::<EvidenceResult<Vec<_>>>()?;
    let headings = Parser::new(&file.text)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            if let Event::Start(Tag::Heading { level, .. }) = event {
                Some((level, range.start))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    for (index, (level, start)) in headings.iter().enumerate() {
        let end = headings[index + 1..]
            .iter()
            .find(|(next, _)| next <= level)
            .map_or(file.text.len(), |(_, offset)| *offset);
        let end = start + file.text[*start..end].trim_end().len();
        units.push(make_unit(
            file,
            snapshot,
            *start..end,
            SourceUnitKind::Section,
            SyntaxQuality::Parsed,
        )?);
    }
    Ok(units)
}

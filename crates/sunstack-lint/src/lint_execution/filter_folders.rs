use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use sunstack_rules::{FilterName, FilterRegistry, RulebookRule};

#[derive(Debug, Clone, Default)]
pub(crate) struct FilterFolderPlan {
    pub folders: BTreeMap<(FilterName, PathBuf), Vec<PathBuf>>,
    pub dependencies: BTreeMap<PathBuf, Vec<PathBuf>>,
    pub context_files: BTreeSet<PathBuf>,
}

pub(crate) fn resolve_filter_folders(
    filters: &FilterRegistry,
    rules: &[RulebookRule],
    files: &[PathBuf],
) -> FilterFolderPlan {
    let demanded = rules
        .iter()
        .filter_map(|rule| rule.gate.as_ref().map(|gate| &gate.filter))
        .collect::<BTreeSet<_>>();
    let mut result = FilterFolderPlan::default();
    for filter in filters
        .values()
        .filter(|filter| demanded.contains(&filter.name))
    {
        for file in files {
            if let Some((folder, _)) = filter.per.folder_for(file) {
                let context = folder.join(&filter.reads.file);
                result.context_files.insert(context.clone());
                result
                    .dependencies
                    .entry(file.clone())
                    .or_default()
                    .push(context);
                result
                    .folders
                    .entry((filter.name.clone(), folder))
                    .or_default()
                    .push(file.clone());
            }
        }
    }
    result
}

pub(crate) fn bind_reads_pieces(
    plan: &FilterFolderPlan,
    filters: &FilterRegistry,
    rules: &[RulebookRule],
    checked_pieces: &[sunstack_evidence::EvidencePiece],
    catalog: &sunstack_evidence::EvidenceCatalog,
) -> Vec<sunstack_rules::FilterFolder> {
    plan.folders
        .iter()
        .filter_map(|((filter, folder), members)| {
            let definition = filters.get(filter)?;
            let demanded = members
                .iter()
                .filter(|file| {
                    rules.iter().any(|rule| {
                        rule.gate
                            .as_ref()
                            .is_some_and(|gate| &gate.filter == filter)
                            && checked_pieces.iter().any(|piece| {
                                &piece.location.file == *file && piece.kind == rule.kind
                            })
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            if demanded.is_empty() {
                return None;
            }
            let input = folder.join(&definition.reads.file);
            let matches = catalog
                .pieces
                .iter()
                .enumerate()
                .filter(|(_, piece)| {
                    piece.location.file == input && piece.kind == definition.reads.kind
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let reads = if catalog.failures.iter().any(|failure| failure.file == input) {
                sunstack_rules::ReadsPiece::Unavailable(
                    sunstack_evidence::FieldUnavailable::Unreadable,
                )
            } else {
                match matches.as_slice() {
                    [] => sunstack_rules::ReadsPiece::None,
                    [index] => sunstack_rules::ReadsPiece::One(*index),
                    _ => sunstack_rules::ReadsPiece::Ambiguous,
                }
            };
            Some(sunstack_rules::FilterFolder {
                filter: filter.clone(),
                folder: folder.clone(),
                members: demanded,
                reads,
            })
        })
        .collect()
}

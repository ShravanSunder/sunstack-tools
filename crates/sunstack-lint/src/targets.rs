use crate::{config_validation::config_directory, *};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub(crate) struct TargetSelection {
    pub entries: Vec<Vec<PathBuf>>,
    pub ignored: u32,
}
pub(crate) fn relative_path(base: &Path, path: &Path) -> PathBuf {
    let base_parts = base.components().collect::<Vec<_>>();
    let path_parts = path.components().collect::<Vec<_>>();
    let shared = base_parts
        .iter()
        .zip(&path_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in shared..base_parts.len() {
        relative.push("..");
    }
    for part in &path_parts[shared..] {
        relative.push(part.as_os_str());
    }
    relative
}
fn matches(patterns: &[GlobPattern], file: &Path) -> bool {
    patterns.iter().any(|pattern| {
        globset::Glob::new(pattern.as_str()).is_ok_and(|glob| glob.compile_matcher().is_match(file))
    })
}
fn named(file: &Path, paths: &[PathBuf]) -> bool {
    paths.is_empty() || paths.iter().any(|path| file.starts_with(path))
}
pub(crate) fn select_targets(
    config: &LintConfig,
    paths: &[PathBuf],
    cwd: &Path,
) -> Result<TargetSelection, LintInvalid> {
    let directory = config_directory(config).canonicalize().map_err(|_| {
        LintInvalid::at(
            "sunstack.lint.json",
            LintInvalidCode::Syntax,
            "Cannot resolve the config directory.",
        )
    })?;
    let mut named_paths = Vec::new();
    for path in paths {
        let absolute = cwd.join(path);
        named_paths.push(absolute.canonicalize().map_err(|_| {
            LintInvalid::at(
                absolute,
                LintInvalidCode::PathNotFound,
                "A named path does not exist.",
            )
        })?);
    }
    let repository = git2::Repository::discover(&directory).ok();
    let root = repository
        .as_ref()
        .and_then(|repository| repository.workdir())
        .unwrap_or(&directory);
    let entries = config.rule_sets.values().collect::<Vec<_>>();
    let mut selected = vec![BTreeSet::new(); entries.len()];
    let mut ignored = BTreeSet::new();
    for entry in ignore::WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .git_global(false)
        .git_ignore(true)
        .git_exclude(true)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build()
    {
        let entry = entry.map_err(|_| {
            LintInvalid::at(
                root,
                LintInvalidCode::PathNotFound,
                "A target path could not be inspected.",
            )
        })?;
        if !entry.file_type().is_some_and(|kind| kind.is_file())
            || !named(entry.path(), &named_paths)
        {
            continue;
        }
        let relative = relative_path(&directory, entry.path());
        for (index, configured) in entries.iter().enumerate() {
            if matches(&configured.files, &relative) {
                if matches(&config.ignore_patterns, &relative) {
                    ignored.insert((index, entry.path().to_path_buf()));
                } else {
                    selected[index].insert(entry.path().to_path_buf());
                }
            }
        }
    }
    // The walk excludes Git-ignored files. Git's status catalogue supplies their
    // exclusion count without reading their contents or admitting them as targets.
    if let Some(repository) = repository.as_ref() {
        let mut options = git2::StatusOptions::new();
        options
            .include_ignored(true)
            .recurse_ignored_dirs(true)
            .include_untracked(true)
            .recurse_untracked_dirs(true);
        if let (Some(root), Ok(statuses)) = (
            repository.workdir(),
            repository.statuses(Some(&mut options)),
        ) {
            for status in statuses
                .iter()
                .filter(|status| status.status().is_ignored())
            {
                if let Ok(path) = status.path() {
                    let absolute = root.join(path);
                    if !absolute.is_file() || !named(&absolute, &named_paths) {
                        continue;
                    }
                    let relative = relative_path(&directory, &absolute);
                    for (index, configured) in entries.iter().enumerate() {
                        if matches(&configured.files, &relative) {
                            ignored.insert((index, absolute.clone()));
                        }
                    }
                }
            }
        }
    }
    Ok(TargetSelection {
        entries: selected
            .into_iter()
            .map(|files| files.into_iter().collect())
            .collect(),
        ignored: u32::try_from(ignored.len()).unwrap_or(u32::MAX),
    })
}

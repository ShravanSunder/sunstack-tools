use crate::{configuration_loading::config_validation::config_directory, *};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
pub(crate) struct TargetSelection {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
    pub ignored: u32,
}
/// Express an absolute path relative to an absolute base directory.
pub fn relative_path(base: &Path, path: &Path) -> PathBuf {
    fn normalized(path: &Path) -> PathBuf {
        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    result.pop();
                }
                component => result.push(component.as_os_str()),
            }
        }
        result
    }
    let base = normalized(base);
    let path = normalized(path);
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
pub(crate) fn select_targets(
    config: &RulesConfig,
    paths: &[PathBuf],
    cwd: &Path,
) -> Result<TargetSelection, LintInvalid> {
    let directory = config_directory(config).canonicalize().map_err(|_| {
        LintInvalid::at(
            "sunstack.lint.json",
            LintInvalidCode::Syntax,
            "cannot resolve config directory",
        )
    })?;
    let repository = git2::Repository::discover(&directory).ok();
    let root = repository
        .as_ref()
        .and_then(|repo| repo.workdir())
        .unwrap_or(&directory)
        .to_path_buf();
    let paths = paths
        .iter()
        .map(|path| {
            cwd.join(path).canonicalize().map_err(|_| {
                LintInvalid::at(
                    path,
                    LintInvalidCode::PathNotFound,
                    "named path does not exist",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let named = |file: &Path| paths.is_empty() || paths.iter().any(|path| file.starts_with(path));
    let mut files = BTreeSet::new();
    let mut ignored = BTreeSet::new();
    for entry in ignore::WalkBuilder::new(&root)
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
                &root,
                LintInvalidCode::PathNotFound,
                "target path could not be inspected",
            )
        })?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) || !named(entry.path()) {
            continue;
        }
        let file = relative_path(&root, entry.path());
        if matches(&config.check.files, &file) {
            if matches(&config.check.ignore, &file) {
                ignored.insert(file);
            } else {
                files.insert(file);
            }
        }
    }
    if let Some(repository) = repository.as_ref() {
        let mut options = git2::StatusOptions::new();
        options
            .include_ignored(true)
            .recurse_ignored_dirs(true)
            .include_untracked(true)
            .recurse_untracked_dirs(true);
        if let Ok(statuses) = repository.statuses(Some(&mut options)) {
            for status in statuses
                .iter()
                .filter(|status| status.status().is_ignored())
            {
                if let Ok(path) = status.path() {
                    let file = PathBuf::from(path);
                    if root.join(&file).is_file()
                        && named(&root.join(&file))
                        && matches(&config.check.files, &file)
                    {
                        ignored.insert(file);
                    }
                }
            }
        }
    }
    Ok(TargetSelection {
        root,
        files: files.into_iter().collect(),
        ignored: u32::try_from(ignored.len()).unwrap_or(u32::MAX),
    })
}

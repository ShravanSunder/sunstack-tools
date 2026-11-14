use crate::*;
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
/// Select checked files: under an explicit root, or the config directory's Git workdir.
pub(crate) fn select_targets(
    check: &CheckedFiles,
    config_directory: &Path,
    paths: &[PathBuf],
    cwd: &Path,
    checked_root: Option<&Path>,
) -> Result<TargetSelection, LintInvalid> {
    if let Some(root) = checked_root {
        let resolved = cwd.join(root).canonicalize().map_err(|_| {
            LintInvalid::at(
                root,
                LintInvalidCode::PathNotFound,
                "checked root does not exist",
            )
        })?;
        if !resolved.is_dir() {
            return Err(LintInvalid::at(
                root,
                LintInvalidCode::PathNotFound,
                "checked root must be a directory",
            ));
        }
        return select_targets_in_root(check, &resolved, paths, cwd);
    }
    let directory = config_directory.canonicalize().map_err(|_| {
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
    select_targets_in_root(check, &root, paths, cwd)
}

pub(crate) fn select_targets_in_root(
    check: &CheckedFiles,
    root: &Path,
    paths: &[PathBuf],
    cwd: &Path,
) -> Result<TargetSelection, LintInvalid> {
    let repository = git2::Repository::discover(root).ok();
    let paths = paths
        .iter()
        .map(|path| {
            let resolved = cwd.join(path).canonicalize().map_err(|_| {
                LintInvalid::at(
                    path,
                    LintInvalidCode::PathNotFound,
                    format!(
                        "named path {} does not exist (checked root {})",
                        cwd.join(path).display(),
                        root.display()
                    ),
                )
            })?;
            if !resolved.starts_with(root) {
                return Err(LintInvalid::at(
                    path,
                    LintInvalidCode::UnsupportedInputs,
                    format!(
                        "named path {} is outside root {}",
                        resolved.display(),
                        root.display()
                    ),
                ));
            }
            Ok(resolved)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let named = |file: &Path| paths.is_empty() || paths.iter().any(|path| file.starts_with(path));
    let mut files = BTreeSet::new();
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
                "target path could not be inspected",
            )
        })?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) || !named(entry.path()) {
            continue;
        }
        let file = relative_path(root, entry.path());
        if matches(&check.files, &file) {
            if matches(&check.ignore, &file) {
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
                    let Some(workdir) = repository.workdir() else {
                        continue;
                    };
                    let absolute = workdir.join(path);
                    let Ok(file) = absolute.strip_prefix(root) else {
                        continue;
                    };
                    let file = file.to_path_buf();
                    if root.join(&file).is_file()
                        && named(&root.join(&file))
                        && matches(&check.files, &file)
                    {
                        ignored.insert(file);
                    }
                }
            }
        }
    }
    Ok(TargetSelection {
        root: root.to_path_buf(),
        files: files.into_iter().collect(),
        ignored: u32::try_from(ignored.len()).unwrap_or(u32::MAX),
    })
}

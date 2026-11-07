use crate::{CheckedFiles, LintReport, RulebookFile, lint_execution::targets};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
    process::Command,
};
use tokio_util::sync::CancellationToken;

pub(super) struct RevisionSnapshot {
    // Keep the archive and its whole tree alive until all cases at this revision are scored.
    _directory: tempfile::TempDir,
    pub tree: PathBuf,
    pub config: PathBuf,
    pub symlink_paths: BTreeSet<PathBuf>,
    pub sources: BTreeMap<PathBuf, Result<String, String>>,
    pub resolved_revision: String,
}
pub(super) struct RulebookOverlay<'a> {
    pub root: &'a Path,
    pub config: &'a Path,
    pub files: &'a [RulebookFile],
    pub rule_directories: &'a [PathBuf],
    pub check: &'a CheckedFiles,
}
fn checked_command(command: &mut Command, description: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("{description}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{description} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
fn relative_input(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let normalized = path.canonicalize().map_err(|error| error.to_string())?;
    normalized
        .strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(|_| {
            format!(
                "rulebook input {} is outside the current repository",
                path.display()
            )
        })
}
// Inspect the authored path before today's overlay can replace historical links.
fn traverses_symlink(tree: &Path, selected_path: &Path) -> Result<bool, String> {
    let mut path = tree.to_path_buf();
    for component in selected_path.components() {
        match component {
            Component::CurDir => continue,
            Component::Normal(name) => path.push(name),
            _ => return Err("selected path must stay inside the snapshot".into()),
        }
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Ok(true),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(false)
}

fn overlay_destination(tree: &Path, relative: &Path) -> Result<PathBuf, String> {
    let mut directory = tree.to_path_buf();
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            match component {
                Component::CurDir => continue,
                Component::Normal(name) => directory.push(name),
                _ => return Err("rulebook overlay must stay inside the snapshot".into()),
            }
            if std::fs::symlink_metadata(&directory)
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                std::fs::remove_file(&directory).map_err(|error| error.to_string())?;
            }
            if !directory.exists() {
                std::fs::create_dir(&directory).map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(tree.join(relative))
}

fn copy_rule_directory(
    source: &Path,
    destination: &Path,
    current_root: &Path,
    ancestors: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let source = source.canonicalize().map_err(|error| error.to_string())?;
    if !source.starts_with(current_root) {
        return Err("rule directory content is outside the current repository".into());
    }
    if !ancestors.insert(source.clone()) {
        return Err("rule directory contains a symbolic-link cycle".into());
    }
    std::fs::create_dir(destination).map_err(|error| error.to_string())?;
    for entry in std::fs::read_dir(&source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_name() == ".git" {
            return Err("rule directory contains repository metadata".into());
        }
        let path = entry
            .path()
            .canonicalize()
            .map_err(|error| error.to_string())?;
        if !path.starts_with(current_root) {
            return Err("rule directory content is outside the current repository".into());
        }
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_rule_directory(&path, &target, current_root, ancestors)?;
        } else if path.is_file() {
            let mut input = std::fs::File::open(&path).map_err(|error| error.to_string())?;
            let mut output = std::fs::File::create(target).map_err(|error| error.to_string())?;
            std::io::copy(&mut input, &mut output).map_err(|error| error.to_string())?;
        } else {
            return Err("rule directory contains an unsupported file type".into());
        }
    }
    ancestors.remove(&source);
    Ok(())
}

fn replace_rule_directories(
    tree: &Path,
    overlay: &RulebookOverlay<'_>,
    selected_paths: &BTreeSet<PathBuf>,
) -> Result<(), String> {
    if overlay.rule_directories.is_empty() {
        return Ok(());
    }
    let checked = targets::select_targets_in_root(overlay.check, tree, &[], tree)
        .map_err(|error| error.to_string())?;
    let mut directories = Vec::new();
    for directory in overlay.rule_directories {
        let canonical_relative = relative_input(overlay.root, directory)?;
        let relative = targets::relative_path(overlay.root, directory);
        if canonical_relative.as_os_str().is_empty() || relative.as_os_str().is_empty() {
            return Err(
                "rule directory resolves to the repository root; case-run overlay is invalid"
                    .into(),
            );
        }
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err("rule directory must stay inside the current repository".into());
        }
        if relative.components().any(|part| part.as_os_str() == ".git") {
            return Err("rule directory overlaps repository metadata".into());
        }
        let covers =
            |path: &Path| path.starts_with(&relative) || path.starts_with(&canonical_relative);
        if selected_paths.iter().any(|path| covers(path))
            || checked.files.iter().any(|path| {
                covers(path)
                    && !path
                        .extension()
                        .is_some_and(|ext| ext == "yml" || ext == "yaml")
            })
        {
            return Err(format!(
                "rule directory {} covers a checked file or selected case path; case-run overlay is invalid",
                relative.display()
            ));
        }
        directories.push((directory, relative));
    }
    for (source, relative) in directories {
        let destination = overlay_destination(tree, &relative)?;
        match std::fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                std::fs::remove_dir_all(&destination).map_err(|error| error.to_string())?;
            }
            Ok(_) => std::fs::remove_file(&destination).map_err(|error| error.to_string())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        copy_rule_directory(source, &destination, overlay.root, &mut BTreeSet::new())?;
    }
    Ok(())
}

pub(super) fn prepare(
    repository: &Path,
    revision: &str,
    overlay: &RulebookOverlay<'_>,
    case_files: &BTreeSet<PathBuf>,
    selected_paths: &BTreeSet<PathBuf>,
) -> Result<RevisionSnapshot, String> {
    let repository_handle =
        git2::Repository::open(repository).map_err(|error| error.to_string())?;
    let commit = repository_handle
        .revparse_single(revision)
        .and_then(|object| object.peel_to_commit())
        .map_err(|error| format!("cannot resolve revision {revision}: {error}"))?;
    let directory = tempfile::Builder::new()
        .prefix("2026-11-sunstack-cases-")
        .tempdir()
        .map_err(|error| error.to_string())?;
    let archive = directory.path().join("repository.tar");
    let tree = directory.path().join("tree");
    std::fs::create_dir(&tree).map_err(|error| error.to_string())?;
    let tree = tree.canonicalize().map_err(|error| error.to_string())?;
    checked_command(
        Command::new("git")
            .current_dir(repository)
            .args(["archive", "--format=tar", "--output"])
            .arg(&archive)
            .arg(commit.id().to_string()),
        "whole-repository git archive",
    )?;
    checked_command(
        Command::new("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(&tree),
        "whole-repository archive extraction",
    )?;
    checked_command(
        Command::new("git").arg("init").arg("--quiet").arg(&tree),
        "disposable snapshot root metadata",
    )?;
    let symlink_paths = selected_paths
        .iter()
        .filter_map(|path| match traverses_symlink(&tree, path) {
            Ok(true) => Some(Ok(path.clone())),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<BTreeSet<_>, String>>()?;
    let sources = case_files
        .iter()
        .filter(|file| !symlink_paths.contains(*file))
        .map(|file| {
            let source = tree
                .join(file)
                .canonicalize()
                .map_err(|error| error.to_string())
                .and_then(|resolved| {
                    if !resolved.starts_with(&tree) {
                        return Err("case file is outside archived repository".into());
                    }
                    std::fs::read_to_string(resolved).map_err(|error| error.to_string())
                });
            (file.clone(), source)
        })
        .collect();
    replace_rule_directories(&tree, overlay, selected_paths)?;
    for file in overlay.files {
        let destination = overlay_destination(&tree, &relative_input(overlay.root, &file.path)?)?;
        // A historical symlink must not redirect today's overlay outside this snapshot.
        if std::fs::symlink_metadata(&destination)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            std::fs::remove_file(&destination).map_err(|error| error.to_string())?;
        }
        std::fs::write(destination, &file.content).map_err(|error| error.to_string())?;
    }
    let config = tree.join(relative_input(overlay.root, overlay.config)?);
    Ok(RevisionSnapshot {
        _directory: directory,
        tree,
        config,
        sources,
        symlink_paths,
        resolved_revision: commit.id().to_string(),
    })
}
async fn invoke(
    executable: &Path,
    snapshot: &RevisionSnapshot,
    paths: &[PathBuf],
    cancellation: &CancellationToken,
) -> Result<std::process::Output, String> {
    let mut command = tokio::process::Command::new(executable);
    command
        .current_dir(&snapshot.tree)
        .arg("lint")
        .arg("--config")
        .arg(&snapshot.config)
        .args(["--format", "json"])
        .arg("--")
        .args(paths);
    command.kill_on_drop(true);
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err("case run cancelled".into()),
        output = command.output() => output.map_err(|error| error.to_string()),
    }
}
pub(super) async fn lint(
    executable: &Path,
    snapshot: &RevisionSnapshot,
    paths: &[PathBuf],
    cancellation: &CancellationToken,
) -> Result<LintReport, String> {
    if paths.is_empty() {
        return Err("no judgeable selected case paths at this revision".into());
    }
    let output = invoke(executable, snapshot, paths, cancellation).await?;
    serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "normal lint report unavailable: {error}; {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[cfg(test)]
pub(super) fn snapshot_for_matching_tests(
    sources: BTreeMap<PathBuf, Result<String, String>>,
) -> Result<RevisionSnapshot, std::io::Error> {
    let directory = tempfile::tempdir()?;
    Ok(RevisionSnapshot {
        tree: directory.path().to_path_buf(),
        config: directory.path().join("unused-config.json"),
        _directory: directory,
        sources,
        symlink_paths: BTreeSet::new(),
        resolved_revision: "matching-unit-fixture".into(),
    })
}

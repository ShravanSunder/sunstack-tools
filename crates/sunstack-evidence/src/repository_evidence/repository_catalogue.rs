use crate::{repository_evidence::repository_capture::RepositoryCapture, *};
use git2::{ObjectType, Oid, TreeWalkMode, TreeWalkResult};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone)]
pub(crate) enum CaptureView {
    Empty,
    Commit(Oid),
    Index,
    Worktree,
}
pub(super) enum ReadTarget {
    Blob(Oid),
    Worktree,
}
pub(super) enum FileCatalogue {
    Empty,
    Captured {
        revision: SourceRevision,
        entries: BTreeMap<PathBuf, ReadTarget>,
    },
}
impl FileCatalogue {
    pub(super) fn blob_id(&self, path: &std::path::Path) -> Option<Oid> {
        match self {
            Self::Captured { entries, .. } => match entries.get(path) {
                Some(ReadTarget::Blob(object_id)) => Some(*object_id),
                _ => None,
            },
            Self::Empty => None,
        }
    }
    pub(super) fn paths(&self) -> Vec<PathBuf> {
        match self {
            Self::Empty => vec![],
            Self::Captured { entries, .. } => entries.keys().cloned().collect(),
        }
    }
}
impl RepositoryCapture<'_> {
    pub(super) fn catalogue(
        &mut self,
        view: &CaptureView,
        side: SourceSide,
    ) -> EvidenceResult<FileCatalogue> {
        match view {
            CaptureView::Empty => Ok(FileCatalogue::Empty),
            CaptureView::Commit(oid) => {
                let commit = self
                    .repository
                    .find_commit(*oid)
                    .map_err(|_| EvidenceCode::RevisionUnavailable)?;
                let tree = commit
                    .tree()
                    .map_err(|_| EvidenceCode::RevisionUnavailable)?;
                let mut entries = BTreeMap::new();
                let mut failure = None;
                let mut timed_out = false;
                let mut timed_out_path = None;
                let walk = tree.walk(TreeWalkMode::PreOrder, |prefix, entry| {
                    if self.check_cancel().is_err() {
                        failure = Some(EvidenceError::from(EvidenceCode::MissingContext));
                        return TreeWalkResult::Abort;
                    }
                    if self.deadline_exhausted() {
                        timed_out = true;
                        timed_out_path = Some(PathBuf::from(format!(
                            "{prefix}{}",
                            String::from_utf8_lossy(entry.name_bytes())
                        )));
                        return TreeWalkResult::Abort;
                    }
                    if entry.kind() == Some(ObjectType::Blob) {
                        let Ok(name) = entry.name() else {
                            failure = Some(
                                EvidenceError::from(EvidenceCode::UnsupportedEncoding).at_path(
                                    PathBuf::from(format!(
                                        "{prefix}{}",
                                        String::from_utf8_lossy(entry.name_bytes())
                                    )),
                                ),
                            );
                            return TreeWalkResult::Abort;
                        };
                        let path = PathBuf::from(format!("{prefix}{name}"));
                        match path_is_allowed(&self.binding, &path) {
                            Ok(true) => {
                                entries.insert(path, ReadTarget::Blob(entry.id()));
                            }
                            Ok(false) => {}
                            Err(error) => {
                                failure = Some(error.at_path(&path));
                                return TreeWalkResult::Abort;
                            }
                        }
                    }
                    TreeWalkResult::Ok
                });
                if timed_out {
                    self.limited(DiscoveryLimit::LocalTime, timed_out_path.as_deref());
                } else {
                    walk.map_err(|_| {
                        failure.unwrap_or_else(|| EvidenceCode::RevisionUnavailable.into())
                    })?;
                }
                Ok(FileCatalogue::Captured {
                    revision: SourceRevision::Commit {
                        commit_id: GitObjectId::new(oid.to_string())?,
                        side,
                    },
                    entries,
                })
            }
            CaptureView::Index => {
                let index = self
                    .repository
                    .index()
                    .map_err(|_| EvidenceCode::SourceUnavailable)?;
                if index.has_conflicts() {
                    return Err(EvidenceCode::SourceUnavailable.into());
                }
                let mut entries = BTreeMap::new();
                let mut identity_bytes = Vec::new();
                for entry in index.iter() {
                    self.check_cancel()?;
                    let path_hint =
                        PathBuf::from(String::from_utf8_lossy(&entry.path).into_owned());
                    if !self.time_available(Some(&path_hint)) {
                        break;
                    }
                    let path = std::str::from_utf8(&entry.path).map_err(|_| {
                        EvidenceError::from(EvidenceCode::UnsupportedEncoding).at_path(&path_hint)
                    })?;
                    identity_bytes.extend_from_slice(&entry.path);
                    identity_bytes.extend_from_slice(entry.id.as_bytes());
                    if path_is_allowed(&self.binding, std::path::Path::new(path))
                        .map_err(|error| error.at_path(&path_hint))?
                    {
                        entries.insert(path.into(), ReadTarget::Blob(entry.id));
                    }
                }
                Ok(FileCatalogue::Captured {
                    revision: SourceRevision::Index {
                        content_identity: content_digest(&identity_bytes),
                        side,
                    },
                    entries,
                })
            }
            CaptureView::Worktree => {
                let mut entries = BTreeMap::new();
                let worktree_root = self.binding.root.clone();
                for entry in ignore::WalkBuilder::new(&worktree_root)
                    .hidden(false)
                    .parents(false)
                    .git_ignore(!self.limits.include_ignored)
                    .git_exclude(!self.limits.include_ignored)
                    .git_global(false)
                    .filter_entry(|entry| entry.file_name() != ".git")
                    .build()
                {
                    self.check_cancel()?;
                    let path_hint = entry.as_ref().ok().map(|entry| {
                        entry
                            .path()
                            .strip_prefix(&self.binding.root)
                            .ok()
                            .filter(|path| !path.as_os_str().is_empty())
                            .map_or_else(
                                || entry.path().to_path_buf(),
                                std::path::Path::to_path_buf,
                            )
                    });
                    if !self.time_available(path_hint.as_deref()) {
                        break;
                    }
                    let entry = entry.map_err(|error| {
                        let mut refusal = EvidenceError::from(EvidenceCode::SourceUnavailable);
                        if let Some(path) = walk_error_path(&error) {
                            refusal = refusal.at_path(path);
                        }
                        refusal
                    })?;
                    if !entry
                        .file_type()
                        .is_some_and(|kind| kind.is_file() || kind.is_symlink())
                    {
                        continue;
                    }
                    let path = entry.path().strip_prefix(&self.binding.root).map_err(|_| {
                        EvidenceError::from(EvidenceCode::PathOutsideRoot).at_path(entry.path())
                    })?;
                    if path_is_allowed(&self.binding, path).map_err(|error| error.at_path(path))? {
                        entries.insert(path.into(), ReadTarget::Worktree);
                    }
                }
                for path in self.cached_worktree_paths() {
                    if path_is_allowed(&self.binding, &path)? {
                        entries.insert(path, ReadTarget::Worktree);
                    }
                }
                Ok(FileCatalogue::Captured {
                    revision: SourceRevision::Worktree {
                        snapshot: self.snapshot.clone(),
                        side,
                    },
                    entries,
                })
            }
        }
    }
}

fn walk_error_path(error: &ignore::Error) -> Option<&std::path::Path> {
    match error {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::Loop { child, .. } => Some(child),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            walk_error_path(err)
        }
        ignore::Error::Partial(errors) if errors.len() == 1 => {
            errors.first().and_then(walk_error_path)
        }
        _ => None,
    }
}

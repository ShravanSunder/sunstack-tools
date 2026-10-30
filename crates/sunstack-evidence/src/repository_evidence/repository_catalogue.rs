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
                let walk = tree.walk(TreeWalkMode::PreOrder, |prefix, entry| {
                    if self.check_cancel().is_err() {
                        failure = Some(EvidenceCode::MissingContext);
                        return TreeWalkResult::Abort;
                    }
                    if self.deadline_exhausted() {
                        timed_out = true;
                        return TreeWalkResult::Abort;
                    }
                    if entry.kind() == Some(ObjectType::Blob) {
                        let Ok(name) = entry.name() else {
                            failure = Some(EvidenceCode::UnsupportedEncoding);
                            return TreeWalkResult::Abort;
                        };
                        let path = PathBuf::from(format!("{prefix}{name}"));
                        match path_is_allowed(&self.binding, &path) {
                            Ok(true) => {
                                entries.insert(path, ReadTarget::Blob(entry.id()));
                            }
                            Ok(false) => {}
                            Err(error) => {
                                failure = Some(error.code);
                                return TreeWalkResult::Abort;
                            }
                        }
                    }
                    TreeWalkResult::Ok
                });
                if timed_out {
                    self.limited(DiscoveryLimit::LocalTime);
                } else {
                    walk.map_err(|_| failure.unwrap_or(EvidenceCode::RevisionUnavailable))?;
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
                    if !self.time_available() {
                        break;
                    }
                    let path = std::str::from_utf8(&entry.path)
                        .map_err(|_| EvidenceCode::UnsupportedEncoding)?;
                    identity_bytes.extend_from_slice(&entry.path);
                    identity_bytes.extend_from_slice(entry.id.as_bytes());
                    if path_is_allowed(&self.binding, std::path::Path::new(path))? {
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
                    if !self.time_available() {
                        break;
                    }
                    let entry = entry.map_err(|_| EvidenceCode::SourceUnavailable)?;
                    if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                        continue;
                    }
                    let path = entry
                        .path()
                        .strip_prefix(&self.binding.root)
                        .map_err(|_| EvidenceCode::PathOutsideRoot)?;
                    if path_is_allowed(&self.binding, path)? {
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

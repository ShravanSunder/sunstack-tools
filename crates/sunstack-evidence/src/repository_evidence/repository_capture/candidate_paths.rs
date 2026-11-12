use super::{
    worktree_capture::{WorktreeBytes, WorktreeReadPurpose},
    *,
};

#[derive(PartialEq, Eq)]
enum RawIdentity {
    Absent,
    Blob(git2::Oid),
    Limited,
}

impl RepositoryCapture<'_> {
    pub(super) fn raw_identity_changed(
        &mut self,
        before: &FileCatalogue,
        after: &FileCatalogue,
        path: &PathBuf,
    ) -> EvidenceResult<bool> {
        let old = self.raw_identity(before, path)?;
        if old == RawIdentity::Limited {
            return Ok(false);
        }
        let new = self.raw_identity(after, path)?;
        Ok(new != RawIdentity::Limited && old != new)
    }

    fn raw_identity(
        &mut self,
        catalogue: &FileCatalogue,
        path: &PathBuf,
    ) -> EvidenceResult<RawIdentity> {
        self.check_cancel().map_err(|error| error.at_path(path))?;
        if !self.time_available(Some(path)) {
            return Ok(RawIdentity::Limited);
        }
        let FileCatalogue::Captured { revision, entries } = catalogue else {
            return Ok(RawIdentity::Absent);
        };
        let Some(target) = entries.get(path) else {
            return Ok(RawIdentity::Absent);
        };
        match target {
            ReadTarget::Blob(oid) => {
                if matches!(revision, SourceRevision::Index { .. }) {
                    self.mutable_files.push(MutableRepositoryFile::Index {
                        root: self.binding.root.clone(),
                        path: path.clone(),
                        blob: Some(GitObjectId::new(oid.to_string())?),
                    });
                }
                Ok(RawIdentity::Blob(*oid))
            }
            ReadTarget::Worktree => self.raw_worktree_identity(path),
        }
    }

    fn raw_worktree_identity(&mut self, path: &PathBuf) -> EvidenceResult<RawIdentity> {
        if let Some(identity) = self.worktree_identities.get(path) {
            return Ok(RawIdentity::Blob(*identity));
        }
        let Some(contents) = self
            .read_worktree_bytes(path, WorktreeReadPurpose::IdentityProbe)
            .map_err(|error| error.at_path(path))?
        else {
            return Ok(RawIdentity::Limited);
        };
        let bytes = contents.bytes();
        self.total_bytes += bytes.len();
        let identity = git2::Oid::hash_object(git2::ObjectType::Blob, bytes)
            .map_err(|_| EvidenceError::from(EvidenceCode::SourceUnavailable).at_path(path))?;
        let snapshot = match contents {
            WorktreeBytes::File(bytes) => MutableRepositoryFile::WorktreeIdentity {
                root: self.binding.root.clone(),
                path: path.clone(),
                blob: GitObjectId::new(identity.to_string())?,
                byte_len: bytes.len(),
            },
            WorktreeBytes::Symlink { target, .. } => MutableRepositoryFile::WorktreeSymlink {
                root: self.binding.root.clone(),
                path: path.clone(),
                target,
            },
        };
        self.mutable_files.push(snapshot);
        self.worktree_identities.insert(path.clone(), identity);
        Ok(RawIdentity::Blob(identity))
    }
}

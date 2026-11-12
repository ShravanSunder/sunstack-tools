use super::*;
use std::io::Read;

pub(super) enum WorktreeReadPurpose {
    Acquisition,
    IdentityProbe,
}

pub(super) enum WorktreeBytes {
    File(Vec<u8>),
    Symlink { target: PathBuf, bytes: Vec<u8> },
}
impl WorktreeBytes {
    pub(super) fn bytes(&self) -> &[u8] {
        match self {
            Self::File(bytes) | Self::Symlink { bytes, .. } => bytes,
        }
    }
}

impl RepositoryCapture<'_> {
    pub(super) fn read_worktree_side_bytes(
        &mut self,
        path: &PathBuf,
    ) -> EvidenceResult<Option<Vec<u8>>> {
        let Some(contents) = self.read_worktree_bytes(path, WorktreeReadPurpose::Acquisition)?
        else {
            return Ok(None);
        };
        let (bytes, snapshot) = match contents {
            WorktreeBytes::File(bytes) => {
                let snapshot = MutableRepositoryFile::Worktree {
                    root: self.binding.root.clone(),
                    path: path.clone(),
                    bytes: Some(bytes.clone()),
                };
                (bytes, snapshot)
            }
            WorktreeBytes::Symlink { target, bytes } => (
                bytes,
                MutableRepositoryFile::WorktreeSymlink {
                    root: self.binding.root.clone(),
                    path: path.clone(),
                    target,
                },
            ),
        };
        self.mutable_files.push(snapshot);
        Ok(Some(bytes))
    }

    pub(super) fn read_worktree_bytes(
        &mut self,
        path: &PathBuf,
        purpose: WorktreeReadPurpose,
    ) -> EvidenceResult<Option<WorktreeBytes>> {
        let unresolved = self.binding.root.join(path);
        // Confine the parent without following the leaf: Git stores a symlink's
        // payload, not the contents or location of its target.
        let parent = unresolved
            .parent()
            .ok_or(EvidenceCode::PathOutsideRoot)?
            .canonicalize()
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        if !parent.starts_with(&self.binding.root) {
            return Err(EvidenceCode::PathOutsideRoot.into());
        }
        let metadata =
            std::fs::symlink_metadata(&unresolved).map_err(|_| EvidenceCode::SourceUnavailable)?;
        if metadata.len() > self.limits.max_file_bytes.get() as u64 {
            self.limited(DiscoveryLimit::FileBytes, Some(path));
            return Ok(None);
        }
        if metadata.len() > self.available_bytes.saturating_sub(self.total_bytes) as u64 {
            self.limited(DiscoveryLimit::TotalBytes, Some(path));
            return Ok(None);
        }
        let contents = if metadata.file_type().is_symlink() {
            if matches!(purpose, WorktreeReadPurpose::Acquisition) {
                self.progress.repository_file();
            }
            let target =
                std::fs::read_link(&unresolved).map_err(|_| EvidenceCode::SourceUnavailable)?;
            let bytes = target.as_os_str().as_encoded_bytes().to_vec();
            self.progress.source_bytes(bytes.len());
            WorktreeBytes::Symlink { target, bytes }
        } else {
            let resolved = unresolved
                .canonicalize()
                .map_err(|_| EvidenceCode::SourceUnavailable)?;
            if !resolved.starts_with(&self.binding.root) {
                return Err(EvidenceCode::PathOutsideRoot.into());
            }
            let mut bytes = Vec::new();
            let file =
                std::fs::File::open(&resolved).map_err(|_| EvidenceCode::SourceUnavailable)?;
            if matches!(purpose, WorktreeReadPurpose::Acquisition) {
                self.progress.repository_file();
            }
            let read = file
                .take(
                    self.limits
                        .max_file_bytes
                        .get()
                        .min(self.available_bytes.saturating_sub(self.total_bytes))
                        as u64,
                )
                .read_to_end(&mut bytes);
            self.progress.source_bytes(bytes.len());
            read.map_err(|_| EvidenceCode::SourceUnavailable)?;
            WorktreeBytes::File(bytes)
        };
        let bytes = contents.bytes();
        if bytes.len() > self.limits.max_file_bytes.get() {
            self.limited(DiscoveryLimit::FileBytes, Some(path));
            return Ok(None);
        }
        if bytes.len() > self.available_bytes.saturating_sub(self.total_bytes) {
            self.limited(DiscoveryLimit::TotalBytes, Some(path));
            return Ok(None);
        }
        Ok(Some(contents))
    }
}

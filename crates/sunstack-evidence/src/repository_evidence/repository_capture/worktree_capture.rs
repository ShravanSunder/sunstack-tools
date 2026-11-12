use super::*;
use std::io::Read;

impl RepositoryCapture<'_> {
    pub(super) fn read_worktree_side_bytes(
        &mut self,
        path: &PathBuf,
    ) -> EvidenceResult<Option<Vec<u8>>> {
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
        let (bytes, snapshot) = if metadata.file_type().is_symlink() {
            self.progress.repository_file();
            let target =
                std::fs::read_link(&unresolved).map_err(|_| EvidenceCode::SourceUnavailable)?;
            let bytes = target.as_os_str().as_encoded_bytes().to_vec();
            self.progress.source_bytes(bytes.len());
            let snapshot = MutableRepositoryFile::WorktreeSymlink {
                root: self.binding.root.clone(),
                path: path.clone(),
                target,
            };
            (bytes, snapshot)
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
            self.progress.repository_file();
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
            let snapshot = MutableRepositoryFile::Worktree {
                root: self.binding.root.clone(),
                path: path.clone(),
                bytes: Some(bytes.clone()),
            };
            (bytes, snapshot)
        };
        if bytes.len() > self.limits.max_file_bytes.get() {
            self.limited(DiscoveryLimit::FileBytes, Some(path));
            return Ok(None);
        }
        if bytes.len() > self.available_bytes.saturating_sub(self.total_bytes) {
            self.limited(DiscoveryLimit::TotalBytes, Some(path));
            return Ok(None);
        }
        self.mutable_files.push(snapshot);
        Ok(Some(bytes))
    }
}

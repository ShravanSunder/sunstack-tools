use std::{
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CheckedReadError {
    #[error("file is outside the repository root or not a regular file")]
    NotContained,
    #[error("file exceeds the byte limit")]
    TooLarge,
    #[error("file cannot be read as UTF-8")]
    Unreadable,
}

/// Every acquisition of checked text or a target opening crosses this boundary.
pub trait CheckedFileSource {
    fn resolve_file(&self, path: &Path) -> Option<PathBuf>;
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError>;
    fn checked_files_read(&self) -> usize;
}

pub struct FilesystemCheckedSource {
    root: PathBuf,
    reads: usize,
}
impl FilesystemCheckedSource {
    pub fn new(root: &Path) -> Result<Self, CheckedReadError> {
        Ok(Self {
            root: root
                .canonicalize()
                .map_err(|_| CheckedReadError::NotContained)?,
            reads: 0,
        })
    }
}
impl CheckedFileSource for FilesystemCheckedSource {
    fn resolve_file(&self, path: &Path) -> Option<PathBuf> {
        let resolved = self.root.join(path).canonicalize().ok()?;
        if !resolved.is_file() {
            return None;
        }
        resolved
            .strip_prefix(&self.root)
            .ok()
            .map(Path::to_path_buf)
    }
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError> {
        let path = self
            .resolve_file(path)
            .ok_or(CheckedReadError::NotContained)?;
        self.reads = self.reads.saturating_add(1);
        let file =
            std::fs::File::open(self.root.join(path)).map_err(|_| CheckedReadError::Unreadable)?;
        let mut bytes = Vec::new();
        file.take(
            u64::try_from(max_bytes)
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut bytes)
        .map_err(|_| CheckedReadError::Unreadable)?;
        if bytes.len() > max_bytes {
            return Err(CheckedReadError::TooLarge);
        }
        String::from_utf8(bytes).map_err(|_| CheckedReadError::Unreadable)
    }
    fn checked_files_read(&self) -> usize {
        self.reads
    }
}

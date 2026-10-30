use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CheckedReadError {
    #[error("file is outside the repository root or not a regular file")]
    NotContained,
    #[error("file exceeds the byte limit")]
    TooLarge,
    #[error("max_total_bytes")]
    TotalBytesExceeded,
    #[error("max_files")]
    FileCountExceeded,
    #[error("file cannot be read as UTF-8")]
    Unreadable,
}

/// Reuse the verified Git snapshot; fallback acquisitions stay inside the checked-file boundary.
pub struct FrozenCheckedSource {
    frozen: BTreeMap<PathBuf, String>,
    fallback: Box<dyn CheckedFileSource + Send>,
    acquired: BTreeSet<PathBuf>,
}
impl FrozenCheckedSource {
    pub fn new(
        frozen: BTreeMap<PathBuf, String>,
        fallback: Box<dyn CheckedFileSource + Send>,
    ) -> Self {
        Self {
            frozen,
            fallback,
            acquired: BTreeSet::new(),
        }
    }
}
fn normalized_relative(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => result.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return None,
        }
    }
    Some(result)
}
// Identity comparison never opens the target. Existing-file resolution remains
// responsible for filesystem containment and readable-content acquisition.
pub(crate) fn contained_target_identity(repository_root: &Path, path: &Path) -> Option<PathBuf> {
    let relative = if path.is_absolute() {
        path.strip_prefix(repository_root).ok()?
    } else {
        path
    };
    normalized_relative(relative)
}
impl CheckedFileSource for FrozenCheckedSource {
    fn resolve_file(&self, path: &Path) -> Option<PathBuf> {
        let normalized = normalized_relative(path).or_else(|| self.fallback.resolve_file(path))?;
        if self.frozen.contains_key(&normalized) {
            Some(normalized)
        } else {
            self.fallback.resolve_file(&normalized)
        }
    }
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError> {
        let path = self
            .resolve_file(path)
            .ok_or(CheckedReadError::NotContained)?;
        if let Some(text) = self.frozen.get(&path) {
            self.acquired.insert(path);
            if text.len() > max_bytes {
                return Err(CheckedReadError::TooLarge);
            }
            return Ok(text.clone());
        }
        self.fallback.read_file(&path, max_bytes)
    }
    fn checked_files_acquired(&self) -> usize {
        self.acquired.len() + self.fallback.checked_files_acquired()
    }
}

/// Every acquisition of checked text or a target opening crosses this boundary.
pub trait CheckedFileSource {
    fn resolve_file(&self, path: &Path) -> Option<PathBuf>;
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError>;
    fn checked_files_acquired(&self) -> usize;
}

pub struct FilesystemCheckedSource {
    root: PathBuf,
    acquired: BTreeSet<PathBuf>,
}
impl FilesystemCheckedSource {
    pub fn new(root: &Path) -> Result<Self, CheckedReadError> {
        Ok(Self {
            root: root
                .canonicalize()
                .map_err(|_| CheckedReadError::NotContained)?,
            acquired: BTreeSet::new(),
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
        let file =
            std::fs::File::open(self.root.join(&path)).map_err(|_| CheckedReadError::Unreadable)?;
        let mut bytes = Vec::new();
        file.take(
            u64::try_from(max_bytes)
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut bytes)
        .map_err(|_| CheckedReadError::Unreadable)?;
        self.acquired.insert(path);
        if bytes.len() > max_bytes {
            return Err(CheckedReadError::TooLarge);
        }
        String::from_utf8(bytes).map_err(|_| CheckedReadError::Unreadable)
    }
    fn checked_files_acquired(&self) -> usize {
        self.acquired.len()
    }
}

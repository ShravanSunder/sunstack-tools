use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CheckedReadError {
    #[error("file is outside the repository root or not a regular file")]
    NotContained,
    #[error("file is missing")]
    Missing { path: PathBuf },
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
    fn resolve_file(&self, path: &Path) -> Result<PathBuf, CheckedReadError> {
        if let Some(normalized) = normalized_relative(path)
            && self.frozen.contains_key(&normalized)
        {
            return Ok(normalized);
        }
        match self.fallback.resolve_file(path) {
            Err(CheckedReadError::Missing { path }) if self.frozen.contains_key(&path) => Ok(path),
            other => other,
        }
    }
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError> {
        let path = self.resolve_file(path)?;
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
    /// Resolve one checked file without acquiring bytes, preserving missing/outside identity.
    fn resolve_file(&self, path: &Path) -> Result<PathBuf, CheckedReadError>;
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
    fn resolve_file(&self, path: &Path) -> Result<PathBuf, CheckedReadError> {
        let requested = self.root.join(path);
        let resolved = match requested.canonicalize() {
            Ok(resolved) => resolved,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(CheckedReadError::Missing {
                    path: missing_identity(&self.root, &requested)?,
                });
            }
            Err(_) => return Err(CheckedReadError::Unreadable),
        };
        let path = resolved
            .strip_prefix(&self.root)
            .map(Path::to_path_buf)
            .map_err(|_| CheckedReadError::NotContained)?;
        if resolved.is_file() {
            Ok(path)
        } else {
            Err(CheckedReadError::Missing { path })
        }
    }
    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError> {
        let path = self.resolve_file(path)?;
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

fn missing_identity(root: &Path, requested: &Path) -> Result<PathBuf, CheckedReadError> {
    let mut ancestor = requested.to_path_buf();
    let mut missing = Vec::new();
    loop {
        match ancestor.canonicalize() {
            Ok(mut resolved) => {
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return contained_target_identity(root, &resolved)
                    .ok_or(CheckedReadError::NotContained);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = ancestor
                    .file_name()
                    .ok_or(CheckedReadError::NotContained)?
                    .to_owned();
                missing.push(name);
                if !ancestor.pop() {
                    return Err(CheckedReadError::NotContained);
                }
            }
            Err(_) => return Err(CheckedReadError::Unreadable),
        }
    }
}

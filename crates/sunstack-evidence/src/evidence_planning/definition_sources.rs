//! Definition provenance shared by file loading, typed inline values and edited snapshots.
//!
//! An origin names where an executable definition came from. File origins keep the
//! CLI's existing path serialization at legacy report edges through [`cli_path`];
//! inline and edited origins exist only for in-memory callers and are never written
//! as fictional file paths.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The locator of captured original content: a file read from disk or an inline source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum SourceLocator {
    File { path: PathBuf },
    Inline { id: String, label: String },
}

/// Where an executable definition value came from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum DefinitionOrigin {
    File {
        path: PathBuf,
    },
    Inline {
        id: String,
        label: String,
    },
    /// A changed value whose baseline was the linked file or inline source.
    Edited {
        id: String,
        based_on: SourceLocator,
    },
}

impl DefinitionOrigin {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File { path: path.into() }
    }
    /// The real file this definition was read from, when it has one.
    pub fn file_path(&self) -> Option<&Path> {
        match self {
            Self::File { path } => Some(path),
            Self::Inline { .. } | Self::Edited { .. } => None,
        }
    }
    /// The CLI path projection: real file paths only; in-memory origins have none.
    pub fn cli_path(&self) -> PathBuf {
        self.file_path().map(Path::to_path_buf).unwrap_or_default()
    }
}

impl From<SourceLocator> for DefinitionOrigin {
    fn from(locator: SourceLocator) -> Self {
        match locator {
            SourceLocator::File { path } => Self::File { path },
            SourceLocator::Inline { id, label } => Self::Inline { id, label },
        }
    }
}

/// How snapshot content represents its definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum SnapshotRepresentation {
    /// The exact original UTF-8 bytes of a file or inline text.
    OriginalText,
    /// The Rust-normalized serialization of a typed value.
    NormalizedValue,
}

/// How the call obtained the snapshot content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum SnapshotAcquisition {
    /// Read from the filesystem during this load or call.
    Read,
    /// Supplied by the caller as previously captured content; no new read happened.
    SuppliedSnapshot,
    /// Authored inline by the caller.
    Inline,
}

/// Exact content of one definition source actually used by a call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub struct DefinitionSnapshot {
    pub origin: DefinitionOrigin,
    pub content: String,
    pub sha256: String,
    pub representation: SnapshotRepresentation,
    pub acquisition: SnapshotAcquisition,
}

impl DefinitionSnapshot {
    pub fn new(
        origin: DefinitionOrigin,
        content: String,
        representation: SnapshotRepresentation,
        acquisition: SnapshotAcquisition,
    ) -> Self {
        Self {
            sha256: crate::content_digest(content.as_bytes()),
            origin,
            content,
            representation,
            acquisition,
        }
    }
}

/// Why a referenced definition source could not be acquired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum SourceUnavailable {
    NotFound,
    Unreadable,
    Io,
    Encoding,
}

/// A definition file that could not be read, before any decoding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: definition source is unavailable ({reason:?})", path.display())]
pub struct SourceAcquisitionFailure {
    pub path: PathBuf,
    pub reason: SourceUnavailable,
    /// The operating system's description, kept for existing CLI wording.
    pub message: String,
}

/// One definition file read exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedFile {
    pub path: PathBuf,
    pub text: String,
}

/// Read a definition file once, classifying failures without retry.
pub fn read_definition_file(path: &Path) -> Result<CapturedFile, SourceAcquisitionFailure> {
    let bytes = std::fs::read(path).map_err(|error| SourceAcquisitionFailure {
        path: path.to_path_buf(),
        reason: match error.kind() {
            std::io::ErrorKind::NotFound => SourceUnavailable::NotFound,
            std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::IsADirectory => {
                SourceUnavailable::Unreadable
            }
            _ => SourceUnavailable::Io,
        },
        message: error.to_string(),
    })?;
    // Same wording as `std::fs::read_to_string` for invalid UTF-8.
    let text = String::from_utf8(bytes).map_err(|_| SourceAcquisitionFailure {
        path: path.to_path_buf(),
        reason: SourceUnavailable::Encoding,
        message: "stream did not contain valid UTF-8".into(),
    })?;
    Ok(CapturedFile {
        path: path.to_path_buf(),
        text,
    })
}

/// Serialize a definition origin at a legacy CLI report edge as its real file path.
///
/// CLI reports only ever hold file origins. An in-memory origin reaching a CLI
/// serializer is refused instead of being written as a fictional path.
pub mod cli_path {
    use super::DefinitionOrigin;
    use serde::{Deserialize, Deserializer, Serializer, ser::Error};
    use std::path::PathBuf;

    pub fn serialize<S: Serializer>(
        origin: &DefinitionOrigin,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match origin.file_path() {
            Some(path) => serde::Serialize::serialize(path, serializer),
            None => Err(S::Error::custom(
                "an in-memory definition origin has no CLI report path",
            )),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DefinitionOrigin, D::Error> {
        PathBuf::deserialize(deserializer).map(DefinitionOrigin::file)
    }
}

#[cfg(test)]
#[path = "definition_sources_test.rs"]
mod tests;

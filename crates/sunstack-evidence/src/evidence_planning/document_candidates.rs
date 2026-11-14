use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "CandidateWire", into = "CandidateWire")]
#[schemars(with = "CandidateWire")]
pub struct DocumentCandidates(CandidateWire);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
enum CandidateWire {
    Single(PathBuf),
    FirstExisting(#[schemars(length(min = 1))] Vec<PathBuf>),
}
impl DocumentCandidates {
    pub fn paths(&self) -> &[PathBuf] {
        match &self.0 {
            CandidateWire::Single(path) => std::slice::from_ref(path),
            CandidateWire::FirstExisting(paths) => paths,
        }
    }
    pub fn records_resolution(&self) -> bool {
        matches!(self.0, CandidateWire::FirstExisting(_))
    }
}
impl TryFrom<CandidateWire> for DocumentCandidates {
    type Error = &'static str;
    fn try_from(wire: CandidateWire) -> Result<Self, Self::Error> {
        let value = Self(wire);
        // Scalar paths retain the existing admission-stage empty-path diagnostic.
        if value.records_resolution()
            && (value.paths().is_empty()
                || value.paths().iter().any(|path| path.as_os_str().is_empty()))
        {
            return Err(
                "document path requires a nonempty path or nonempty list of nonempty paths",
            );
        }
        Ok(value)
    }
}
impl From<DocumentCandidates> for CandidateWire {
    fn from(value: DocumentCandidates) -> Self {
        value.0
    }
}
impl From<PathBuf> for DocumentCandidates {
    fn from(path: PathBuf) -> Self {
        Self(CandidateWire::Single(path))
    }
}
impl From<&str> for DocumentCandidates {
    fn from(path: &str) -> Self {
        Self(CandidateWire::Single(Path::new(path).to_path_buf()))
    }
}

#[cfg(test)]
#[path = "document_candidates_test.rs"]
mod tests;

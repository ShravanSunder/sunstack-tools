//! Per-acquisition resource consumption, returned to the session even on refusal.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
pub(crate) struct AcquisitionProgress {
    repository_files: AtomicUsize,
    source_bytes: AtomicUsize,
    transcript_bytes: AtomicUsize,
    history_commits: AtomicUsize,
}
impl AcquisitionProgress {
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }
    pub fn repository_file(&self) {
        self.repository_files.fetch_add(1, Ordering::Relaxed);
    }
    pub fn source_bytes(&self, count: usize) {
        self.source_bytes.fetch_add(count, Ordering::Relaxed);
    }
    pub fn transcript_bytes(&self, count: usize) {
        self.source_bytes(count);
        self.transcript_bytes.fetch_add(count, Ordering::Relaxed);
    }
    pub fn history_commit(&self) {
        self.history_commits.fetch_add(1, Ordering::Relaxed);
    }
    pub fn consumed_repository_files(&self) -> usize {
        self.repository_files.load(Ordering::Relaxed)
    }
    pub fn consumed_bytes(&self) -> usize {
        self.source_bytes.load(Ordering::Relaxed)
    }
    pub fn consumed_transcript_bytes(&self) -> usize {
        self.transcript_bytes.load(Ordering::Relaxed)
    }
    pub fn consumed_history_commits(&self) -> usize {
        self.history_commits.load(Ordering::Relaxed)
    }
}

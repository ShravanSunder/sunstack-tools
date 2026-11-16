#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

struct CountingCheckedSource {
    inner: crate::FilesystemCheckedSource,
    resolves: std::cell::Cell<usize>,
    reads: usize,
}

impl CheckedFileSource for CountingCheckedSource {
    fn resolve_file(&self, path: &Path) -> Result<PathBuf, CheckedReadError> {
        self.resolves.set(self.resolves.get() + 1);
        self.inner.resolve_file(path)
    }

    fn read_file(&mut self, path: &Path, max_bytes: usize) -> Result<String, CheckedReadError> {
        self.reads += 1;
        if path == Path::new("failed.html") {
            return Err(CheckedReadError::Unreadable);
        }
        self.inner.read_file(path, max_bytes)
    }

    fn checked_files_acquired(&self) -> usize {
        self.inner.checked_files_acquired()
    }
}

#[test]
fn captured_byte_queries_use_only_original_acquired_bytes() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(root.path().join("sgconfig.yml"), "ruleDirs: [rules]\n").unwrap();
    std::fs::write(root.path().join("source.html"), "a\r\néz").unwrap();
    std::fs::write(root.path().join("failed.html"), "unreadable").unwrap();
    let project = crate::load_evidence_project(
        &root.path().join("sgconfig.yml"),
        &[],
        None,
        &Default::default(),
    )
    .unwrap();
    let mut source = CountingCheckedSource {
        inner: crate::FilesystemCheckedSource::new(root.path()).unwrap(),
        resolves: std::cell::Cell::new(0),
        reads: 0,
    };
    let build = EvidenceBuild::start(
        &project,
        &BTreeMap::new(),
        &["source.html".into(), "failed.html".into()],
        &mut source,
        PieceBuildLimits {
            max_files: 10,
            max_file_bytes: 10000,
            max_total_bytes: 100000,
            max_matches: 10,
            evidence_max_chars: 6000,
        },
    )
    .unwrap();
    let attempts = (source.resolves.get(), source.reads);
    assert_eq!(attempts.1, 2, "capture must attempt the failing source");
    std::fs::write(root.path().join("source.html"), "changed").unwrap();
    std::fs::remove_file(root.path().join("source.html")).unwrap();

    assert_eq!(
        build.captured_byte_range_digest(Path::new("source.html"), [1, 4]),
        CapturedByteRange::Captured {
            digest: content_digest(&[b'\r', b'\n', 0xc3]),
        },
    );
    for bytes in [[0, 7], [2, 2], [4, 3]] {
        assert_eq!(
            build.captured_byte_range_digest(Path::new("source.html"), bytes),
            CapturedByteRange::OutOfBounds,
        );
    }
    for file in ["failed.html", "never-requested.html"] {
        assert_eq!(
            build.captured_byte_range_digest(Path::new(file), [0, 1]),
            CapturedByteRange::Unacquired,
        );
    }
    assert_eq!((source.resolves.get(), source.reads), attempts);

    // Sensitivity: an actual failed read must be visible to this counter.
    assert!(source.read_file(Path::new("failed.html"), 100).is_err());
    assert_eq!(source.reads, attempts.1 + 1);
}

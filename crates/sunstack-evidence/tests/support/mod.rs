#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(dead_code)]
use git2::{Oid, Repository, RepositoryInitOptions, Signature, Time};
use sunstack_evidence::*;
pub struct GitFixture {
    pub directory: tempfile::TempDir,
    pub repository: Repository,
}
impl GitFixture {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(directory.path(), &options).unwrap();
        Self {
            directory,
            repository,
        }
    }
    pub fn write(&self, path: &str, text: &str) {
        let path = self.directory.path().join(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, text).unwrap();
    }
    pub fn stage(&self, path: &str) {
        let mut index = self.repository.index().unwrap();
        index.add_path(std::path::Path::new(path)).unwrap();
        index.write().unwrap();
    }
    pub fn commit(&self, seconds: i64) -> Oid {
        let mut index = self.repository.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = self.repository.find_tree(tree_id).unwrap();
        let signature = Signature::new(
            "Fixture Author",
            "fixture@example.invalid",
            &Time::new(seconds, 0),
        )
        .unwrap();
        let parent = self
            .repository
            .head()
            .ok()
            .and_then(|head| head.target())
            .map(|id| self.repository.find_commit(id).unwrap());
        let parents = parent.iter().collect::<Vec<_>>();
        self.repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "fixture commit",
                &tree,
                &parents,
            )
            .unwrap()
    }
    pub fn binding(&self, scope: ChangeScope) -> RepositoryBinding {
        RepositoryBinding {
            key: RepositoryId::new("fixture").unwrap(),
            root: self.directory.path().into(),
            input_paths: vec![QueryPath::Glob {
                pattern: "**".into(),
            }],
            change_scope: scope,
        }
    }
}
pub fn discovery_limits() -> DiscoveryLimits {
    DiscoveryLimits {
        max_files: 100.try_into().unwrap(),
        max_file_bytes: 1000000.try_into().unwrap(),
        max_total_bytes: 10000000.try_into().unwrap(),
        max_matches: 1000.try_into().unwrap(),
        include_ignored: false,
        max_unit_visits: 1000.try_into().unwrap(),
        max_relationship_candidates: 1000.try_into().unwrap(),
        max_fan_out: 100.try_into().unwrap(),
        max_history_commits: 100.try_into().unwrap(),
        max_local_time_ms: 10000.try_into().unwrap(),
    }
}

pub mod connected_inputs;

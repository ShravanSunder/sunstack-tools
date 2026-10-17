#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use std::path::Path;
use sunstack_evidence::*;
pub struct ChangeFixture {
    pub root: tempfile::TempDir,
    repository: git2::Repository,
}
impl ChangeFixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repository = git2::Repository::init(root.path()).unwrap();
        Self { root, repository }
    }
    pub fn write(&self, path: &str, content: &str) {
        let path = self.root.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    pub fn stage(&self, path: &str) {
        let mut index = self.repository.index().unwrap();
        index.add_path(Path::new(path)).unwrap();
        index.write().unwrap();
    }
    pub fn commit(&self) -> String {
        let mut index = self.repository.index().unwrap();
        let tree = self
            .repository
            .find_tree(index.write_tree().unwrap())
            .unwrap();
        let signature = git2::Signature::new(
            "Fixture",
            "fixture@example.invalid",
            &git2::Time::new(1000, 0),
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
                "fixture",
                &tree,
                &parents,
            )
            .unwrap()
            .to_string()
    }
    pub fn binding(&self, key: &str, scope: ChangeScope) -> RepositoryBinding {
        RepositoryBinding {
            key: RepositoryId::new(key).unwrap(),
            root: self.root.path().into(),
            input_paths: vec![QueryPath::Glob {
                pattern: "**".into(),
            }],
            change_scope: scope,
        }
    }
}
pub fn discovery() -> EvidenceDiscovery {
    EvidenceDiscovery::Bounded {
        limits: DiscoveryLimits {
            max_files: 100.try_into().unwrap(),
            max_file_bytes: 100000.try_into().unwrap(),
            max_total_bytes: 1000000.try_into().unwrap(),
            max_matches: 1000.try_into().unwrap(),
            include_ignored: false,
            max_unit_visits: 1000.try_into().unwrap(),
            max_relationship_candidates: 1000.try_into().unwrap(),
            max_fan_out: 100.try_into().unwrap(),
            max_history_commits: 100.try_into().unwrap(),
            max_local_time_ms: 10000.try_into().unwrap(),
        },
    }
}
pub fn related() -> serde_json::Value {
    serde_json::json!({"kind":"related_units","languages":["typescript"],"unit_kinds":["function"],"source_sides":"after_only","directions":["callers"],"requested_hops":1,"chunk_settings":{"grouping":"single_group","seed_content":{"kind":"whole_unit"},"related_content":{"kind":"whole_unit"},"max_chunk_bytes":10000},"minimum_related_units":1})
}

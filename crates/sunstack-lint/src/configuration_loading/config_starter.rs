use crate::*;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

const STARTER_FILES: &[(&str, &str)] = &[
    (
        "rulebook.md",
        "Paragraphs contain no unfinished TODO markers.<!-- K001 -->\n",
    ),
    (
        "rules.json",
        r#"{
  "K001": {
    "name": "unfinished_marker", "evidence": "paragraph", "severity": "warn",
    "decision": {"start": "unfinished", "steps": {
      "unfinished": {
        "mechanical_check": {"tool": "regex", "field": "body", "pattern": "\\bTODO\\b"},
        "yes": {"finding": "Resolve this TODO marker.", "at": "body"}, "no": "pass"
      }
    }}
  }
}
"#,
    ),
    ("sgconfig.yml", "ruleDirs: [rules]\n"),
    (
        "rules/paragraph.yml",
        "id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n",
    ),
    (
        "sample.md",
        "# Example\n\nA finished paragraph for a local-only check.\n",
    ),
];

pub(super) fn create_starter(directory: &Path) -> Result<PathBuf, LintInvalid> {
    let config_path = directory.join("sunstack.lint.json");
    let starter = directory.join("sunstack-starter");
    let failure = |path: &Path, error: &dyn std::fmt::Display| {
        LintInvalid::at(
            path,
            LintInvalidCode::Syntax,
            format!(
                "Cannot create mechanical starter at {}: {error}. Existing files are never overwritten.",
                path.display()
            ),
        )
    };
    // Refuse before any write, including broken symlinks at either reserved path.
    for path in [&config_path, &starter] {
        match std::fs::symlink_metadata(path) {
            Ok(_) => return Err(failure(path, &"path already exists")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(failure(path, &error)),
        }
    }
    let config = serde_json::json!({
        "$schema": format!("https://github.com/ShravanSunder/sunstack-tools/releases/download/v{}/sunstack.lint.schema.json", env!("CARGO_PKG_VERSION")),
        "rulebook": [{"markdown": "sunstack-starter/rulebook.md", "rules": "sunstack-starter/rules.json"}],
        "marker_prefix": "K", "check": {"files": ["sunstack-starter/sample.md"]},
        "evidence": {"ast_grep": "sunstack-starter/sgconfig.yml", "kinds": {"paragraph": {
            "rule": "paragraph", "fields": {"body": {"from": "capture", "regex": "(?s)(.+)"}}
        }}}, "decision_model": {"model": "jev-latest"}
    });
    let mut config_bytes =
        serde_json::to_vec_pretty(&config).map_err(|error| failure(&config_path, &error))?;
    config_bytes.push(b'\n');
    std::fs::create_dir(&starter).map_err(|error| failure(&starter, &error))?;
    let rules_directory = starter.join("rules");
    let mut created = Vec::new();
    let result = (|| {
        std::fs::create_dir(&rules_directory).map_err(|error| failure(&rules_directory, &error))?;
        for (name, text) in STARTER_FILES {
            write_new(&starter.join(name), text.as_bytes(), &mut created)
                .map_err(|error| failure(&starter.join(name), &error))?;
        }
        write_new(&config_path, &config_bytes, &mut created)
            .map_err(|error| failure(&config_path, &error))?;
        Ok(config_path)
    })();
    if result.is_err() {
        for path in created.iter().rev() {
            let _ = std::fs::remove_file(path);
        }
        let _ = std::fs::remove_dir(&rules_directory);
        let _ = std::fs::remove_dir(&starter);
    }
    result
}

fn write_new(path: &Path, bytes: &[u8], created: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    created.push(path.into());
    file.write_all(bytes)
}

#[cfg(test)]
#[path = "config_starter_test.rs"]
mod tests;

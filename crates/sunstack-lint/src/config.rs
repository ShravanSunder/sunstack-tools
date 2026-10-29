use crate::*;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub fn discover_config(
    start: &Path,
    explicit: Option<&Path>,
) -> Result<ConfigLocation, LintInvalid> {
    if let Some(explicit) = explicit {
        return locate(start.join(explicit));
    }
    let mut directory = start.to_path_buf();
    loop {
        let candidate = directory.join("sunstack.lint.json");
        if candidate.is_file() {
            return locate(candidate);
        }
        if !directory.pop() {
            break;
        }
    }
    Err(LintInvalid::at(
        start.join("sunstack.lint.json"),
        LintInvalidCode::NoConfigFound,
        "No sunstack.lint.json found in this directory or its parents.",
    ))
}
fn locate(file: PathBuf) -> Result<ConfigLocation, LintInvalid> {
    let file = file.canonicalize().map_err(|_| {
        LintInvalid::at(
            &file,
            LintInvalidCode::NoConfigFound,
            "Cannot open the selected sunstack.lint.json.",
        )
    })?;
    let directory = file
        .parent()
        .ok_or_else(|| {
            LintInvalid::at(
                &file,
                LintInvalidCode::NoConfigFound,
                "The config has no parent directory.",
            )
        })?
        .to_path_buf();
    Ok(ConfigLocation { file, directory })
}
pub fn load_config(location: &ConfigLocation) -> Result<RulesConfig, LintInvalid> {
    let text = std::fs::read_to_string(&location.file).map_err(|_| {
        LintInvalid::at(
            &location.file,
            LintInvalidCode::Syntax,
            "Cannot read sunstack.lint.json as UTF-8.",
        )
    })?;
    crate::config_decoding::decode_config(&text, location)
}
pub fn config_schema() -> schemars::Schema {
    schemars::schema_for!(RulesConfig)
}
pub fn init_config(directory: &Path) -> Result<ConfigLocation, LintInvalid> {
    let path = directory.join("sunstack.lint.json");
    let skeleton = serde_json::json!({
        "$schema":format!("https://github.com/ShravanSunder/sunstack-tools/releases/download/v{}/sunstack.lint.schema.json",env!("CARGO_PKG_VERSION")),
        "rulebook":[{"markdown":"rulebook.md","rules":"rulebook.rules.json"}],
        "marker_prefix":"K",
        "check":{"files":["**/*.md"]},
        "evidence":{"ast_grep":"evidence/sgconfig.yml","kinds":{"piece":{"rule":"your-rule-id","fields":{"body":{"from":"remainder"}}}}},
        "decision_model":{"model":"jev-latest"}
    });
    let bytes = serde_json::to_vec_pretty(&skeleton).map_err(|_| {
        LintInvalid::at(
            &path,
            LintInvalidCode::Syntax,
            "Cannot encode the config skeleton.",
        )
    })?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| {
            LintInvalid::at(
                &path,
                LintInvalidCode::Syntax,
                "Cannot create sunstack.lint.json; an existing file is never overwritten.",
            )
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|_| {
            LintInvalid::at(
                &path,
                LintInvalidCode::Syntax,
                "Cannot write the config skeleton.",
            )
        })?;
    locate(path)
}

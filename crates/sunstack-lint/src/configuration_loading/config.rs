use crate::*;
use std::{
    io::ErrorKind,
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
    let metadata = std::fs::metadata(&file).map_err(|error| config_open_error(&file, &error))?;
    if !metadata.is_file() {
        return Err(LintInvalid::at(
            &file,
            LintInvalidCode::NoConfigFound,
            format!("Cannot open config {}: not a file.", file.display()),
        ));
    }
    let file = file
        .canonicalize()
        .map_err(|error| config_open_error(&file, &error))?;
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
fn config_open_error(file: &Path, error: &std::io::Error) -> LintInvalid {
    let reason = if error.kind() == ErrorKind::NotFound {
        "not found"
    } else {
        "unreadable"
    };
    LintInvalid::at(
        file,
        LintInvalidCode::NoConfigFound,
        format!("Cannot open config {}: {reason} ({error}).", file.display()),
    )
}
pub fn load_config(location: &ConfigLocation) -> Result<RulesConfig, LintInvalid> {
    let text = std::fs::read_to_string(&location.file).map_err(|error| {
        if error.kind() == ErrorKind::InvalidData {
            LintInvalid::at(
                &location.file,
                LintInvalidCode::Syntax,
                format!("Cannot read config {} as UTF-8.", location.file.display()),
            )
        } else {
            config_open_error(&location.file, &error)
        }
    })?;
    crate::configuration_loading::config_decoding::decode_config(&text, location)
}
/// Decode `sunstack.lint.json` text exactly as `load_config` does, without reading it.
pub fn parse_config(text: &str, location: &ConfigLocation) -> Result<RulesConfig, LintInvalid> {
    crate::configuration_loading::config_decoding::decode_config(text, location)
}
pub fn config_schema() -> schemars::Schema {
    schemars::schema_for!(RulesConfig)
}
pub fn init_config(directory: &Path) -> Result<ConfigLocation, LintInvalid> {
    let path = super::config_starter::create_starter(directory)?;
    locate(path)
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;

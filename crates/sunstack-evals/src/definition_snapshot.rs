use crate::{case_validation::invalid, *};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};
use sunstack_evidence::content_digest;

pub(crate) type DefinitionContents = BTreeMap<PathBuf, Vec<u8>>;

pub(crate) fn relative_definition_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
pub(crate) fn capture_definitions(
    input: &FreezeInput,
) -> Result<(Vec<FrozenFile>, DefinitionContents), EvalError> {
    let root = input
        .definition_root
        .canonicalize()
        .map_err(|_| EvalError::File {
            path: input.definition_root.clone(),
        })?;
    if input.definition_files.is_empty() || !relative_definition_path(&input.rule.rule_set) {
        return Err(invalid("rule.rule_set"));
    }
    let mut files = Vec::new();
    let mut contents = BTreeMap::new();
    for path in &input.definition_files {
        if path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(invalid("definition_files.path"));
        }
        let actual = root
            .join(path)
            .canonicalize()
            .map_err(|_| EvalError::File { path: path.clone() })?;
        let relative = actual
            .strip_prefix(&root)
            .map_err(|_| invalid("definition_files.outside_root"))?
            .to_owned();
        if !relative_definition_path(&relative) {
            return Err(invalid("definition_files.path"));
        }
        let bytes = std::fs::read(&actual).map_err(|_| EvalError::File {
            path: actual.clone(),
        })?;
        files.push(FrozenFile {
            path: relative.clone(),
            content_identity: ContentIdentity::new(content_digest(&bytes))?,
        });
        if contents.insert(relative, bytes).is_some() {
            return Err(invalid("definition_files"));
        }
    }
    Ok((files, contents))
}
pub(crate) fn write_definitions(
    directory: &Path,
    contents: &DefinitionContents,
) -> Result<(), EvalError> {
    for (relative, bytes) in contents {
        let path = directory.join("definitions").join(relative);
        let parent = path
            .parent()
            .ok_or_else(|| invalid("definition_files.path"))?;
        std::fs::create_dir_all(parent).map_err(|_| EvalError::File {
            path: parent.into(),
        })?;
        crate::case_store::write_new(&path, bytes)?;
    }
    Ok(())
}
pub(crate) fn load_definitions(
    home: &Path,
    files: &[FrozenFile],
) -> Result<DefinitionContents, EvalError> {
    let mut contents = BTreeMap::new();
    for file in files {
        if !relative_definition_path(&file.path) {
            return Err(invalid("manifest.definition_files.path"));
        }
        let path = home.join(&file.path);
        let actual = path
            .canonicalize()
            .map_err(|_| EvalError::File { path: path.clone() })?;
        if !actual.starts_with(home) {
            return Err(invalid("manifest.definition_files.path"));
        }
        let bytes = std::fs::read(&actual).map_err(|_| EvalError::File { path: path.clone() })?;
        if file.content_identity.as_str() != content_digest(&bytes) {
            return Err(EvalError::Integrity { path });
        }
        if contents.insert(file.path.clone(), bytes).is_some() {
            return Err(invalid("manifest.definition_files.path"));
        }
    }
    Ok(contents)
}

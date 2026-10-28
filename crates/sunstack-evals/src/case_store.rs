use crate::{
    case_validation::{invalid, validate_cases},
    frozen_decoding::decode,
    *,
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use sunstack_evidence::content_digest;

fn read(path: &Path) -> Result<Vec<u8>, EvalError> {
    fs::read(path).map_err(|_| EvalError::File { path: path.into() })
}
fn encoded(value: &impl Serialize) -> Result<Vec<u8>, EvalError> {
    serde_json::to_vec(value).map_err(|_| invalid("json"))
}
fn frozen_file(path: &Path) -> Result<FrozenFile, EvalError> {
    let path = path
        .canonicalize()
        .map_err(|_| EvalError::File { path: path.into() })?;
    Ok(FrozenFile {
        content_identity: ContentIdentity::new(content_digest(&read(&path)?))?,
        path,
    })
}
fn freeze_files(paths: &[PathBuf], field: &str) -> Result<Vec<FrozenFile>, EvalError> {
    if paths.is_empty() {
        return Err(invalid(field));
    }
    let files = paths
        .iter()
        .map(|path| frozen_file(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique = BTreeSet::new();
    if files.iter().any(|file| !unique.insert(&file.path)) {
        return Err(invalid(field));
    }
    Ok(files)
}
pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), EvalError> {
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|_| EvalError::File { path: path.into() })
}
fn write_lines<T: Serialize>(path: &Path, values: &[T]) -> Result<(), EvalError> {
    let mut bytes = vec![];
    for value in values {
        bytes.extend(encoded(value)?);
        bytes.push(b'\n');
    }
    write_new(path, &bytes)
}
fn read_lines<T: DeserializeOwned>(bytes: &[u8]) -> Result<Vec<T>, EvalError> {
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(decode)
        .collect()
}
fn verified_bytes(file: &FrozenFile) -> Result<Vec<u8>, EvalError> {
    let bytes = read(&file.path)?;
    if file.content_identity.as_str() != content_digest(&bytes) {
        return Err(EvalError::Integrity {
            path: file.path.clone(),
        });
    }
    Ok(bytes)
}
fn validate_manifest(manifest: &CaseSetManifest) -> Result<(), EvalError> {
    if manifest.definition_files.is_empty() || manifest.source_files.is_empty() {
        return Err(invalid("manifest.files"));
    }
    let mut unique = BTreeSet::new();
    for file in std::iter::once(&manifest.sources)
        .chain(std::iter::once(&manifest.labels))
        .chain(&manifest.source_files)
    {
        if !file.path.is_absolute() || !unique.insert(&file.path) {
            return Err(invalid("manifest.path"));
        }
    }
    let mut definition_paths = BTreeSet::new();
    if !crate::definition_snapshot::relative_definition_path(&manifest.rule.rule_set)
        || manifest.definition_files.iter().any(|file| {
            !crate::definition_snapshot::relative_definition_path(&file.path)
                || !definition_paths.insert(&file.path)
        })
    {
        return Err(invalid("manifest.definition_files.path"));
    }
    Ok(())
}

/// Freeze JSONL sources and labels separately. Definition bytes are copied under definitions/;
/// referenced captures stay pinned in place, and loading refuses changed frozen bytes.
/// The destination is new and never overwrites an existing frozen set.
pub fn freeze_cases(directory: &Path, input: &FreezeInput) -> Result<FrozenCaseSet, EvalError> {
    validate_cases(&input.sources, &input.labels)?;
    let source_files = freeze_files(&input.source_files, "source_files")?;
    let (definition_files, definition_bytes) =
        crate::definition_snapshot::capture_definitions(input)?;
    sunstack_rules::frozen_rule_pass_means(&input.rule, definition_bytes.clone())
        .map_err(|diagnostics| EvalError::Definitions { diagnostics })?;
    fs::create_dir(directory).map_err(|_| EvalError::File {
        path: directory.into(),
    })?;
    crate::definition_snapshot::write_definitions(directory, &definition_bytes)?;
    write_lines(&directory.join("sources.jsonl"), &input.sources)?;
    write_lines(&directory.join("labels.jsonl"), &input.labels)?;
    let manifest = CaseSetManifest {
        rule: input.rule.clone(),
        split: input.split,
        sources: frozen_file(&directory.join("sources.jsonl"))?,
        labels: frozen_file(&directory.join("labels.jsonl"))?,
        source_files,
        definition_files,
    };
    validate_manifest(&manifest)?;
    write_new(&directory.join("manifest.json"), &encoded(&manifest)?)?;
    load_case_set(&directory.join("manifest.json"))
}

/// Verify every pinned byte sequence before interpreting either JSONL file. Returned data is
/// an immutable version: callers reload for a new run rather than mixing it with changed files.
pub fn load_case_set(path: &Path) -> Result<FrozenCaseSet, EvalError> {
    let bytes = read(path)?;
    let manifest: CaseSetManifest = decode(&bytes)?;
    validate_manifest(&manifest)?;
    let sources_bytes = verified_bytes(&manifest.sources)?;
    let labels_bytes = verified_bytes(&manifest.labels)?;
    for file in &manifest.source_files {
        verified_bytes(file)?;
    }
    let definition_path = path
        .parent()
        .ok_or_else(|| invalid("manifest.path"))?
        .join("definitions");
    let definition_home = definition_path
        .canonicalize()
        .map_err(|_| EvalError::File {
            path: definition_path,
        })?;
    let definition_bytes =
        crate::definition_snapshot::load_definitions(&definition_home, &manifest.definition_files)?;
    let pass_means = sunstack_rules::frozen_rule_pass_means(&manifest.rule, definition_bytes)
        .map_err(|diagnostics| EvalError::Definitions { diagnostics })?;
    let sources = read_lines(&sources_bytes)?;
    let labels = read_lines(&labels_bytes)?;
    validate_cases(&sources, &labels)?;
    let identity = CaseSetIdentity::new(content_digest(&bytes))?;
    // Paths identify storage, not question content. Preserve declared file order in the version.
    let definition_identities = manifest
        .definition_files
        .iter()
        .map(|file| &file.content_identity)
        .collect::<Vec<_>>();
    let rule_version = ContentIdentity::new(content_digest(&encoded(&definition_identities)?))?;
    Ok(FrozenCaseSet {
        manifest,
        identity,
        rule_version,
        definition_home,
        pass_means,
        sources,
        labels,
    })
}

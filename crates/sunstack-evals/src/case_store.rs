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
    let (definition_files, mut definition_bytes) =
        crate::definition_snapshot::capture_definitions(input)?;
    let payload_schemas = crate::payload_snapshot::capture_schemas(input, &mut definition_bytes)?;
    sunstack_rules::admit_frozen_entry(
        &input.rule,
        definition_bytes.clone(),
        &input.entry,
        &crate::payload_snapshot::schema_paths(&payload_schemas),
    )
    .map_err(|diagnostics| EvalError::Definitions { diagnostics })?;
    fs::create_dir(directory).map_err(|_| EvalError::File {
        path: directory.into(),
    })?;
    crate::definition_snapshot::write_definitions(directory, &definition_bytes)?;
    write_lines(&directory.join("sources.jsonl"), &input.sources)?;
    write_lines(&directory.join("labels.jsonl"), &input.labels)?;
    let manifest = CaseSetManifest {
        entry: input.entry.clone(),
        payload_schemas,
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
    verify_for_replay(path)?.finish()
}

pub(crate) struct VerifiedReplayInput {
    pub manifest: CaseSetManifest,
    pub sources: Vec<CaseSource>,
    pub definition_home: PathBuf,
    identity: CaseSetIdentity,
    rule_version: ContentIdentity,
    pass_means: sunstack_rules::PassMeans,
    labels_bytes: Vec<u8>,
}
impl VerifiedReplayInput {
    pub fn finish(self) -> Result<FrozenCaseSet, EvalError> {
        let labels = read_lines(&self.labels_bytes)?;
        validate_cases(&self.sources, &labels)?;
        Ok(FrozenCaseSet {
            manifest: self.manifest,
            sources: self.sources,
            labels,
            definition_home: self.definition_home,
            identity: self.identity,
            rule_version: self.rule_version,
            pass_means: self.pass_means,
        })
    }
}
pub(crate) fn verify_for_replay(path: &Path) -> Result<VerifiedReplayInput, EvalError> {
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
    let mut definition_bytes =
        crate::definition_snapshot::load_definitions(&definition_home, &manifest.definition_files)?;
    crate::payload_snapshot::load_schemas(
        &definition_home,
        &manifest.payload_schemas,
        &mut definition_bytes,
    )?;
    let pass_means = sunstack_rules::admit_frozen_entry(
        &manifest.rule,
        definition_bytes,
        &manifest.entry,
        &crate::payload_snapshot::schema_paths(&manifest.payload_schemas),
    )
    .map_err(|diagnostics| EvalError::Definitions { diagnostics })?;
    let sources = read_lines(&sources_bytes)?;

    let identity = CaseSetIdentity::new(content_digest(&bytes))?;
    // Paths identify storage, not question content. Preserve declared file order in the version.
    let definition_identities = manifest
        .definition_files
        .iter()
        .map(|file| &file.content_identity)
        .chain(
            manifest
                .payload_schemas
                .iter()
                .map(|schema| &schema.content_identity),
        )
        .collect::<Vec<_>>();
    let rule_version = ContentIdentity::new(content_digest(&encoded(&definition_identities)?))?;
    Ok(VerifiedReplayInput {
        manifest,
        identity,
        rule_version,
        definition_home,
        pass_means,
        sources,
        labels_bytes,
    })
}

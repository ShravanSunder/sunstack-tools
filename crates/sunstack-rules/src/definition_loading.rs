use crate::{
    definition_admission::locate, input_binding::resolved_path,
    invocation_admission::LoadedRuleSet, *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio_util::sync::CancellationToken;
#[derive(Debug, Clone)]
pub struct LocatedRuleDefinition {
    pub source: PathBuf,
    pub line: usize,
    pub definition: Arc<RuleDefinition>,
}
#[derive(Debug, Clone)]
pub struct ValidatedRuleFile {
    pub source: PathBuf,
    pub content_identity: String,
    pub rules: Vec<LocatedRuleDefinition>,
}
pub(crate) struct DefinitionLoader {
    max_bytes: usize,
    cancellation: CancellationToken,
    files: BTreeMap<PathBuf, Arc<ValidatedRuleFile>>,
    sets: BTreeMap<PathBuf, Arc<LoadedRuleSet>>,
}
impl DefinitionLoader {
    pub(crate) fn new(max_bytes: usize, cancellation: CancellationToken) -> Self {
        Self {
            max_bytes,
            cancellation,
            files: BTreeMap::new(),
            sets: BTreeMap::new(),
        }
    }
    pub(crate) fn read(&self, path: &Path) -> Result<Vec<u8>, DiagnosticList> {
        let invalid = || {
            RuleDiagnostic::new(RuleDiagnosticCode::MalformedJson)
                .definition(path, 1, 1, vec![])
                .into()
        };
        if self.cancellation.is_cancelled() {
            return Err(DiagnosticList::rule(RuleDiagnosticCode::CallerAborted));
        }
        let metadata = std::fs::metadata(path).map_err(|_| invalid())?;
        if !metadata.is_file() || metadata.len() > self.max_bytes as u64 {
            return Err(invalid());
        }
        let mut file = std::fs::File::open(path).map_err(|_| invalid())?;
        let mut bytes = Vec::new();
        let mut buffer = [0; 16384];
        loop {
            if self.cancellation.is_cancelled() {
                return Err(DiagnosticList::rule(RuleDiagnosticCode::CallerAborted));
            }
            let read = file.read(&mut buffer).map_err(|_| invalid())?;
            if read == 0 {
                break;
            }
            if bytes.len().saturating_add(read) > self.max_bytes {
                return Err(invalid());
            }
            bytes.extend_from_slice(&buffer[..read]);
        }
        Ok(bytes)
    }
    pub(crate) fn load_set(&mut self, path: PathBuf) -> Result<Arc<LoadedRuleSet>, DiagnosticList> {
        if let Some(set) = self.sets.get(&path) {
            return Ok(set.clone());
        }
        let definition = decode_rule_set(&self.read(&path)?, &path)?;
        let directory = path
            .parent()
            .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
        let mut source_paths = BTreeSet::new();
        let mut rule_ids = BTreeSet::new();
        let mut rules = Vec::new();
        for source in &definition.rule_paths {
            let source = resolved_path(directory, source)?;
            if !source_paths.insert(source.clone()) {
                return Err(RuleDiagnostic::new(RuleDiagnosticCode::DuplicateRuleSource)
                    .definition(&path, 1, 1, vec!["rule_paths".into()])
                    .into());
            }
            let file = if let Some(file) = self.files.get(&source) {
                file.clone()
            } else {
                let bytes = self.read(&source)?;
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::MalformedJson))?;
                let mut located = Vec::new();
                for (position, line) in text.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let rule = decode_rule_definition(line.as_bytes(), &source, position + 1)?;
                    located.push(LocatedRuleDefinition {
                        source: source.clone(),
                        line: position + 1,
                        definition: Arc::new(rule),
                    });
                }
                if located.is_empty() {
                    return Err(RuleDiagnostic::new(RuleDiagnosticCode::EmptyRuleFile)
                        .definition(&source, 1, 1, vec![])
                        .into());
                }
                let file = Arc::new(ValidatedRuleFile {
                    source: source.clone(),
                    content_identity: sunstack_evidence::content_digest(&bytes),
                    rules: located,
                });
                self.files.insert(source, file.clone());
                file
            };
            for located in &file.rules {
                let rule = &located.definition;
                if !rule_ids.insert(rule.rule_id.clone()) {
                    return Err(RuleDiagnostic::new(RuleDiagnosticCode::DuplicateRuleId)
                        .definition(&located.source, located.line, 1, vec!["rule_id".into()])
                        .into());
                }
                validate_rule_definition(rule, &definition.inputs).map_err(|mut diagnostics| {
                    locate(&mut diagnostics, &located.source, located.line);
                    diagnostics
                })?;
                rules.push(located.clone());
            }
        }
        let set = Arc::new(LoadedRuleSet {
            path: path.clone(),
            definition,
            rules,
            skipped: vec![],
        });
        self.sets.insert(path, set.clone());
        Ok(set)
    }
}

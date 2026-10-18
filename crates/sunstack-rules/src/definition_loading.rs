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
    pub provenance: Option<RuleSourceProvenance>,
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
                        provenance: None,
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
        let mut skipped = Vec::new();
        let mut assembled = Vec::new();
        let mut pairs = BTreeSet::new();
        for pair in &definition.rule_sources {
            let pair = RuleSourcePair {
                markdown: resolved_path(directory, &pair.markdown)?,
                rules: resolved_path(directory, &pair.rules)?,
            };
            if !pairs.insert((pair.markdown.clone(), pair.rules.clone())) {
                return Err(RuleDiagnostic::new(RuleDiagnosticCode::DuplicateRuleSource)
                    .definition(&path, 1, 1, vec!["rule_sources".into()])
                    .into());
            }
            match self.load_source(&pair, &path, &definition) {
                Ok(source) => {
                    assembled.extend(source.rules);
                    skipped.extend(source.skipped);
                }
                Err((source_file, diagnostics)) => {
                    if self.cancellation.is_cancelled() {
                        return Err(diagnostics);
                    }
                    skipped.push(SkippedRule::Source {
                        rule_set: path.clone(),
                        source_file,
                        markdown: pair.markdown,
                        rules: pair.rules,
                        diagnostics,
                    });
                }
            }
        }
        let mut names = BTreeMap::new();
        for located in &assembled {
            *names
                .entry(located.definition.rule_id.clone())
                .or_insert(0usize) += 1;
        }
        for located in assembled {
            if rule_ids.contains(&located.definition.rule_id)
                || names
                    .get(&located.definition.rule_id)
                    .copied()
                    .unwrap_or_default()
                    > 1
            {
                let provenance = located.provenance.as_ref().ok_or_else(|| {
                    DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError)
                })?;
                let diagnostics = RuleDiagnostic::new(RuleDiagnosticCode::DuplicateRuleId)
                    .definition(
                        &located.source,
                        provenance.entry_location.line,
                        provenance.entry_location.column,
                        vec!["name".into()],
                    )
                    .into();
                skipped.push(SkippedRule::Entry {
                    rule_set: path.clone(),
                    markdown: provenance.markdown.clone(),
                    rules: provenance.rules.clone(),
                    key: provenance.marker_id.as_str().into(),
                    rule_id: Some(located.definition.rule_id.clone()),
                    diagnostics,
                });
            } else {
                rules.push(located);
            }
        }
        let set = Arc::new(LoadedRuleSet {
            path: path.clone(),
            definition,
            rules,
            skipped,
        });
        self.sets.insert(path, set.clone());
        Ok(set)
    }

    fn load_source(
        &self,
        pair: &RuleSourcePair,
        path: &Path,
        definition: &RuleSetDefinition,
    ) -> Result<crate::rule_source_assembly::AssembledSource, (PathBuf, DiagnosticList)> {
        let markdown = self
            .read(&pair.markdown)
            .map_err(|errors| (pair.markdown.clone(), errors))?;
        let markdown = std::str::from_utf8(&markdown).map_err(|_| {
            (
                pair.markdown.clone(),
                RuleDiagnostic::new(RuleDiagnosticCode::MalformedJson)
                    .definition(&pair.markdown, 1, 1, vec![])
                    .into(),
            )
        })?;
        let rules = self
            .read(&pair.rules)
            .map_err(|errors| (pair.rules.clone(), errors))?;
        let prefix = definition.marker_prefix.as_ref().ok_or_else(|| {
            (
                path.to_owned(),
                DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration),
            )
        })?;
        crate::rule_source_assembly::assemble_source(
            markdown,
            &rules,
            pair,
            path,
            prefix,
            definition.max_question_words,
            &definition.inputs,
        )
        .map_err(|errors| (pair.rules.clone(), errors))
    }
}

//! One immutable set of captured definitions, independent of how it was acquired.
//!
//! File loading and typed in-memory values both produce a [`DefinitionBundle`].
//! Admission and execution read only the bundle; nothing downstream reopens a
//! definition file.
use crate::{
    CheckedFiles, ConfigLocation, DecisionModelSettings, LintInvalid, LintInvalidCode, LintLimits,
    RulebookFile, RulesConfig,
};
use std::{
    collections::BTreeMap,
    num::NonZeroU32,
    path::{Path, PathBuf},
};
use sunstack_evidence::{
    ConfigPath, DefinitionOrigin, DefinitionSnapshot, EvidenceKindDefinition, EvidenceKindName,
    EvidenceProjectDefinitions, EvidenceProjectFiles, MarkerPrefix, ProjectReadFailure,
};
use sunstack_rules::{FilterName, FilterWire, RulebookReadFailure, RulebookSource};

/// Settings every configuration supplies, whatever its acquisition format.
#[derive(Debug, Clone)]
pub struct ConfigurationSettings {
    pub marker_prefix: MarkerPrefix,
    pub max_question_words: NonZeroU32,
    pub check: CheckedFiles,
    pub decision_model: DecisionModelSettings,
    pub limits: Option<LintLimits>,
    pub filters: BTreeMap<FilterName, FilterWire>,
    pub kinds: BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    /// The project's HTML and Markdown conventions, applied wherever its evidence is read.
    pub document: sunstack_evidence::DocumentEvidenceSettings,
}

impl From<&RulesConfig> for ConfigurationSettings {
    fn from(config: &RulesConfig) -> Self {
        Self {
            marker_prefix: config.marker_prefix.clone(),
            max_question_words: config.max_question_words,
            check: config.check.clone(),
            decision_model: config.decision_model.clone(),
            limits: config.limits.clone(),
            filters: config.filters.clone(),
            kinds: config.evidence.kinds.clone(),
            document: config.evidence.document_settings(),
        }
    }
}

/// The root definition: where settings came from and how diagnostics locate fields.
#[derive(Debug, Clone)]
pub struct RootDefinition {
    pub origin: DefinitionOrigin,
    /// Original JSON text for line/column positions; absent for typed values.
    pub text: Option<String>,
    /// Base for resolving definition references; never the checked root.
    pub base_directory: PathBuf,
}

impl RootDefinition {
    /// A refusal owned by the root definition, at its first position.
    pub fn invalid(&self, code: LintInvalidCode, message: impl Into<String>) -> LintInvalid {
        LintInvalid::at(self.origin.cli_path(), code, message).in_definition(&self.origin)
    }

    /// A refusal at a configuration field, positioned in the original text when known.
    pub fn field_error(
        &self,
        path: &ConfigPath,
        code: LintInvalidCode,
        message: impl Into<String>,
    ) -> LintInvalid {
        let (line, column) = match &self.text {
            Some(text) => {
                let position =
                    crate::configuration_loading::config_decoding::config_field_position(
                        text, path,
                    )
                    .unwrap_or(0);
                let prefix = &text[..position];
                (
                    u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1)
                        .unwrap_or(u32::MAX),
                    u32::try_from(
                        prefix
                            .rsplit('\n')
                            .next()
                            .map_or(1, |tail| tail.chars().count() + 1),
                    )
                    .unwrap_or(u32::MAX),
                )
            }
            None => (1, 1),
        };
        LintInvalid {
            file: self.origin.cli_path(),
            line: Some(line),
            column: Some(column),
            json_path: vec![path.to_string()],
            code,
            message: message.into(),
            definition: Some(Box::new(self.origin.clone())),
        }
    }
}

/// Evidence definitions as acquired: a read ast-grep project or decoded values.
#[derive(Clone)]
pub enum EvidenceAcquisition {
    /// A project read once from files; decoding and compiling preserve CLI refusal order.
    ProjectFiles(Box<Result<EvidenceProjectFiles, ProjectReadFailure>>),
    /// Already decoded definitions with their origins.
    Definitions(EvidenceProjectDefinitions),
}

/// Everything admission and execution need; immutable once built.
#[derive(Clone)]
pub struct DefinitionBundle {
    pub root: RootDefinition,
    pub settings: ConfigurationSettings,
    pub evidence: EvidenceAcquisition,
    /// The origin inline kind matchers are attributed to.
    pub inline_kind_origin: DefinitionOrigin,
    pub rulebooks: Vec<Result<RulebookSource, RulebookReadFailure>>,
    /// Exact definition content the call used, for SDK provenance.
    pub sources: Vec<DefinitionSnapshot>,
}

impl DefinitionBundle {
    /// Exact captured text of a definition file this bundle read.
    pub(crate) fn captured_text(&self, path: &Path) -> Option<&str> {
        if self.root.origin.file_path() == Some(path) {
            return self.root.text.as_deref();
        }
        for rulebook in self
            .rulebooks
            .iter()
            .filter_map(|rulebook| rulebook.as_ref().ok())
        {
            if rulebook.markdown_origin.file_path() == Some(path) {
                return Some(&rulebook.markdown);
            }
            if rulebook.entries_origin.file_path() == Some(path) {
                return std::str::from_utf8(&rulebook.entries).ok();
            }
        }
        if let EvidenceAcquisition::ProjectFiles(files) = &self.evidence
            && let Ok(files) = files.as_ref()
        {
            if files.project.path == path {
                return Some(&files.project.text);
            }
            for file in files
                .utility_files
                .iter()
                .chain(&files.rule_files)
                .filter_map(|file| file.as_ref().ok())
            {
                if file.path == path {
                    return Some(&file.text);
                }
            }
        }
        self.sources
            .iter()
            .find(|source| source.origin.file_path() == Some(path))
            .map(|source| source.content.as_str())
    }

    /// The CLI's `rulebook_files` from captured bytes; no definition file is reopened.
    pub(crate) fn file_snapshots(&self, paths: &[PathBuf]) -> Result<Vec<RulebookFile>, PathBuf> {
        paths
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|path| {
                let content = self.captured_text(path).ok_or_else(|| path.clone())?;
                Ok(RulebookFile {
                    path: path.clone(),
                    sha256: sunstack_evidence::content_digest(content.as_bytes()),
                    content: content.to_owned(),
                })
            })
            .collect()
    }
}

/// Acquire a decoded file configuration's referenced definitions, each read once.
///
/// Read failures are retained in place so admission reports them at the same point
/// the CLI always has.
pub fn acquire_file_definitions(
    config: &RulesConfig,
    location: &ConfigLocation,
) -> DefinitionBundle {
    let base = location.directory.clone();
    let root_origin = DefinitionOrigin::file(&location.file);
    let evidence = sunstack_evidence::read_evidence_project(&base.join(&config.evidence.ast_grep));
    let rulebooks = config
        .rulebook
        .iter()
        .map(|pair| {
            sunstack_rules::read_rulebook_pair(&sunstack_rules::RulebookPair {
                markdown: base.join(&pair.markdown),
                rules: base.join(&pair.rules),
            })
        })
        .collect();
    DefinitionBundle {
        root: RootDefinition {
            origin: root_origin.clone(),
            text: config.source.as_ref().map(|source| source.text.clone()),
            base_directory: base,
        },
        settings: ConfigurationSettings::from(config),
        evidence: EvidenceAcquisition::ProjectFiles(Box::new(evidence)),
        inline_kind_origin: root_origin,
        rulebooks,
        sources: vec![],
    }
}

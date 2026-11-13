use crate::{CapturedFile, DefinitionOrigin, SourceAcquisitionFailure, read_definition_file};
use ast_grep_config::{DeserializeEnv, GlobalRules, RuleCollection, SerializableGlobalRule};
use ast_grep_language::{LanguageExt, SupportLang};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {message}", path.display())]
pub struct ProjectRefusal {
    pub origin: Option<crate::ConfigPath>,
    /// The CLI path projection of `definition` (empty for in-memory definitions).
    pub path: PathBuf,
    /// The definition, or definition file/folder, that the refusal concerns.
    pub definition: Box<DefinitionOrigin>,
    pub message: String,
    pub line: usize,
    pub column: Option<usize>,
}
impl ProjectRefusal {
    fn at(path: &Path, message: impl ToString) -> Self {
        Self::in_definition(&DefinitionOrigin::file(path), message)
    }
    pub fn in_definition(definition: &DefinitionOrigin, message: impl ToString) -> Self {
        Self {
            origin: None,
            path: definition.cli_path(),
            definition: Box::new(definition.clone()),
            message: message.to_string(),
            line: 1,
            column: None,
        }
    }
    fn located(
        definition: &DefinitionOrigin,
        message: impl ToString,
        position: Option<(u64, u64)>,
    ) -> Self {
        let mut refusal = Self::in_definition(definition, message);
        if let Some((line, column)) = position {
            refusal.line = usize::try_from(line).unwrap_or(usize::MAX);
            refusal.column = usize::try_from(column).ok();
        }
        refusal
    }
}

#[derive(Debug, Clone)]
pub struct InlineRule {
    pub kind: crate::EvidenceKindName,
    pub text: String,
    pub origin: crate::ConfigPath,
}

#[derive(Deserialize)]
struct InlineRuleWire {
    #[serde(default, deserialize_with = "refuse_inline_id")]
    #[serde(rename = "id")]
    _id: (),
    #[serde(flatten)]
    rule: ast_grep_config::SerializableRuleConfig<SupportLang>,
}

fn refuse_inline_id<'de, D: serde::Deserializer<'de>>(_: D) -> Result<(), D::Error> {
    Err(serde::de::Error::custom("inline_rule must not declare id"))
}

#[derive(Clone)]
pub struct EvidenceProject {
    pub rules: Arc<RuleCollection<SupportLang>>,
    pub rule_ids: BTreeSet<crate::EvidenceRuleId>,
    pub inputs: Vec<PathBuf>,
    rule_directories: Vec<PathBuf>,
    rule_languages: BTreeMap<crate::EvidenceRuleId, EvidenceLanguage>,
    pub document_settings: crate::DocumentEvidenceSettings,
}

/// Result-side language names (snake_case), distinct from authored matcher spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "typescript", derive(specta::Type))]
pub enum EvidenceLanguage {
    Bash,
    C,
    Cpp,
    CSharp,
    Css,
    Dart,
    Go,
    Elixir,
    Haskell,
    Hcl,
    Html,
    Java,
    JavaScript,
    Json,
    Kotlin,
    Lua,
    Markdown,
    Nix,
    Php,
    Python,
    Ruby,
    Rust,
    Scala,
    Solidity,
    Swift,
    Tsx,
    TypeScript,
    Yaml,
    Zig,
}
impl EvidenceLanguage {
    fn from_support(language: SupportLang) -> Self {
        match language {
            SupportLang::Bash => Self::Bash,
            SupportLang::C => Self::C,
            SupportLang::Cpp => Self::Cpp,
            SupportLang::CSharp => Self::CSharp,
            SupportLang::Css => Self::Css,
            SupportLang::Dart => Self::Dart,
            SupportLang::Go => Self::Go,
            SupportLang::Elixir => Self::Elixir,
            SupportLang::Haskell => Self::Haskell,
            SupportLang::Hcl => Self::Hcl,
            SupportLang::Html => Self::Html,
            SupportLang::Java => Self::Java,
            SupportLang::JavaScript => Self::JavaScript,
            SupportLang::Json => Self::Json,
            SupportLang::Kotlin => Self::Kotlin,
            SupportLang::Lua => Self::Lua,
            SupportLang::Markdown => Self::Markdown,
            SupportLang::Nix => Self::Nix,
            SupportLang::Php => Self::Php,
            SupportLang::Python => Self::Python,
            SupportLang::Ruby => Self::Ruby,
            SupportLang::Rust => Self::Rust,
            SupportLang::Scala => Self::Scala,
            SupportLang::Solidity => Self::Solidity,
            SupportLang::Swift => Self::Swift,
            SupportLang::Tsx => Self::Tsx,
            SupportLang::TypeScript => Self::TypeScript,
            SupportLang::Yaml => Self::Yaml,
            SupportLang::Zig => Self::Zig,
        }
    }
    fn support(self) -> SupportLang {
        match self {
            Self::Bash => SupportLang::Bash,
            Self::C => SupportLang::C,
            Self::Cpp => SupportLang::Cpp,
            Self::CSharp => SupportLang::CSharp,
            Self::Css => SupportLang::Css,
            Self::Dart => SupportLang::Dart,
            Self::Go => SupportLang::Go,
            Self::Elixir => SupportLang::Elixir,
            Self::Haskell => SupportLang::Haskell,
            Self::Hcl => SupportLang::Hcl,
            Self::Html => SupportLang::Html,
            Self::Java => SupportLang::Java,
            Self::JavaScript => SupportLang::JavaScript,
            Self::Json => SupportLang::Json,
            Self::Kotlin => SupportLang::Kotlin,
            Self::Lua => SupportLang::Lua,
            Self::Markdown => SupportLang::Markdown,
            Self::Nix => SupportLang::Nix,
            Self::Php => SupportLang::Php,
            Self::Python => SupportLang::Python,
            Self::Ruby => SupportLang::Ruby,
            Self::Rust => SupportLang::Rust,
            Self::Scala => SupportLang::Scala,
            Self::Solidity => SupportLang::Solidity,
            Self::Swift => SupportLang::Swift,
            Self::Tsx => SupportLang::Tsx,
            Self::TypeScript => SupportLang::TypeScript,
            Self::Yaml => SupportLang::Yaml,
            Self::Zig => SupportLang::Zig,
        }
    }
    fn rejects_parse_errors(self) -> bool {
        matches!(self, Self::Json | Self::Yaml | Self::Hcl)
    }
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MatchRange {
    pub start: usize,
    pub end: usize,
}
/// Parser identity is meaningful only within one file's match index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeKey(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentChild {
    pub node: NodeKey,
    pub range: MatchRange,
    pub continuation: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentEntry {
    pub parent: NodeKey,
    pub children: Vec<ParentChild>,
    pub position: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMatchIndex {
    pub by_rule: BTreeMap<crate::EvidenceRuleId, Vec<MatchRange>>,
    pub limited: bool,
    pub match_nodes: BTreeMap<crate::EvidenceRuleId, Vec<NodeKey>>,
    pub parents: BTreeMap<NodeKey, ParentEntry>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub failed_languages: BTreeSet<EvidenceLanguage>,
}

impl EvidenceProject {
    /// Declared rule directories resolved against the ast-grep config directory.
    pub fn rule_directories(&self) -> &[PathBuf] {
        &self.rule_directories
    }
    pub fn language_for_rule(&self, rule: &crate::EvidenceRuleId) -> Option<EvidenceLanguage> {
        self.rule_languages.get(rule).copied()
    }
    pub fn match_file(&self, path: &Path, text: &str, max_matches: usize) -> FileMatchIndex {
        self.match_rules(text, max_matches, self.rules.for_path(path))
    }
    pub(crate) fn match_file_as_language(
        &self,
        path: &Path,
        text: &str,
        max_matches: usize,
        language: EvidenceLanguage,
    ) -> FileMatchIndex {
        let mut rules = self.rules.get_rule_from_lang(path, language.support());
        rules.sort_by_key(|rule| &rule.id);
        self.match_rules(text, max_matches, rules)
    }
    fn match_rules(
        &self,
        text: &str,
        max_matches: usize,
        rules: Vec<&ast_grep_config::RuleConfig<SupportLang>>,
    ) -> FileMatchIndex {
        let mut index = FileMatchIndex::default();
        let languages = rules
            .iter()
            .map(|rule| EvidenceLanguage::from_support(rule.language))
            .collect::<BTreeSet<_>>();
        // Keep every tree alive so its node identities cannot be reused by another parse.
        let documents = languages
            .into_iter()
            .map(|language| (language, language.support().ast_grep(text)))
            .collect::<BTreeMap<_, _>>();
        for (language, document) in &documents {
            if language.rejects_parse_errors() && document.root().get_inner_node().has_error() {
                index.failed_languages.insert(*language);
            }
        }
        let mut count = 0usize;
        for rule in rules {
            let language = EvidenceLanguage::from_support(rule.language);
            if index.failed_languages.contains(&language) {
                continue;
            }
            let Some(document) = documents.get(&language) else {
                continue;
            };
            let mut spans = Vec::new();
            for found in document.root().find_all(&rule.matcher) {
                if count >= max_matches {
                    index.limited = true;
                    break;
                }
                let range = found.range();
                let node = NodeKey(found.node_id());
                if let Some(parent) = found.parent() {
                    let children = parent
                        .children()
                        .filter(|child| child.is_named())
                        .map(|child| {
                            let range = child.range();
                            ParentChild {
                                node: NodeKey(child.node_id()),
                                range: MatchRange {
                                    start: range.start,
                                    end: range.end,
                                },
                                continuation: child.kind() == "block_continuation",
                            }
                        })
                        .collect::<Vec<_>>();
                    if let Some(position) = children.iter().position(|child| child.node == node) {
                        index.parents.entry(node).or_insert(ParentEntry {
                            parent: NodeKey(parent.node_id()),
                            children,
                            position,
                        });
                    }
                }
                spans.push((
                    MatchRange {
                        start: range.start,
                        end: range.end,
                    },
                    node,
                ));
                count = count.saturating_add(1);
            }
            spans.sort_by_key(|(span, _)| (span.start, span.end));
            if let Ok(id) = crate::EvidenceRuleId::new(rule.id.clone()) {
                let (ranges, nodes) = spans.into_iter().unzip();
                index.match_nodes.insert(id.clone(), nodes);
                index.by_rule.insert(id, ranges);
            }
        }
        index
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSettings {
    rule_dirs: Vec<PathBuf>,
    #[serde(default)]
    util_dirs: Vec<PathBuf>,
}

/// One utility definition and the source it was authored in.
#[derive(Clone)]
pub struct UtilityDefinition {
    pub origin: DefinitionOrigin,
    pub utility: SerializableGlobalRule<SupportLang>,
}

/// One named matcher definition and the source it was authored in.
#[derive(Clone)]
pub struct MatcherDefinition {
    pub origin: DefinitionOrigin,
    pub matcher: ast_grep_config::SerializableRuleConfig<SupportLang>,
}

/// Decoded evidence definitions, independent of how they were acquired.
#[derive(Clone)]
pub struct EvidenceProjectDefinitions {
    /// Owner of project-wide refusals: the ast-grep project file or the configuration.
    pub project: DefinitionOrigin,
    pub rule_directories: Vec<PathBuf>,
    pub utilities: Vec<UtilityDefinition>,
    pub matchers: Vec<MatcherDefinition>,
    /// Real definition files consulted, in acquisition order.
    pub inputs: Vec<PathBuf>,
}

/// One acquisition step of an ast-grep project, kept in the order the CLI visits it.
pub type AcquiredFile = Result<CapturedFile, Box<ProjectAcquisitionFailure>>;

/// A project file or folder that could not be acquired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAcquisitionFailure {
    pub source: SourceAcquisitionFailure,
    /// The existing CLI wording for this failure.
    pub refusal: ProjectRefusal,
}

/// Every file of an ast-grep project read exactly once.
#[derive(Debug, Clone)]
pub struct EvidenceProjectFiles {
    pub project: CapturedFile,
    pub rule_directories: Vec<PathBuf>,
    pub utility_files: Vec<AcquiredFile>,
    pub rule_files: Vec<AcquiredFile>,
}

/// Why an ast-grep project could not be read before decoding.
#[derive(Debug, Clone)]
pub enum ProjectReadFailure {
    Unavailable(Box<ProjectAcquisitionFailure>),
    Refused(ProjectRefusal),
}

fn acquisition_failure(
    failure: SourceAcquisitionFailure,
    message: String,
) -> Box<ProjectAcquisitionFailure> {
    Box::new(ProjectAcquisitionFailure {
        refusal: ProjectRefusal::at(&failure.path, message),
        source: failure,
    })
}

fn acquire(path: &Path) -> AcquiredFile {
    read_definition_file(path).map_err(|failure| {
        let message = failure.message.clone();
        acquisition_failure(failure, message)
    })
}

/// Read the ast-grep project file and every util/rule YAML file it names, once.
pub fn read_evidence_project(
    config_path: &Path,
) -> Result<EvidenceProjectFiles, ProjectReadFailure> {
    let project = acquire(config_path).map_err(ProjectReadFailure::Unavailable)?;
    let settings: ProjectSettings = ast_grep_config::from_str(&project.text).map_err(|error| {
        ProjectReadFailure::Refused(ProjectRefusal::located(
            &DefinitionOrigin::file(config_path),
            error.to_string(),
            error
                .location()
                .map(|location| (location.line(), location.column())),
        ))
    })?;
    let base = config_path.parent().unwrap_or(Path::new("."));
    let collect = |folders: &[PathBuf]| {
        let mut files = Vec::new();
        for folder in folders {
            match yaml_files(&base.join(folder)) {
                Ok(paths) => files.extend(paths.iter().map(|path| acquire(path))),
                Err(failure) => files.push(Err(failure)),
            }
        }
        files
    };
    Ok(EvidenceProjectFiles {
        utility_files: collect(&settings.util_dirs),
        rule_files: collect(&settings.rule_dirs),
        rule_directories: settings
            .rule_dirs
            .iter()
            .map(|folder| base.join(folder))
            .collect(),
        project,
    })
}

// The upstream YAML error type is not re-exported; refusals keep its message and position.
macro_rules! yaml_refusal {
    ($path:expr, $error:expr) => {
        ProjectRefusal::located(
            &DefinitionOrigin::file($path),
            $error.to_string(),
            $error
                .location()
                .map(|location| (location.line(), location.column())),
        )
    };
}

/// Decode one utility YAML document stream exactly as the ast-grep project loader does.
pub fn decode_utility_file(
    file: &CapturedFile,
) -> Result<Vec<SerializableGlobalRule<SupportLang>>, ProjectRefusal> {
    ast_grep_config::from_multiple::<SerializableGlobalRule<SupportLang>>(&file.text)
        .map_err(|error| yaml_refusal!(&file.path, error))
}

/// Decode one rule YAML document stream exactly as the ast-grep project loader does.
pub fn decode_matcher_file(
    file: &CapturedFile,
) -> Result<Vec<ast_grep_config::SerializableRuleConfig<SupportLang>>, ProjectRefusal> {
    ast_grep_config::from_multiple::<ast_grep_config::SerializableRuleConfig<SupportLang>>(
        &file.text,
    )
    .map_err(|error| yaml_refusal!(&file.path, error))
}

/// Decode a read project into definitions, failing at the first unread or malformed file.
pub fn decode_evidence_project(
    files: &EvidenceProjectFiles,
) -> Result<EvidenceProjectDefinitions, ProjectReadFailure> {
    let mut definitions = EvidenceProjectDefinitions {
        project: DefinitionOrigin::file(&files.project.path),
        rule_directories: files.rule_directories.clone(),
        utilities: vec![],
        matchers: vec![],
        inputs: vec![files.project.path.clone()],
    };
    for acquired in &files.utility_files {
        let file = acquired
            .as_ref()
            .map_err(|failure| ProjectReadFailure::Unavailable(failure.clone()))?;
        let origin = DefinitionOrigin::file(&file.path);
        for utility in decode_utility_file(file).map_err(ProjectReadFailure::Refused)? {
            definitions.utilities.push(UtilityDefinition {
                origin: origin.clone(),
                utility,
            });
        }
        definitions.inputs.push(file.path.clone());
    }
    for acquired in &files.rule_files {
        let file = acquired
            .as_ref()
            .map_err(|failure| ProjectReadFailure::Unavailable(failure.clone()))?;
        let origin = DefinitionOrigin::file(&file.path);
        for matcher in decode_matcher_file(file).map_err(ProjectReadFailure::Refused)? {
            definitions.matchers.push(MatcherDefinition {
                origin: origin.clone(),
                matcher,
            });
        }
        definitions.inputs.push(file.path.clone());
    }
    Ok(definitions)
}

/// The one evidence compiler shared by file and in-memory definitions.
struct ProjectCompiler {
    reserved_ids: BTreeSet<&'static str>,
    utils: Vec<SerializableGlobalRule<SupportLang>>,
    globals: GlobalRules,
    rules: Vec<ast_grep_config::RuleConfig<SupportLang>>,
    rule_ids: BTreeSet<crate::EvidenceRuleId>,
    rule_languages: BTreeMap<crate::EvidenceRuleId, EvidenceLanguage>,
}

impl ProjectCompiler {
    fn new(project: &DefinitionOrigin) -> Result<Self, ProjectRefusal> {
        let reserved = crate::document_evidence::standard_markdown_utils::standard_utils();
        let reserved_ids = reserved.iter().map(|(id, _)| *id).collect::<BTreeSet<_>>();
        let mut utils = Vec::new();
        for (_, yaml) in reserved {
            utils.push(
                ast_grep_config::from_str::<SerializableGlobalRule<SupportLang>>(yaml)
                    .map_err(|error| ProjectRefusal::in_definition(project, error))?,
            );
        }
        Ok(Self {
            reserved_ids,
            utils,
            globals: GlobalRules::default(),
            rules: vec![],
            rule_ids: BTreeSet::new(),
            rule_languages: BTreeMap::new(),
        })
    }

    fn add_utility(
        &mut self,
        utility: SerializableGlobalRule<SupportLang>,
        origin: &DefinitionOrigin,
    ) -> Result<(), ProjectRefusal> {
        refuse_reserved_local_utils(&utility.core, &self.reserved_ids, origin)?;
        if self.reserved_ids.contains(utility.id.as_str()) {
            return Err(ProjectRefusal::in_definition(
                origin,
                format!("reserved standard util ID: {}", utility.id),
            ));
        }
        self.utils.push(utility);
        Ok(())
    }

    fn seal_utilities(&mut self, project: &DefinitionOrigin) -> Result<(), ProjectRefusal> {
        self.globals = DeserializeEnv::parse_global_utils(std::mem::take(&mut self.utils))
            .map_err(|error| ProjectRefusal::in_definition(project, error))?;
        Ok(())
    }

    fn add_matcher(
        &mut self,
        wire: ast_grep_config::SerializableRuleConfig<SupportLang>,
        origin: &DefinitionOrigin,
    ) -> Result<(), ProjectRefusal> {
        refuse_reserved_local_utils(&wire.core, &self.reserved_ids, origin)?;
        let rule = ast_grep_config::RuleConfig::try_from(wire, &self.globals)
            .map_err(|error| ProjectRefusal::in_definition(origin, error))?;
        if self.reserved_ids.contains(rule.id.as_str()) {
            return Err(ProjectRefusal::in_definition(
                origin,
                format!("reserved standard util ID: {}", rule.id),
            ));
        }
        let language = EvidenceLanguage::from_support(rule.language);
        let id = crate::EvidenceRuleId::new(rule.id.clone())
            .map_err(|error| ProjectRefusal::in_definition(origin, error))?;
        if !self.rule_ids.insert(id.clone()) {
            return Err(ProjectRefusal::in_definition(
                origin,
                format!("duplicate evidence rule ID: {}", rule.id),
            ));
        }
        self.rule_languages.insert(id, language);
        self.rules.push(rule);
        Ok(())
    }

    fn add_inline_rules(
        &mut self,
        inline_rules: &[InlineRule],
        inline_origin: &DefinitionOrigin,
    ) -> Result<(), ProjectRefusal> {
        for inline in inline_rules {
            let mut compile = || {
                let mut wire: InlineRuleWire = ast_grep_config::from_str(&inline.text)
                    .map_err(|error| ProjectRefusal::in_definition(inline_origin, error))?;
                wire.rule.id = inline.kind.as_str().to_owned();
                self.add_matcher(wire.rule, inline_origin)
            };
            compile().map_err(|mut refusal| {
                refusal.origin = Some(inline.origin.clone());
                refusal
            })?;
        }
        Ok(())
    }

    fn finish(
        self,
        project: &DefinitionOrigin,
        rule_directories: Vec<PathBuf>,
        inputs: Vec<PathBuf>,
        document_settings: &crate::DocumentEvidenceSettings,
    ) -> Result<EvidenceProject, ProjectRefusal> {
        let rules = RuleCollection::try_new(self.rules)
            .map_err(|error| ProjectRefusal::in_definition(project, error))?;
        Ok(EvidenceProject {
            document_settings: document_settings.clone(),
            rules: Arc::new(rules),
            rule_ids: self.rule_ids,
            inputs,
            rule_directories,
            rule_languages: self.rule_languages,
        })
    }
}

/// Compile decoded evidence definitions plus inline kind rules with the shared policy.
pub fn compile_evidence_project(
    definitions: &EvidenceProjectDefinitions,
    inline_rules: &[InlineRule],
    inline_origin: &DefinitionOrigin,
    document_settings: &crate::DocumentEvidenceSettings,
) -> Result<EvidenceProject, ProjectRefusal> {
    let mut compiler = ProjectCompiler::new(&definitions.project)?;
    for utility in &definitions.utilities {
        compiler.add_utility(utility.utility.clone(), &utility.origin)?;
    }
    compiler.seal_utilities(&definitions.project)?;
    for matcher in &definitions.matchers {
        compiler.add_matcher(matcher.matcher.clone(), &matcher.origin)?;
    }
    compiler.add_inline_rules(inline_rules, inline_origin)?;
    let mut inputs = definitions.inputs.clone();
    if !inline_rules.is_empty()
        && let Some(path) = inline_origin.file_path()
    {
        inputs.push(path.to_owned());
    }
    compiler.finish(
        &definitions.project,
        definitions.rule_directories.clone(),
        inputs,
        document_settings,
    )
}

/// Load the consumer project and the embedded Markdown utils into one registration.
///
/// Files are read once and visited in the established order: a decode or compile
/// refusal in an earlier file is reported before a later file's acquisition failure.
pub fn load_evidence_project(
    config_path: &Path,
    inline_rules: &[InlineRule],
    rules_config_path: Option<&Path>,
    document_settings: &crate::DocumentEvidenceSettings,
) -> Result<EvidenceProject, ProjectRefusal> {
    let files = read_evidence_project(config_path).map_err(|failure| match failure {
        ProjectReadFailure::Unavailable(failure) => failure.refusal,
        ProjectReadFailure::Refused(refusal) => refusal,
    })?;
    compile_evidence_project_files(
        &files,
        inline_rules,
        &DefinitionOrigin::file(rules_config_path.unwrap_or(config_path)),
        document_settings,
    )
}

/// Decode and compile a read project file by file, preserving the CLI's refusal order.
pub fn compile_evidence_project_files(
    files: &EvidenceProjectFiles,
    inline_rules: &[InlineRule],
    inline_origin: &DefinitionOrigin,
    document_settings: &crate::DocumentEvidenceSettings,
) -> Result<EvidenceProject, ProjectRefusal> {
    let project = DefinitionOrigin::file(&files.project.path);
    let mut compiler = ProjectCompiler::new(&project)?;
    let mut inputs = vec![files.project.path.clone()];
    for acquired in &files.utility_files {
        let file = acquired
            .as_ref()
            .map_err(|failure| failure.refusal.clone())?;
        let origin = DefinitionOrigin::file(&file.path);
        for utility in decode_utility_file(file)? {
            compiler.add_utility(utility, &origin)?;
        }
        inputs.push(file.path.clone());
    }
    compiler.seal_utilities(&project)?;
    for acquired in &files.rule_files {
        let file = acquired
            .as_ref()
            .map_err(|failure| failure.refusal.clone())?;
        let origin = DefinitionOrigin::file(&file.path);
        for matcher in decode_matcher_file(file)? {
            compiler.add_matcher(matcher, &origin)?;
        }
        inputs.push(file.path.clone());
    }
    compiler.add_inline_rules(inline_rules, inline_origin)?;
    if !inline_rules.is_empty()
        && let Some(path) = inline_origin.file_path()
    {
        inputs.push(path.to_owned());
    }
    compiler.finish(
        &project,
        files.rule_directories.clone(),
        inputs,
        document_settings,
    )
}

fn refuse_reserved_local_utils(
    core: &ast_grep_config::SerializableRuleCore,
    reserved: &BTreeSet<&str>,
    origin: &DefinitionOrigin,
) -> Result<(), ProjectRefusal> {
    if let Some(id) = core
        .utils
        .as_ref()
        .and_then(|utils| utils.keys().find(|id| reserved.contains(id.as_str())))
    {
        return Err(ProjectRefusal::in_definition(
            origin,
            format!("reserved standard util ID: {id}"),
        ));
    }
    Ok(())
}

fn yaml_files(folder: &Path) -> Result<Vec<PathBuf>, Box<ProjectAcquisitionFailure>> {
    let unreadable = |error: std::io::Error| {
        acquisition_failure(
            SourceAcquisitionFailure {
                path: folder.to_path_buf(),
                reason: match error.kind() {
                    std::io::ErrorKind::NotFound => crate::SourceUnavailable::NotFound,
                    std::io::ErrorKind::PermissionDenied => crate::SourceUnavailable::Unreadable,
                    _ => crate::SourceUnavailable::Io,
                },
                message: error.to_string(),
            },
            error.to_string(),
        )
    };
    let mut files = Vec::new();
    let entries = std::fs::read_dir(folder).map_err(unreadable)?;
    for entry in entries {
        let path = entry.map_err(unreadable)?.path();
        if path.is_dir() {
            files.extend(yaml_files(&path)?);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "yml" || ext == "yaml")
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
#[path = "evidence_project_loading_test.rs"]
mod tests;

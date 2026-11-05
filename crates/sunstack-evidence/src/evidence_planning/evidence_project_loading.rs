use ast_grep_config::{DeserializeEnv, RuleCollection, SerializableGlobalRule};
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
    pub path: PathBuf,
    pub message: String,
    pub line: usize,
    pub column: Option<usize>,
}
impl ProjectRefusal {
    fn at(path: &Path, message: impl ToString) -> Self {
        Self {
            path: path.into(),
            message: message.to_string(),
            line: 1,
            column: None,
        }
    }
    fn located(path: &Path, message: impl ToString, position: Option<(u64, u64)>) -> Self {
        let mut refusal = Self::at(path, message);
        if let Some((line, column)) = position {
            refusal.line = usize::try_from(line).unwrap_or(usize::MAX);
            refusal.column = usize::try_from(column).ok();
        }
        refusal
    }
}

#[derive(Clone)]
pub struct EvidenceProject {
    pub rules: Arc<RuleCollection<SupportLang>>,
    pub rule_ids: BTreeSet<crate::EvidenceRuleId>,
    pub inputs: Vec<PathBuf>,
    rule_languages: BTreeMap<crate::EvidenceRuleId, EvidenceLanguage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLanguage {
    Markdown,
    Json,
}
impl EvidenceLanguage {
    fn from_support(language: SupportLang) -> Option<Self> {
        match language {
            SupportLang::Markdown => Some(Self::Markdown),
            SupportLang::Json => Some(Self::Json),
            _ => None,
        }
    }
    fn support(self) -> SupportLang {
        match self {
            Self::Markdown => SupportLang::Markdown,
            Self::Json => SupportLang::Json,
        }
    }
}

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
            .filter_map(|rule| EvidenceLanguage::from_support(rule.language))
            .collect::<BTreeSet<_>>();
        // Keep every tree alive so its node identities cannot be reused by another parse.
        let documents = languages
            .into_iter()
            .map(|language| (language, language.support().ast_grep(text)))
            .collect::<BTreeMap<_, _>>();
        for (language, document) in &documents {
            if *language == EvidenceLanguage::Json && document.root().get_inner_node().has_error() {
                index.failed_languages.insert(*language);
            }
        }
        let mut count = 0usize;
        for rule in rules {
            let Some(language) = EvidenceLanguage::from_support(rule.language) else {
                continue;
            };
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

/// Load the consumer project and the embedded Markdown utils into one registration.
pub fn load_evidence_project(config_path: &Path) -> Result<EvidenceProject, ProjectRefusal> {
    let read = |path: &Path| {
        std::fs::read_to_string(path).map_err(|error| ProjectRefusal::at(path, error))
    };
    let settings: ProjectSettings =
        ast_grep_config::from_str(&read(config_path)?).map_err(|error| {
            ProjectRefusal::located(
                config_path,
                error.to_string(),
                error
                    .location()
                    .map(|location| (location.line(), location.column())),
            )
        })?;
    let base = config_path.parent().unwrap_or(Path::new("."));
    let reserved = crate::document_evidence::standard_markdown_utils::standard_utils();
    let reserved_ids = reserved.iter().map(|(id, _)| *id).collect::<BTreeSet<_>>();
    let mut utils = Vec::new();
    for (_, yaml) in reserved {
        utils.push(
            ast_grep_config::from_str::<SerializableGlobalRule<SupportLang>>(yaml)
                .map_err(|error| ProjectRefusal::at(config_path, error))?,
        );
    }
    let mut inputs = vec![config_path.into()];
    for folder in &settings.util_dirs {
        for path in yaml_files(&base.join(folder))? {
            for util in
                ast_grep_config::from_multiple::<SerializableGlobalRule<SupportLang>>(&read(&path)?)
                    .map_err(|error| {
                        ProjectRefusal::located(
                            &path,
                            error.to_string(),
                            error
                                .location()
                                .map(|location| (location.line(), location.column())),
                        )
                    })?
            {
                refuse_reserved_local_utils(&util.core, &reserved_ids, &path)?;
                if reserved_ids.contains(util.id.as_str()) {
                    return Err(ProjectRefusal::at(
                        &path,
                        format!("reserved standard util ID: {}", util.id),
                    ));
                }
                if EvidenceLanguage::from_support(util.language).is_none() {
                    return Err(ProjectRefusal::at(
                        &path,
                        "only Markdown and JSON evidence is supported",
                    ));
                }
                utils.push(util);
            }
            inputs.push(path);
        }
    }
    let globals = DeserializeEnv::parse_global_utils(utils)
        .map_err(|error| ProjectRefusal::at(config_path, error))?;
    let mut rules = Vec::new();
    let mut rule_ids = BTreeSet::new();
    let mut rule_languages = BTreeMap::new();
    for folder in &settings.rule_dirs {
        for path in yaml_files(&base.join(folder))? {
            let loaded = ast_grep_config::from_multiple::<
                ast_grep_config::SerializableRuleConfig<SupportLang>,
            >(&read(&path)?)
            .map_err(|error| {
                ProjectRefusal::located(
                    &path,
                    error.to_string(),
                    error
                        .location()
                        .map(|location| (location.line(), location.column())),
                )
            })?;
            for wire in loaded {
                refuse_reserved_local_utils(&wire.core, &reserved_ids, &path)?;
                let rule = ast_grep_config::RuleConfig::try_from(wire, &globals)
                    .map_err(|error| ProjectRefusal::at(&path, error))?;
                if reserved_ids.contains(rule.id.as_str()) {
                    return Err(ProjectRefusal::at(
                        &path,
                        format!("reserved standard util ID: {}", rule.id),
                    ));
                }
                if EvidenceLanguage::from_support(rule.language).is_none() {
                    return Err(ProjectRefusal::at(
                        &path,
                        "only Markdown and JSON evidence is supported",
                    ));
                }
                let id = crate::EvidenceRuleId::new(rule.id.clone())
                    .map_err(|error| ProjectRefusal::at(&path, error))?;
                if !rule_ids.insert(id.clone()) {
                    return Err(ProjectRefusal::at(
                        &path,
                        format!("duplicate evidence rule ID: {}", rule.id),
                    ));
                }
                if let Some(language) = EvidenceLanguage::from_support(rule.language) {
                    rule_languages.insert(id, language);
                }
                rules.push(rule);
            }
            inputs.push(path);
        }
    }
    let rules =
        RuleCollection::try_new(rules).map_err(|error| ProjectRefusal::at(config_path, error))?;
    Ok(EvidenceProject {
        rules: Arc::new(rules),
        rule_ids,
        inputs,
        rule_languages,
    })
}

fn refuse_reserved_local_utils(
    core: &ast_grep_config::SerializableRuleCore,
    reserved: &BTreeSet<&str>,
    path: &Path,
) -> Result<(), ProjectRefusal> {
    if let Some(id) = core
        .utils
        .as_ref()
        .and_then(|utils| utils.keys().find(|id| reserved.contains(id.as_str())))
    {
        return Err(ProjectRefusal::at(
            path,
            format!("reserved standard util ID: {id}"),
        ));
    }
    Ok(())
}

fn yaml_files(folder: &Path) -> Result<Vec<PathBuf>, ProjectRefusal> {
    let mut files = Vec::new();
    let entries = std::fs::read_dir(folder).map_err(|error| ProjectRefusal::at(folder, error))?;
    for entry in entries {
        let path = entry
            .map_err(|error| ProjectRefusal::at(folder, error))?
            .path();
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

use ast_grep_config::{DeserializeEnv, RuleCollection, SerializableGlobalRule};
use ast_grep_language::{LanguageExt, SupportLang};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
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

pub struct EvidenceProject {
    pub rules: RuleCollection<SupportLang>,
    pub rule_ids: BTreeSet<crate::EvidenceRuleId>,
    pub inputs: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MatchRange {
    pub start: usize,
    pub end: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMatchIndex {
    pub by_rule: BTreeMap<crate::EvidenceRuleId, Vec<MatchRange>>,
    pub limited: bool,
}

impl EvidenceProject {
    pub fn match_file(&self, path: &Path, text: &str, max_matches: usize) -> FileMatchIndex {
        let document = SupportLang::Markdown.ast_grep(text);
        let mut index = FileMatchIndex::default();
        let mut count = 0usize;
        for rule in self.rules.for_path(path) {
            let mut spans = Vec::new();
            for found in document.root().find_all(&rule.matcher) {
                if count >= max_matches {
                    index.limited = true;
                    break;
                }
                let range = found.range();
                spans.push(MatchRange {
                    start: range.start,
                    end: range.end,
                });
                count = count.saturating_add(1);
            }
            spans.sort_by_key(|span| (span.start, span.end));
            if let Ok(id) = crate::EvidenceRuleId::new(rule.id.clone()) {
                index.by_rule.insert(id, spans);
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
    let reserved = crate::standard_markdown_utils::standard_utils();
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
                if util.language != SupportLang::Markdown {
                    return Err(ProjectRefusal::at(
                        &path,
                        "only Markdown evidence is supported",
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
                if rule.language != SupportLang::Markdown {
                    return Err(ProjectRefusal::at(
                        &path,
                        "only Markdown evidence is supported",
                    ));
                }
                let id = crate::EvidenceRuleId::new(rule.id.clone())
                    .map_err(|error| ProjectRefusal::at(&path, error))?;
                if !rule_ids.insert(id) {
                    return Err(ProjectRefusal::at(
                        &path,
                        format!("duplicate evidence rule ID: {}", rule.id),
                    ));
                }
                rules.push(rule);
            }
            inputs.push(path);
        }
    }
    let rules =
        RuleCollection::try_new(rules).map_err(|error| ProjectRefusal::at(config_path, error))?;
    Ok(EvidenceProject {
        rules,
        rule_ids,
        inputs,
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

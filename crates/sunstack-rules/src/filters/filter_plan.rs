use crate::AdmissionRefusal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};
use sunstack_decisions::{ChoiceLabel, NamedQuestionMap};
use sunstack_evidence::{ConfigPath, EvidenceKindDefinition, EvidenceKindName, FieldName};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct FilterName(String);
impl FilterName {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.is_empty() {
            return Err("filter name must not be empty");
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for FilterName {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<FilterName> for String {
    fn from(value: FilterName) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadsSpec {
    pub file: PathBuf,
    pub kind: EvidenceKindName,
    pub fields: Vec<FieldName>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FilterWire {
    #[schemars(length(min = 1))]
    pub per: String,
    pub reads: ReadsSpec,
    pub labels: Vec<ChoiceLabel>,
    #[schemars(schema_with = "filter_questions_schema")]
    pub questions: NamedQuestionMap,
    pub decision: crate::DecisionGraphWire,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reference: crate::ReferenceMaterial,
}
#[derive(Debug, Clone)]
pub struct FolderPattern {
    segments: Vec<String>,
}
impl FolderPattern {
    pub fn new(value: &str) -> Result<Self, &'static str> {
        let segments = value.split('/').map(String::from).collect::<Vec<_>>();
        if segments
            .iter()
            .filter(|segment| segment.as_str() == "{name}")
            .count()
            != 1
            || segments.iter().any(|segment| {
                segment.is_empty()
                    || segment == "."
                    || segment == ".."
                    || (segment != "*"
                        && segment != "{name}"
                        && segment.contains(['*', '?', '[', ']', '{', '}', '\\']))
            })
        {
            return Err(
                "per requires exactly one {name} segment and literal or * segments; ** is not allowed",
            );
        }
        Ok(Self { segments })
    }
    pub fn folder_for(&self, file: &Path) -> Option<(PathBuf, String)> {
        let components = file
            .components()
            .map(|part| match part {
                Component::Normal(part) => part.to_str(),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        if components.len() <= self.segments.len() {
            return None;
        }
        let mut display = None;
        for (expected, actual) in self.segments.iter().zip(&components) {
            match expected.as_str() {
                "{name}" => display = Some((*actual).to_owned()),
                "*" => {}
                literal if literal == *actual => {}
                _ => return None,
            }
        }
        Some((components[..self.segments.len()].iter().collect(), display?))
    }
}
#[derive(Debug, Clone)]
pub struct FilterPlan {
    pub name: FilterName,
    pub per: FolderPattern,
    pub reads: ReadsSpec,
    pub questions: NamedQuestionMap,
    pub decision: crate::DecisionGraph,
    pub reference: crate::ReferenceMaterial,
    pub answers: BTreeSet<ChoiceLabel>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OnlyWhenGate {
    pub filter: FilterName,
    pub answers: BTreeSet<ChoiceLabel>,
}
pub type FilterRegistry = BTreeMap<FilterName, FilterPlan>;
pub type OnlyWhenWire = BTreeMap<FilterName, Vec<ChoiceLabel>>;

#[cfg(test)]
#[path = "filter_plan_test.rs"]
mod tests;

pub fn admit_filters(
    wires: &BTreeMap<FilterName, FilterWire>,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    project: &sunstack_evidence::EvidenceProject,
) -> Result<FilterRegistry, Vec<AdmissionRefusal>> {
    use crate::rulebook_planning::rulebook_admission::{
        RESERVED_STATE_KEYS, admit_graph, admit_questions, question_state_paths,
    };
    let mut plans = BTreeMap::new();
    let mut errors = Vec::new();
    for (name, wire) in wires {
        let path = ConfigPath::keys(["filters", name.as_str()]);
        let start_errors = errors.len();
        let pattern = match FolderPattern::new(&wire.per) {
            Ok(value) => Some(value),
            Err(message) => {
                errors.push(AdmissionRefusal {
                    field_path: path.with_key("per"),
                    message: message.into(),
                });
                None
            }
        };
        if wire.reads.file.as_os_str().is_empty()
            || wire
                .reads
                .file
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            errors.push(AdmissionRefusal {
                field_path: path.with_key("reads").with_key("file"),
                message: "reads.file must be a nonempty relative file inside the folder".into(),
            });
        }
        let labels = wire.labels.iter().cloned().collect::<BTreeSet<_>>();
        if labels.is_empty()
            || labels.len() != wire.labels.len()
            || labels.iter().any(|label| {
                ["unsure", "uncertain", "pass", "not_applicable"].contains(&label.as_str())
            })
        {
            errors.push(AdmissionRefusal {
                field_path: path.with_key("labels"),
                message: "labels must be unique, nonempty and distinct from reserved tree endings"
                    .into(),
            });
        }
        let fields = wire.reads.fields.iter().cloned().collect::<BTreeSet<_>>();
        if fields.is_empty() || fields.len() != wire.reads.fields.len() {
            errors.push(AdmissionRefusal {
                field_path: path.with_key("reads").with_key("fields"),
                message: "reads.fields must be nonempty and unique".into(),
            });
        }
        let Some(kind) = kinds.get(&wire.reads.kind) else {
            errors.push(AdmissionRefusal {
                field_path: path.with_key("reads").with_key("kind"),
                message: "undefined evidence kind".into(),
            });
            continue;
        };
        if kind.select.is_some() || kind.within.is_some() {
            errors.push(AdmissionRefusal {field_path:path.with_key("reads").with_key("kind"),message:"filter reads must be a directly collected kind, not selected or within another kind".into()});
        }
        for field in &fields {
            if !kind
                .fields
                .get(field)
                .is_some_and(|source| source.shape() == sunstack_evidence::FieldShape::Text)
                || RESERVED_STATE_KEYS.contains(&field.as_str())
            {
                errors.push(AdmissionRefusal {
                    field_path: path.with_key("reads").with_key("fields"),
                    message: format!(
                        "filter field must name declared text and cannot be reserved: {}",
                        field.as_str()
                    ),
                });
            }
        }
        for key in wire.reference.keys() {
            if RESERVED_STATE_KEYS.contains(&key.as_str())
                || kind.fields.keys().any(|field| field.as_str() == key)
            {
                errors.push(AdmissionRefusal {
                    field_path: path.with_key("reference").with_key(key),
                    message: "reference name clashes with a reserved state key or evidence field"
                        .into(),
                });
            }
        }
        let mut declared = kind.clone();
        declared.fields.retain(|field, _| fields.contains(field));
        admit_questions(
            &wire.questions,
            &declared,
            &wire.reference,
            &path.with_key("questions"),
            &mut errors,
        );
        if wire.questions.is_empty() {
            errors.push(AdmissionRefusal {
                field_path: path.with_key("questions"),
                message: "filter tree requires named questions".into(),
            });
        }
        if question_state_paths(&wire.questions)
            .iter()
            .any(|field| field == "standard" || field == "where" || field.starts_with("where."))
        {
            errors.push(AdmissionRefusal {
                field_path: path.with_key("questions"),
                message: "filters cannot read a standard, file name or folder location".into(),
            });
        }
        for (question, definition) in &wire.questions {
            if let sunstack_decisions::JudgmentQuestion::Choice { criteria, .. } = definition
                && criteria.len() > sunstack_decisions::MAX_CHOICE_OPTIONS
            {
                let refusal = sunstack_decisions::ChoiceOptionsExceeded {
                    options: criteria.len(),
                    limit: sunstack_decisions::MAX_CHOICE_OPTIONS,
                };
                errors.push(AdmissionRefusal {
                    field_path: path
                        .with_key("questions")
                        .with_key(question.as_str())
                        .with_key("criteria"),
                    message: refusal.to_string(),
                });
            }
        }
        let graph_errors_start = errors.len();
        let graph = admit_graph(
            &wire.decision,
            &wire.questions,
            &declared,
            project,
            name.as_str(),
            &mut errors,
            Some(&labels),
        );
        for error in &mut errors[graph_errors_start..] {
            error.field_path = ConfigPath::new(
                path.segments()
                    .iter()
                    .chain(error.field_path.segments())
                    .cloned(),
            );
        }
        if errors.len() == start_errors
            && let Some(per) = pattern
        {
            plans.insert(
                name.clone(),
                FilterPlan {
                    name: name.clone(),
                    per,
                    reads: wire.reads.clone(),
                    questions: wire.questions.clone(),
                    decision: graph,
                    reference: wire.reference.clone(),
                    answers: labels,
                },
            );
        }
    }
    if errors.is_empty() {
        Ok(plans)
    } else {
        Err(errors)
    }
}

pub fn admit_only_when(
    wire: &OnlyWhenWire,
    filters: &FilterRegistry,
) -> Result<OnlyWhenGate, AdmissionRefusal> {
    let refusal = |message: &str| AdmissionRefusal {
        field_path: ConfigPath::keys(["only_when"]),
        message: message.into(),
    };
    if wire.len() != 1 {
        return Err(refusal("only_when must name exactly one filter"));
    }
    let (filter, answers) = wire
        .iter()
        .next()
        .ok_or_else(|| refusal("only_when must name one filter"))?;
    let plan = filters
        .get(filter)
        .ok_or_else(|| refusal("only_when names an unknown filter"))?;
    let unique = answers.iter().cloned().collect::<BTreeSet<_>>();
    if unique.is_empty() || unique.len() != answers.len() || !unique.is_subset(&plan.answers) {
        return Err(refusal(
            "only_when answers must be nonempty, unique and defined by the filter",
        ));
    }
    Ok(OnlyWhenGate {
        filter: filter.clone(),
        answers: unique,
    })
}

fn filter_questions_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    let mut schema = <NamedQuestionMap as JsonSchema>::json_schema(generator);
    if let Some(question) = schema
        .as_object_mut()
        .and_then(|object| object.get_mut("additionalProperties"))
    {
        *question = serde_json::json!({"allOf":[question.clone(),
            {"if":{"properties":{"type":{"const":"choice"}},"required":["type"]},"then":{"properties":{"criteria":{"minProperties":2}}}},
            {"if":{"properties":{"type":{"const":"score"}},"required":["type"]},"then":{"properties":{"criteria":{"minItems":2}}}}
        ]});
    }
    schema
}

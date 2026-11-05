use crate::AdmissionRefusal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};
use sunstack_decisions::{ChoiceLabel, JudgmentQuestion, QuestionName};
use sunstack_evidence::{
    ConfigPath, EvidenceKindDefinition, EvidenceKindName, FieldName, FieldShape,
};

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
    pub field: FieldName,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FilterWire {
    #[schemars(length(min = 1))]
    pub per: String,
    pub reads: ReadsSpec,
    #[schemars(extend("properties"={"type":{"const":"choice"},"criteria":{"minProperties":2}}))]
    pub question: JudgmentQuestion,
    #[schemars(extend("exclusiveMinimum"=0,"maximum"=1))]
    pub min_probability: f64,
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
    pub question: JudgmentQuestion,
    pub answers: BTreeSet<ChoiceLabel>,
    pub min_probability: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OnlyWhenGate {
    pub filter: FilterName,
    pub answers: BTreeSet<ChoiceLabel>,
}
pub type FilterRegistry = BTreeMap<FilterName, FilterPlan>;
pub type OnlyWhenWire = BTreeMap<FilterName, Vec<ChoiceLabel>>;

pub fn admit_filters(
    wires: &BTreeMap<FilterName, FilterWire>,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
) -> Result<FilterRegistry, Vec<AdmissionRefusal>> {
    let mut plans = BTreeMap::new();
    let mut errors = Vec::new();
    for (name, wire) in wires {
        let path = ConfigPath::keys(["filters", name.as_str()]);
        let mut refuse = |field: &str, message: &str| {
            errors.push(AdmissionRefusal {
                field_path: if field.is_empty() {
                    path.clone()
                } else {
                    field
                        .split('.')
                        .fold(path.clone(), |path, key| path.with_key(key))
                },
                message: message.into(),
            })
        };
        let pattern = FolderPattern::new(&wire.per);
        if let Err(message) = &pattern {
            refuse("per", message);
        }
        if wire.reads.file.as_os_str().is_empty()
            || wire
                .reads
                .file
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            refuse(
                "reads.file",
                "reads.file must be a nonempty relative file inside the folder",
            );
        }
        match kinds.get(&wire.reads.kind) {
            None => refuse("reads.kind", "undefined evidence kind"),
            Some(kind) => {
                if kind.select.is_some() || kind.within.is_some() {
                    refuse(
                        "reads.kind",
                        "reads.kind must have neither select nor within",
                    );
                }
                if !kind
                    .fields
                    .get(&wire.reads.field)
                    .is_some_and(|field| field.shape() == FieldShape::Text)
                {
                    refuse("reads.field", "reads.field must name one text field");
                }
            }
        }
        let answers = match &wire.question {
            JudgmentQuestion::Choice { criteria, .. } if criteria.len() >= 2 => {
                criteria.keys().cloned().collect()
            }
            _ => {
                refuse(
                    "question",
                    "filter question must be choice with at least two criteria",
                );
                BTreeSet::new()
            }
        };
        if let Ok(question_name) = QuestionName::new("filter") {
            let questions = BTreeMap::from([(question_name, wire.question.clone())]);
            if sunstack_decisions::judgment_processing::judgment_validation::validate_questions(
                &questions,
            )
            .is_err()
            {
                refuse(
                    "question",
                    "filter question rejected by decision model schema",
                );
            }
        }
        if !wire.min_probability.is_finite()
            || wire.min_probability <= 0.0
            || wire.min_probability > 1.0
        {
            refuse("min_probability", "min_probability must be in (0, 1]");
        }
        if let Ok(per) = pattern {
            plans.insert(
                name.clone(),
                FilterPlan {
                    name: name.clone(),
                    per,
                    reads: wire.reads.clone(),
                    question: wire.question.clone(),
                    answers,
                    min_probability: wire.min_probability,
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

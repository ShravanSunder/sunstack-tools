use crate::Severity;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sunstack_decisions::{ChoiceLabel, QuestionName};
use sunstack_evidence::{EvidenceRuleId, FieldName};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct StepName(String);
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("step name is empty or reserved")]
pub struct InvalidStepName;
impl StepName {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidStepName> {
        let value = value.into();
        if value.is_empty() || ["pass", "not_applicable", "uncertain"].contains(&value.as_str()) {
            Err(InvalidStepName)
        } else {
            Ok(Self(value))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for StepName {
    type Error = InvalidStepName;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<StepName> for String {
    fn from(value: StepName) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RouteWire {
    Named(String),
    Finding(FindingRoute),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindingRoute {
    pub finding: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<FieldName>,
}
#[derive(Debug, Clone)]
pub enum DecisionRoute {
    Step(StepName),
    Pass,
    NotApplicable,
    Uncertain,
    Finding(FindingRoute),
}
impl TryFrom<RouteWire> for DecisionRoute {
    type Error = InvalidStepName;
    fn try_from(route: RouteWire) -> Result<Self, Self::Error> {
        Ok(match route {
            RouteWire::Finding(finding) => Self::Finding(finding),
            RouteWire::Named(name) => match name.as_str() {
                "pass" => Self::Pass,
                "not_applicable" => Self::NotApplicable,
                "uncertain" => Self::Uncertain,
                _ => Self::Step(StepName::new(name)?),
            },
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionMechanicalTool {
    AstGrep { rule: EvidenceRuleId },
    Regex { field: FieldName, pattern: String },
    Contains { field: FieldName, text: String },
    MaxChars { field: FieldName, max: usize },
    FileExists { field: FieldName },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecisionGraphWire {
    pub start: StepName,
    pub steps: BTreeMap<StepName, DecisionStepWire>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DecisionStepWire {
    Mechanical(MechanicalStepWire),
    AskBinary(AskBinaryWire),
    AskChoice(AskChoiceWire),
    Combine(CombineStepWire),
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MechanicalStepWire {
    pub mechanical_check: DecisionMechanicalTool,
    pub yes: RouteWire,
    pub no: RouteWire,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AskBinaryWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertain: Option<RouteWire>,
    pub ask: QuestionName,
    pub yes_at_or_above: f64,
    pub no_at_or_below: f64,
    pub yes: RouteWire,
    pub no: RouteWire,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AskChoiceWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertain: Option<RouteWire>,
    pub ask: QuestionName,
    pub min_probability: f64,
    pub routes: BTreeMap<ChoiceLabel, RouteWire>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CombineStepWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertain: Option<RouteWire>,
    pub combine: BTreeMap<QuestionName, f64>,
    pub yes_at_or_above: f64,
    pub no_at_or_below: f64,
    pub yes: RouteWire,
    pub no: RouteWire,
}

#[derive(Debug, Clone)]
pub struct DecisionGraph {
    pub start: StepName,
    pub steps: BTreeMap<StepName, DecisionStep>,
}
#[derive(Debug, Clone)]
pub struct BinaryRoutes {
    pub uncertain: DecisionRoute,
    pub yes_at_or_above: f64,
    pub no_at_or_below: f64,
    pub yes: DecisionRoute,
    pub no: DecisionRoute,
}
#[derive(Debug, Clone)]
pub enum DecisionStep {
    Mechanical {
        tool: DecisionMechanicalTool,
        yes: DecisionRoute,
        no: DecisionRoute,
    },
    AskNoul {
        question: QuestionName,
        routes: BinaryRoutes,
    },
    AskScore {
        question: QuestionName,
        levels: usize,
        routes: BinaryRoutes,
    },
    AskChoice {
        uncertain: DecisionRoute,
        question: QuestionName,
        min_probability: f64,
        routes: BTreeMap<ChoiceLabel, DecisionRoute>,
    },
    Combine {
        weights: BTreeMap<QuestionName, f64>,
        routes: BinaryRoutes,
    },
}
impl DecisionStep {
    pub fn routes(&self) -> Vec<&DecisionRoute> {
        match self {
            Self::Mechanical { yes, no, .. } => vec![yes, no],
            Self::AskNoul { routes, .. }
            | Self::AskScore { routes, .. }
            | Self::Combine { routes, .. } => vec![&routes.yes, &routes.no, &routes.uncertain],
            Self::AskChoice {
                routes, uncertain, ..
            } => routes.values().chain(std::iter::once(uncertain)).collect(),
        }
    }
}

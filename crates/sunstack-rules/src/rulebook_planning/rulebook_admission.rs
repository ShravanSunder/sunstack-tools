use crate::{
    BinaryRoutes, DecisionGraph, DecisionGraphWire, DecisionMechanicalTool, DecisionRoute,
    DecisionStep, DecisionStepWire, RuleEntryWire,
};
use std::collections::{BTreeMap, BTreeSet};
use sunstack_decisions::{
    JudgmentQuestion, NamedQuestionMap, QuestionName, StructuredEntry, StructuredJson,
};
use sunstack_evidence::{
    ConfigPath, EvidenceKindDefinition, EvidenceKindName, EvidenceProject, FieldName, FieldShape,
    FieldSource, MatchSource,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRefusal {
    pub field_path: ConfigPath,
    pub message: String,
}
fn refusal(path: impl Into<ConfigPath>, message: impl Into<String>) -> AdmissionRefusal {
    AdmissionRefusal {
        field_path: path.into(),
        message: message.into(),
    }
}

pub fn admit_evidence_kinds(
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    project: &EvidenceProject,
    default_max_chars: usize,
) -> Vec<AdmissionRefusal> {
    let mut errors = Vec::new();
    if default_max_chars < 1000 {
        errors.push(refusal(
            ConfigPath::keys(["limits", "evidence_max_chars"]),
            "must be at least 1000",
        ));
    }
    for (name, kind) in kinds {
        let path = ConfigPath::keys(["evidence", "kinds", name.as_str()]);
        if !project.rule_ids.contains(&kind.rule) {
            errors.push(refusal(path.with_key("rule"), "unknown evidence rule"));
        }
        if kind.max_chars.is_some_and(|n| n.get() < 1000) {
            errors.push(refusal(path.with_key("max_chars"), "must be at least 1000"));
        }
        for (field, source) in &kind.fields {
            let path = path.with_key("fields").with_key(field.as_str());
            match source {
                FieldSource::Inside { of }
                | FieldSource::Enclosing { of }
                | FieldSource::SameFile { of }
                | FieldSource::Remainder { exclude: Some(of) } => match of {
                    MatchSource::Rule { rule } if !project.rule_ids.contains(rule) => {
                        errors.push(refusal(&path, "unknown evidence rule"))
                    }
                    MatchSource::Kind { kind } if !kinds.contains_key(kind) => {
                        errors.push(refusal(&path, "undefined evidence kind"))
                    }
                    _ => {}
                },
                FieldSource::Capture { regex } | FieldSource::Target { regex, .. } => {
                    match regex::Regex::new(regex) {
                        Err(_) => errors.push(refusal(&path, "invalid field regex")),
                        Ok(pattern) if pattern.captures_len() < 2 => {
                            errors.push(refusal(&path, "field regex needs a capture group"))
                        }
                        Ok(_) => {}
                    }
                    if let FieldSource::Target {
                        as_kind: Some(target_kind),
                        ..
                    } = source
                    {
                        match kinds.get(target_kind) {
                            None => errors.push(refusal(&path, "as_kind names an undefined kind")),
                            Some(target)
                                if target
                                    .fields
                                    .values()
                                    .any(|s| matches!(s, FieldSource::Target { .. })) =>
                            {
                                errors.push(refusal(&path, "as_kind must name a target-free kind"))
                            }
                            Some(_) => {}
                        }
                    }
                }
                FieldSource::FirstLine {}
                | FieldSource::Yaml { .. }
                | FieldSource::Remainder { exclude: None } => {}
            }
        }
    }
    if let Err(refusals) = sunstack_evidence::EvidenceKindPlan::derive(kinds) {
        errors.extend(
            refusals
                .into_iter()
                .map(|error| refusal(error.field, error.message)),
        );
    }
    if let Err(refusals) = admit_selections(kinds) {
        errors.extend(refusals);
    }

    errors
}

pub fn admit_rule_entry(
    entry: &RuleEntryWire,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    project: &EvidenceProject,
) -> Result<DecisionGraph, Vec<AdmissionRefusal>> {
    let Some(kind) = kinds.get(&entry.evidence) else {
        return Err(vec![refusal(
            ConfigPath::keys(["evidence"]),
            "undefined evidence kind",
        )]);
    };
    let mut errors = Vec::new();
    if !entry.questions.is_empty() {
        if sunstack_decisions::judgment_processing::judgment_validation::validate_questions(
            &entry.questions,
        )
        .is_err()
        {
            errors.push(refusal(
                ConfigPath::keys(["questions"]),
                "question rejected by decision model schema",
            ));
        }
        for (name, question) in &entry.questions {
            let path = ConfigPath::keys(["questions", name.as_str()]);
            let instructions = match question {
                JudgmentQuestion::Choice {
                    instructions,
                    criteria,
                } => {
                    if criteria.len() < 2 {
                        errors.push(refusal(&path, "choice requires at least two options"));
                    }
                    instructions
                }
                JudgmentQuestion::Score {
                    instructions,
                    criteria,
                } => {
                    if criteria.len() < 2 {
                        errors.push(refusal(&path, "score requires at least two levels"));
                    }
                    instructions
                }
                JudgmentQuestion::Noul { instructions, .. } => instructions,
            };
            if let StructuredEntry::Object(instructions) = instructions {
                for (key, value) in instructions {
                    if ["check", "against", "context"].contains(&key.as_str())
                        && !field_references_valid(value, kind)
                    {
                        errors.push(refusal(
                            path.with_key("instructions").with_key(key),
                            "instruction must name defined fields",
                        ));
                    }
                }
            }
        }
    }
    let graph = admit_graph(
        &entry.decision,
        &entry.questions,
        kind,
        project,
        &mut errors,
    );
    if errors.is_empty() {
        Ok(graph)
    } else {
        Err(errors)
    }
}

fn field_references_valid(value: &StructuredJson, kind: &EvidenceKindDefinition) -> bool {
    let contains = |name: &str| {
        FieldName::new(name)
            .ok()
            .is_some_and(|name| kind.fields.contains_key(&name))
    };
    match value {
        StructuredJson::Text(name) => contains(name),
        StructuredJson::Array(names) => names
            .iter()
            .all(|name| matches!(name, StructuredJson::Text(name) if contains(name))),
        _ => false,
    }
}

pub(crate) fn question_field_references(questions: &NamedQuestionMap) -> BTreeSet<FieldName> {
    let mut fields = BTreeSet::new();
    for question in questions.values() {
        let instructions = match question {
            JudgmentQuestion::Noul { instructions, .. }
            | JudgmentQuestion::Choice { instructions, .. }
            | JudgmentQuestion::Score { instructions, .. } => instructions,
        };
        if let StructuredEntry::Object(instructions) = instructions {
            for key in ["check", "against", "context"] {
                let values = match instructions.get(key) {
                    Some(StructuredJson::Text(name)) => vec![name.as_str()],
                    Some(StructuredJson::Array(names)) => names
                        .iter()
                        .filter_map(|name| {
                            if let StructuredJson::Text(name) = name {
                                Some(name.as_str())
                            } else {
                                None
                            }
                        })
                        .collect(),
                    _ => vec![],
                };
                for name in values {
                    if let Ok(field) = FieldName::new(name) {
                        fields.insert(field);
                    }
                }
            }
        }
    }
    fields
}

fn admit_graph(
    wire: &DecisionGraphWire,
    questions: &NamedQuestionMap,
    kind: &EvidenceKindDefinition,
    project: &EvidenceProject,
    errors: &mut Vec<AdmissionRefusal>,
) -> DecisionGraph {
    let mut steps = BTreeMap::new();
    for (name, step) in &wire.steps {
        let path = ConfigPath::keys(["decision", "steps", name.as_str()]);
        let mut route = |route: &crate::RouteWire| match DecisionRoute::try_from(route.clone()) {
            Ok(route) => {
                if let DecisionRoute::Finding(finding) = &route
                    && finding
                        .at
                        .as_ref()
                        .is_some_and(|field| !kind.fields.contains_key(field))
                {
                    errors.push(refusal(&path, "finding at names an undefined field"));
                }
                route
            }
            Err(message) => {
                errors.push(refusal(&path, message.to_string()));
                DecisionRoute::Uncertain
            }
        };
        let admitted = match step {
            DecisionStepWire::Mechanical(step) => {
                let yes = route(&step.yes);
                let no = route(&step.no);
                validate_tool(&step.mechanical_check, kind, project, &path, errors);
                DecisionStep::Mechanical {
                    tool: step.mechanical_check.clone(),
                    yes,
                    no,
                }
            }
            DecisionStepWire::AskBinary(step) => {
                let yes = route(&step.yes);
                let no = route(&step.no);
                validate_thresholds(step.yes_at_or_above, step.no_at_or_below, &path, errors);
                let routes = BinaryRoutes {
                    yes_at_or_above: step.yes_at_or_above,
                    no_at_or_below: step.no_at_or_below,
                    yes,
                    no,
                };
                match questions.get(&step.ask) {
                    Some(JudgmentQuestion::Noul { .. }) => DecisionStep::AskNoul {
                        question: step.ask.clone(),
                        routes,
                    },
                    Some(JudgmentQuestion::Score { criteria, .. }) => DecisionStep::AskScore {
                        question: step.ask.clone(),
                        levels: criteria.len(),
                        routes,
                    },
                    _ => {
                        errors.push(refusal(
                            &path,
                            "binary ask requires a defined noul or score question",
                        ));
                        continue;
                    }
                }
            }
            DecisionStepWire::AskChoice(step) => {
                let routes = step
                    .routes
                    .iter()
                    .map(|(label, value)| (label.clone(), route(value)))
                    .collect::<BTreeMap<_, _>>();
                if !step.min_probability.is_finite()
                    || step.min_probability <= 0.0
                    || step.min_probability > 1.0
                {
                    errors.push(refusal(&path, "min_probability must be in (0, 1]"));
                }
                match questions.get(&step.ask) {
                    Some(JudgmentQuestion::Choice { criteria, .. })
                        if criteria.keys().eq(routes.keys()) => {}
                    _ => errors.push(refusal(
                        &path,
                        "choice requires a defined choice question and exactly its option routes",
                    )),
                }
                DecisionStep::AskChoice {
                    question: step.ask.clone(),
                    min_probability: step.min_probability,
                    routes,
                }
            }
            DecisionStepWire::Combine(step) => {
                let yes = route(&step.yes);
                let no = route(&step.no);
                validate_thresholds(step.yes_at_or_above, step.no_at_or_below, &path, errors);
                if step.combine.is_empty()
                    || step.combine.values().any(|w| !w.is_finite() || *w <= 0.0)
                    || (step.combine.values().sum::<f64>() - 1.0).abs() > 1e-9
                {
                    errors.push(refusal(
                        &path,
                        "combine weights must be positive and sum to one",
                    ));
                }
                for question in step.combine.keys() {
                    if !matches!(
                        questions.get(question),
                        Some(JudgmentQuestion::Noul { .. } | JudgmentQuestion::Score { .. })
                    ) {
                        errors.push(refusal(
                            &path,
                            "combine requires defined noul or score questions",
                        ));
                    }
                }
                DecisionStep::Combine {
                    weights: step.combine.clone(),
                    routes: BinaryRoutes {
                        yes_at_or_above: step.yes_at_or_above,
                        no_at_or_below: step.no_at_or_below,
                        yes,
                        no,
                    },
                }
            }
        };
        steps.insert(name.clone(), admitted);
    }
    if !steps.contains_key(&wire.start) {
        errors.push(refusal(
            ConfigPath::keys(["decision", "start"]),
            "start names a missing step",
        ));
    }
    for step in steps.values() {
        for route in step.routes() {
            if let DecisionRoute::Step(name) = route
                && !steps.contains_key(name)
            {
                errors.push(refusal(
                    ConfigPath::keys(["decision", "steps"]),
                    "route names a missing step",
                ));
            }
        }
    }
    let graph = DecisionGraph {
        start: wire.start.clone(),
        steps,
    };
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    // Refuse unreachable cycles too: every authored step must be well-formed.
    for name in graph.steps.keys() {
        if has_cycle(name, &graph, &mut visiting, &mut visited) {
            errors.push(refusal(
                ConfigPath::keys(["decision", "steps"]),
                "decision graph contains a cycle",
            ));
            break;
        }
    }
    graph
}

fn has_cycle(
    name: &crate::StepName,
    graph: &DecisionGraph,
    visiting: &mut BTreeSet<crate::StepName>,
    visited: &mut BTreeSet<crate::StepName>,
) -> bool {
    if visited.contains(name) {
        return false;
    }
    if !visiting.insert(name.clone()) {
        return true;
    }
    if let Some(step) = graph.steps.get(name) {
        for route in step.routes() {
            if let DecisionRoute::Step(next) = route
                && has_cycle(next, graph, visiting, visited)
            {
                return true;
            }
        }
    }
    visiting.remove(name);
    visited.insert(name.clone());
    false
}
fn validate_thresholds(yes: f64, no: f64, path: &ConfigPath, errors: &mut Vec<AdmissionRefusal>) {
    if !yes.is_finite()
        || !no.is_finite()
        || !(0.0..=1.0).contains(&yes)
        || !(0.0..=1.0).contains(&no)
        || yes <= no
    {
        errors.push(refusal(
            path,
            "thresholds must satisfy 0 <= no_at_or_below < yes_at_or_above <= 1",
        ));
    }
}
fn validate_tool(
    tool: &DecisionMechanicalTool,
    kind: &EvidenceKindDefinition,
    project: &EvidenceProject,
    path: &ConfigPath,
    errors: &mut Vec<AdmissionRefusal>,
) {
    let (field, shapes): (&FieldName, &[FieldShape]) = match tool {
        DecisionMechanicalTool::AstGrep { rule } => {
            if !project.rule_ids.contains(rule) {
                errors.push(refusal(path, "ast_grep names an unknown rule"));
            }
            return;
        }
        DecisionMechanicalTool::Regex { field, pattern } => {
            if regex::Regex::new(pattern).is_err() {
                errors.push(refusal(path, "invalid mechanical regex"));
            }
            (field, &[FieldShape::Text, FieldShape::Texts])
        }
        DecisionMechanicalTool::Contains { field, .. } => {
            (field, &[FieldShape::Text, FieldShape::Texts])
        }
        DecisionMechanicalTool::MaxChars { field, .. } => (field, &[FieldShape::Text]),
        DecisionMechanicalTool::FileExists { field } => (field, &[FieldShape::Targets]),
    };
    if !kind
        .fields
        .get(field)
        .is_some_and(|source| shapes.contains(&source.shape()))
    {
        errors.push(refusal(
            path,
            "mechanical tool names an undefined or incompatible field",
        ));
    }
}

pub(crate) fn normalized_answer(
    question: &QuestionName,
    questions: &NamedQuestionMap,
    answers: &sunstack_decisions::NamedAnswerMap,
) -> Option<f64> {
    match (questions.get(question)?, answers.get(question)?) {
        (
            JudgmentQuestion::Noul { .. },
            sunstack_decisions::JudgmentAnswer::Noul { yes_probability },
        ) => Some(*yes_probability),
        (
            JudgmentQuestion::Score { criteria, .. },
            sunstack_decisions::JudgmentAnswer::Score { score, .. },
        ) if criteria.len() >= 2 => Some(*score / (criteria.len() - 1) as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests;

/// Admit selection questions at the decision boundary, leaving evidence model-free.
pub fn admit_selections(
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
) -> Result<Vec<crate::AdmittedSelection>, Vec<AdmissionRefusal>> {
    let mut selections = Vec::new();
    let mut errors = Vec::new();
    for (name, kind) in kinds {
        let Some(wire) = &kind.select else {
            continue;
        };
        let path = ConfigPath::keys(["evidence", "kinds", name.as_str(), "select"]);
        validate_thresholds(
            wire.yes_at_or_above,
            wire.no_at_or_below,
            &path,
            &mut errors,
        );
        for field in &wire.shared {
            if !matches!(kind.fields.get(field), Some(FieldSource::SameFile { .. })) {
                errors.push(refusal(
                    path.with_key("shared"),
                    "shared must name a defined same_file field",
                ));
            }
        }
        let question = match serde_json::from_value::<JudgmentQuestion>(wire.question.clone()) {
            Ok(question) => question,
            Err(_) => {
                errors.push(refusal(
                    path.with_key("question"),
                    "selection question rejected by decision model schema",
                ));
                continue;
            }
        };
        if !matches!(question, JudgmentQuestion::Noul { .. }) {
            errors.push(refusal(
                path.with_key("question"),
                "selection question must be noul",
            ));
            continue;
        }
        let question_name = QuestionName::new("selection");
        let Ok(question_name) = question_name else {
            continue;
        };
        let questions = NamedQuestionMap::from([(question_name, question.clone())]);
        if sunstack_decisions::judgment_processing::judgment_validation::validate_questions(
            &questions,
        )
        .is_err()
        {
            errors.push(refusal(
                path.with_key("question"),
                "selection question rejected by decision model schema",
            ));
        }
        if let JudgmentQuestion::Noul {
            instructions: StructuredEntry::Object(instructions),
            ..
        } = &question
        {
            for key in ["check", "against", "context"] {
                if let Some(value) = instructions.get(key)
                    && !field_references_valid(value, kind)
                {
                    errors.push(refusal(
                        path.with_key("question")
                            .with_key("instructions")
                            .with_key(key),
                        "instruction must name defined fields",
                    ));
                }
            }
        }
        for field in question_field_references(&questions) {
            if matches!(kind.fields.get(&field), Some(FieldSource::Target { .. })) {
                errors.push(refusal(
                    path.with_key("question"),
                    "selection question cannot name a target field",
                ));
            }
        }
        selections.push(crate::AdmittedSelection {
            kind: name.clone(),
            question,
            yes_at_or_above: wire.yes_at_or_above,
            no_at_or_below: wire.no_at_or_below,
            shared: wire.shared.clone(),
        });
    }
    if errors.is_empty() {
        Ok(selections)
    } else {
        Err(errors)
    }
}

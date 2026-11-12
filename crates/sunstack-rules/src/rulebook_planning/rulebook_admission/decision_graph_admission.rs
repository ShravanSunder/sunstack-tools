use super::*;

pub(crate) fn admit_graph(
    wire: &DecisionGraphWire,
    questions: &NamedQuestionMap,
    kind: &EvidenceKindDefinition,
    project: &EvidenceProject,
    entry_name: &str,
    errors: &mut Vec<AdmissionRefusal>,
    filter_labels: Option<&BTreeSet<sunstack_decisions::ChoiceLabel>>,
) -> DecisionGraph {
    let mut steps = BTreeMap::new();
    let question_names = questions.keys().cloned().collect::<BTreeSet<_>>();
    let mut locator_definitions = BTreeMap::new();
    for (name, step) in &wire.steps {
        let path = ConfigPath::keys(["decision", "steps", name.as_str()]);
        let missing_unsure = match step {
            DecisionStepWire::AskBinary(step) => step.uncertain.is_none(),
            DecisionStepWire::AskChoice(step) => step.uncertain.is_none(),
            DecisionStepWire::Combine(step) => step.uncertain.is_none(),
            DecisionStepWire::Mechanical(_) => false,
        };
        if missing_unsure {
            errors.push(refusal(
                path.with_key("uncertain"),
                "every model step requires an explicit in-between/not-sure route",
            ));
        }
        let mut route = |wire_route: &crate::RouteWire| match DecisionRoute::try_from(
            wire_route.clone(),
        ) {
            Ok(route) => {
                match (&route, filter_labels) {
                    (DecisionRoute::Label(label), Some(labels)) if labels.contains(label) => {},
                    (DecisionRoute::Label(_), _) => errors.push(refusal(&path, "label ending must name a declared filter label; rule trees cannot end in a label")),
                    (DecisionRoute::Pass | DecisionRoute::NotApplicable | DecisionRoute::Finding(_), Some(_)) => errors.push(refusal(&path, "filter tree ends only in a declared label or unsure")),
                    (DecisionRoute::Uncertain, Some(_)) if !matches!(wire_route, crate::RouteWire::Named(name) if name == "unsure") => errors.push(refusal(&path, "filter unsure ending must be written unsure")),
                    (DecisionRoute::Uncertain, None) if matches!(wire_route, crate::RouteWire::Named(name) if name == "unsure") => errors.push(refusal(&path, "rule unsure ending must be written uncertain")),
                    _ => {},
                }
                if let DecisionRoute::Finding(finding) = &route
                    && finding
                        .at
                        .as_ref()
                        .is_some_and(|field| !kind.fields.contains_key(field))
                {
                    errors.push(refusal(&path, "finding at names an undefined field"));
                }
                if let DecisionRoute::Finding(finding) = &route
                    && let Some(locator) = &finding.locate
                {
                    let locator_path = path.with_key("locate");
                    let context = format!("entry {} route {} locate", entry_name, name.as_str());
                    if finding.at.is_none() {
                        errors.push(refusal(&locator_path, format!("{context} requires at")));
                    }
                    let definition = (
                        finding.at.clone(),
                        locator.task.clone(),
                        locator.min_probability,
                    );
                    if question_names.contains(&locator.name)
                        || locator_definitions
                            .get(&locator.name)
                            .is_some_and(|existing| existing != &definition)
                    {
                        errors.push(refusal(
                            locator_path.with_key("name"),
                            format!(
                                "{context} name {} collides with a question or locator",
                                locator.name.as_str()
                            ),
                        ));
                    } else {
                        locator_definitions
                            .entry(locator.name.clone())
                            .or_insert(definition);
                    }
                    if !locator.min_probability.is_finite()
                        || locator.min_probability <= 0.0
                        || locator.min_probability > 1.0
                    {
                        errors.push(refusal(
                            locator_path.with_key("min_probability"),
                            format!(
                                "{context} min_probability must be greater than 0 and at most 1"
                            ),
                        ));
                    }
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
                let uncertain = step
                    .uncertain
                    .as_ref()
                    .map_or(DecisionRoute::Uncertain, &mut route);
                validate_thresholds(step.yes_at_or_above, step.no_at_or_below, &path, errors);
                let routes = BinaryRoutes {
                    uncertain,
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
                let uncertain = step
                    .uncertain
                    .as_ref()
                    .map_or(DecisionRoute::Uncertain, &mut route);
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
                    uncertain,
                    question: step.ask.clone(),
                    min_probability: step.min_probability,
                    routes,
                }
            }
            DecisionStepWire::Combine(step) => {
                let yes = route(&step.yes);
                let no = route(&step.no);
                let uncertain = step
                    .uncertain
                    .as_ref()
                    .map_or(DecisionRoute::Uncertain, &mut route);
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
                        uncertain,
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
pub(super) fn validate_thresholds(
    yes: f64,
    no: f64,
    path: &ConfigPath,
    errors: &mut Vec<AdmissionRefusal>,
) {
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
        DecisionMechanicalTool::ContainsField { field, needle } => {
            if !kind
                .fields
                .get(needle)
                .is_some_and(|source| source.shape() == FieldShape::Text)
            {
                errors.push(refusal(
                    path,
                    "mechanical tool names an undefined or incompatible field",
                ));
            }
            (field, &[FieldShape::Text, FieldShape::Texts])
        }
        DecisionMechanicalTool::MaxChars { field, .. } => (field, &[FieldShape::Text]),
        DecisionMechanicalTool::FileExists { field }
        | DecisionMechanicalTool::AnchorExists { field } => (field, &[FieldShape::Targets]),
        DecisionMechanicalTool::Complete { field } => (
            field,
            &[FieldShape::Text, FieldShape::Texts, FieldShape::Targets],
        ),
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

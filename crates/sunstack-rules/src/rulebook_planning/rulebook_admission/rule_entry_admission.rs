use super::*;

pub fn admit_rule_entry(
    entry: &RuleEntryWire,
    kinds: &BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    project: &EvidenceProject,
    filters: &crate::FilterRegistry,
) -> Result<DecisionGraph, Vec<AdmissionRefusal>> {
    let Some(kind) = kinds.get(&entry.evidence) else {
        return Err(vec![refusal(
            ConfigPath::keys(["evidence"]),
            "undefined evidence kind",
        )]);
    };
    let mut errors = Vec::new();
    for name in entry.reference.keys() {
        if RESERVED_STATE_KEYS.contains(&name.as_str())
            || kind.fields.keys().any(|field| field.as_str() == name)
        {
            errors.push(refusal(
                ConfigPath::keys(["reference", name]),
                "reference name clashes with a reserved state key or evidence field",
            ));
        }
    }
    if let Some(gate) = &entry.only_when
        && let Err(error) = crate::admit_only_when(gate, filters)
    {
        errors.push(error);
    }
    for (position, field) in entry.related.iter().enumerate() {
        if !kind.fields.contains_key(field) {
            errors.push(refusal(
                ConfigPath::keys(["related"]).with_index(position),
                format!(
                    "entry {}: related names undefined field {}",
                    entry.name.as_str(),
                    field.as_str()
                ),
            ));
        }
    }
    admit_questions(
        &entry.questions,
        kind,
        &entry.reference,
        &ConfigPath::keys(["questions"]),
        &mut errors,
    );
    let graph = admit_graph(
        &entry.decision,
        &entry.questions,
        kind,
        project,
        entry.name.as_str(),
        &mut errors,
        None,
    );
    if errors.is_empty() {
        Ok(graph)
    } else {
        Err(errors)
    }
}

pub(crate) fn admit_questions(
    questions: &NamedQuestionMap,
    kind: &EvidenceKindDefinition,
    reference: &crate::ReferenceMaterial,
    root: &ConfigPath,
    errors: &mut Vec<AdmissionRefusal>,
) {
    if !questions.is_empty() {
        if sunstack_decisions::judgment_processing::judgment_validation::validate_questions(
            questions,
        )
        .is_err()
        {
            errors.push(refusal(
                root.clone(),
                "question rejected by decision model schema",
            ));
        }
        for (name, question) in questions {
            let path = root.with_key(name.as_str());
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
                if !["check", "against", "context"]
                    .iter()
                    .any(|key| instructions.contains_key(*key))
                {
                    errors.push(refusal(
                        path.with_key("instructions"),
                        "question must name its fields with check/context",
                    ));
                }
                for (key, value) in instructions {
                    if ["check", "against", "context"].contains(&key.as_str())
                        && !field_references_valid(value, kind, reference)
                    {
                        errors.push(refusal(
                            path.with_key("instructions").with_key(key),
                            "instruction must name defined fields or existing reference paths with check/context",
                        ));
                    }
                }
            } else {
                errors.push(refusal(path.with_key("instructions"), "question must name its fields with check/context; plain-text instructions are no longer supported"));
            }
        }
    }
}

pub(super) fn field_references_valid(
    value: &StructuredJson,
    kind: &EvidenceKindDefinition,
    reference: &crate::ReferenceMaterial,
) -> bool {
    let contains = |name: &str| {
        if [
            "standard",
            "where",
            "where.file",
            "where.first_line",
            "where.last_line",
        ]
        .contains(&name)
        {
            return true;
        }
        if name == "reference" {
            return !reference.is_empty();
        }
        if let Some(path) = name.strip_prefix("reference.") {
            let mut parts = path.split('.');
            let Some(mut current) = parts.next().and_then(|key| reference.get(key)) else {
                return false;
            };
            for key in parts {
                current = match current {
                    StructuredJson::Object(fields) => match fields.get(key) {
                        Some(value) => value,
                        None => return false,
                    },
                    StructuredJson::Array(fields) => match key
                        .parse::<usize>()
                        .ok()
                        .and_then(|index| fields.get(index))
                    {
                        Some(value) => value,
                        None => return false,
                    },
                    _ => return false,
                };
            }
            return true;
        }
        FieldName::new(name)
            .ok()
            .is_some_and(|name| kind.fields.contains_key(&name))
    };
    match value {
        StructuredJson::Text(name) => contains(name),
        StructuredJson::Array(names) => {
            !names.is_empty()
                && names
                    .iter()
                    .all(|name| matches!(name, StructuredJson::Text(name) if contains(name)))
        }
        _ => false,
    }
}

pub(crate) fn question_field_references(questions: &NamedQuestionMap) -> BTreeSet<FieldName> {
    question_state_paths(questions)
        .into_iter()
        .filter_map(|name| FieldName::new(name).ok())
        .collect()
}

pub(crate) fn question_state_paths(questions: &NamedQuestionMap) -> BTreeSet<String> {
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
                    fields.insert(name.to_owned());
                }
            }
        }
    }
    fields
}

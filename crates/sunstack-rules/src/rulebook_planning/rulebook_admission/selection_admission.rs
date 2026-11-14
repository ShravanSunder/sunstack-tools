use super::*;

/// Read the binary statistic using the authored question's scale.
pub fn normalized_answer(
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
            if !matches!(
                kind.fields.get(field),
                Some(FieldSource::SameFile {
                    containing: None,
                    ..
                })
            ) {
                errors.push(refusal(
                    path.with_key("shared"),
                    "shared must name a defined same_file field without a subject-dependent containing filter",
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
                    && !field_references_valid(value, kind, &Default::default())
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
            let unavailable_source = match kind.fields.get(&field) {
                Some(FieldSource::Target { .. }) => Some("target"),
                Some(FieldSource::Document { .. }) => Some("document"),
                _ => None,
            };
            let unavailable_source = unavailable_source.or_else(|| {
                kind.field_needs_document(&field)
                    .then_some("document-dependent")
            });
            if let Some(source) = unavailable_source {
                errors.push(refusal(
                    path.with_key("question"),
                    format!("selection question cannot name a {source} field"),
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

use super::*;

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
                | FieldSource::Enclosing { of, .. }
                | FieldSource::SameFile { of, .. }
                | FieldSource::Remainder { exclude: Some(of) } => match of {
                    MatchSource::Rule { rule } if !project.rule_ids.contains(rule) => {
                        errors.push(refusal(&path, "unknown evidence rule"))
                    }
                    MatchSource::Kind { kind } if !kinds.contains_key(kind) => {
                        errors.push(refusal(&path, "undefined evidence kind"))
                    }
                    _ => {}
                },
                FieldSource::Next { rule, .. } | FieldSource::Previous { rule, .. } => {
                    if !project.rule_ids.contains(rule) {
                        errors.push(refusal(&path, "unknown evidence rule"));
                    }
                }
                FieldSource::Capture { regex } | FieldSource::Target { regex, .. } => {
                    if let FieldSource::Target {
                        exclude: Some(exclude),
                        ..
                    } = source
                        && !project.rule_ids.contains(&exclude.rule)
                    {
                        errors.push(refusal(
                            path.with_key("exclude").with_key("rule"),
                            "unknown evidence rule",
                        ));
                    }
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

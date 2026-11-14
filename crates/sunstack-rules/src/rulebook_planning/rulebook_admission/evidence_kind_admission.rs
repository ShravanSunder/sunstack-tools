use super::*;
use sunstack_evidence::{
    ContextText, EvidenceLanguage, KindMatcher, MarkdownUnitKind, MarkdownUnitSelection,
    SectionText,
};

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
        match &kind.matcher {
            KindMatcher::HtmlUnits(selection) => {
                if selection.validate().is_err() {
                    errors.push(refusal(
                        path.with_key("html_units"),
                        format!("kind {}: invalid html_units selection", name.as_str()),
                    ));
                }
            }
            KindMatcher::MarkdownUnits(selection) => {
                if selection.validate().is_err() {
                    errors.push(refusal(
                        path.with_key("markdown_units"),
                        format!("kind {}: invalid markdown_units selection", name.as_str()),
                    ));
                }
                if matches!(selection, MarkdownUnitSelection::CallSite { .. }) {
                    errors.push(refusal(
                        path.with_key("markdown_units").with_key("mode"),
                        format!(
                            "kind {}: markdown_units requires unit mode; call_site is not supported in rulebook kinds",
                            name.as_str()
                        ),
                    ));
                }
            }
            KindMatcher::Named(_) | KindMatcher::Inline { .. } => {
                if kind
                    .matcher
                    .ast_rule_id(name)
                    .is_none_or(|rule| !project.rule_ids.contains(&rule))
                {
                    errors.push(refusal(path.with_key("rule"), "unknown evidence rule"));
                }
            }
        }
        let language = match &kind.matcher {
            KindMatcher::HtmlUnits(_) => Some(EvidenceLanguage::Html),
            KindMatcher::MarkdownUnits(_) => Some(EvidenceLanguage::Markdown),
            KindMatcher::Named(_) | KindMatcher::Inline { .. } => kind
                .matcher
                .ast_rule_id(name)
                .and_then(|rule| project.language_for_rule(&rule)),
        };
        if kind.max_chars.is_some_and(|n| n.get() < 1000) {
            errors.push(refusal(path.with_key("max_chars"), "must be at least 1000"));
        }
        for (field, source) in &kind.fields {
            let path = path.with_key("fields").with_key(field.as_str());
            if RESERVED_STATE_KEYS.contains(&field.as_str()) {
                errors.push(refusal(
                    &path,
                    "evidence field name is a reserved state key",
                ));
            }
            if matches!(source, FieldSource::Section { .. })
                && !is_native_section_matcher(&kind.matcher)
            {
                errors.push(refusal(
                    &path,
                    format!(
                        "kind {} field {}: section source requires an html_units matcher or markdown_units with unit mode and units exactly [section]; AST-backed kinds do not have outline sections",
                        name.as_str(), field.as_str()
                    ),
                ));
            }
            if let FieldSource::Enclosing {
                of,
                text: ContextText::Heading,
            } = source
            {
                let section_kind = match of {
                    MatchSource::Kind { kind } => kinds
                        .get(kind)
                        .is_some_and(|kind| is_native_section_matcher(&kind.matcher)),
                    MatchSource::Rule { .. } => false,
                };
                if !section_kind {
                    errors.push(refusal(
                        &path,
                        format!(
                            "kind {} field {}: enclosing heading requires a native section kind (html_units or markdown_units with unit mode and units exactly [section])",
                            name.as_str(), field.as_str()
                        ),
                    ));
                }
            }
            if matches!(
                source,
                FieldSource::SameFile {
                    text: ContextText::Heading,
                    ..
                }
            ) {
                errors.push(refusal(
                    &path,
                    format!(
                        "kind {} field {}: same_file does not support heading text",
                        name.as_str(),
                        field.as_str()
                    ),
                ));
            }
            let native_matcher = match &kind.matcher {
                KindMatcher::HtmlUnits(_) => Some("html_units"),
                KindMatcher::MarkdownUnits(_) => Some("markdown_units"),
                KindMatcher::Named(_) | KindMatcher::Inline { .. } => None,
            };
            let neighbour_source = match source {
                FieldSource::Next { .. } => Some("next"),
                FieldSource::Previous { .. } => Some("previous"),
                _ => None,
            };
            if let (Some(native_matcher), Some(neighbour_source)) =
                (native_matcher, neighbour_source)
            {
                errors.push(refusal(
                    &path,
                    format!(
                        "kind {} field {}: {neighbour_source} source requires an AST-backed matcher; {native_matcher} has no AST candidate node",
                        name.as_str(), field.as_str()
                    ),
                ));
            }
            if let Some(language) = language {
                let context = format!("kind {} field {}", name.as_str(), field.as_str());
                if language != EvidenceLanguage::Markdown
                    && matches!(source, FieldSource::Yaml { .. })
                {
                    errors.push(refusal(
                        &path,
                        if language == EvidenceLanguage::Json {
                            format!("{context}: yaml is not supported on JSON evidence")
                        } else {
                            format!("{context}: yaml is not supported on {language:?} evidence")
                        },
                    ));
                }
                let referenced = match source {
                    FieldSource::ScopedInside { .. } => None,
                    FieldSource::SectionPrefix { kind } => kinds.get(kind).and_then(|definition| {
                        sunstack_evidence::evidence_kind_language(project, kind, definition)
                    }),
                    FieldSource::Inside { of }
                    | FieldSource::Enclosing { of, .. }
                    | FieldSource::SameFile { of, .. }
                    | FieldSource::Remainder { exclude: Some(of) } => match of {
                        MatchSource::Rule { rule } => project.language_for_rule(rule),
                        MatchSource::Kind { kind } => {
                            kinds
                                .get(kind)
                                .and_then(|definition| match &definition.matcher {
                                    KindMatcher::HtmlUnits(_) => Some(EvidenceLanguage::Html),
                                    KindMatcher::MarkdownUnits(_) => {
                                        Some(EvidenceLanguage::Markdown)
                                    }
                                    KindMatcher::Named(_) | KindMatcher::Inline { .. } => {
                                        definition
                                            .matcher
                                            .ast_rule_id(kind)
                                            .and_then(|rule| project.language_for_rule(&rule))
                                    }
                                })
                        }
                    },
                    FieldSource::Next { rule, .. } | FieldSource::Previous { rule, .. } => {
                        project.language_for_rule(rule)
                    }
                    FieldSource::Target {
                        exclude: Some(exclude),
                        ..
                    } => project.language_for_rule(&exclude.rule),
                    FieldSource::Document { .. }
                    | FieldSource::FirstLine { .. }
                    | FieldSource::Section { .. }
                    | FieldSource::Capture { .. }
                    | FieldSource::Yaml { .. }
                    | FieldSource::Remainder { exclude: None }
                    | FieldSource::Target { exclude: None, .. } => None,
                };
                let island_context = language == EvidenceLanguage::Markdown
                    && referenced == Some(EvidenceLanguage::Html)
                    && !project.document_settings.html.embedded_markdown.is_empty()
                    && matches!(
                        source,
                        FieldSource::Enclosing { .. } | FieldSource::SectionPrefix { .. }
                    );
                if referenced.is_some_and(|referenced| referenced != language) && !island_context {
                    errors.push(refusal(
                        &path,
                        format!("{context}: referenced rule or kind has another evidence language"),
                    ));
                }
            }
            match source {
                FieldSource::ScopedInside { kind, scope } => {
                    if !kinds.contains_key(kind) {
                        errors.push(refusal(path.with_key("kind"), "undefined evidence kind"));
                    }
                    if scope.kinds().iter().any(|kind| !kinds.contains_key(kind)) {
                        errors.push(refusal(
                            path.with_key("scope"),
                            "undefined section scope kind",
                        ));
                    }
                }
                FieldSource::SectionPrefix { kind } => {
                    if !kinds.contains_key(kind) {
                        errors.push(refusal(&path, "undefined evidence kind"));
                    }
                }
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
                            Some(target)
                                if target.fields.values().any(|source| {
                                    matches!(source, FieldSource::Document { .. })
                                }) =>
                            {
                                errors
                                    .push(refusal(&path, "as_kind must name a document-free kind"))
                            }
                            Some(target)
                                if target.fields.values().any(|source| {
                                    matches!(
                                        source,
                                        FieldSource::Section {
                                            text: SectionText::Heading
                                        } | FieldSource::Enclosing {
                                            text: ContextText::Heading,
                                            ..
                                        } | FieldSource::SameFile {
                                            text: ContextText::Heading,
                                            ..
                                        }
                                    )
                                }) =>
                            {
                                errors.push(refusal(&path, "as_kind must name a heading-free kind"))
                            }
                            Some(_) => {}
                        }
                    }
                }
                FieldSource::Document {
                    path: document_path,
                    rule,
                    ..
                } => {
                    if document_path.paths().is_empty()
                        || document_path
                            .paths()
                            .iter()
                            .any(|path| path.as_os_str().is_empty())
                    {
                        errors.push(refusal(
                            path.with_key("path"),
                            "document path must not be empty",
                        ));
                    }
                    if !project.rule_ids.contains(rule) {
                        errors.push(refusal(path.with_key("rule"), "unknown evidence rule"));
                    }
                }
                FieldSource::FirstLine {}
                | FieldSource::Section { .. }
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

fn is_native_section_matcher(matcher: &KindMatcher) -> bool {
    match matcher {
        KindMatcher::HtmlUnits(_) => true,
        KindMatcher::MarkdownUnits(MarkdownUnitSelection::Unit { units, .. }) => {
            units.as_slice() == [MarkdownUnitKind::Section]
        }
        KindMatcher::MarkdownUnits(MarkdownUnitSelection::CallSite { .. })
        | KindMatcher::Named(_)
        | KindMatcher::Inline { .. } => false,
    }
}

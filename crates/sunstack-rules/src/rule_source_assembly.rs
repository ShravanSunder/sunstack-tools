use crate::{definition_admission::locate_at, rule_source_decoding::decode_source_entries, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};
use sunstack_decisions::{JudgmentQuestion, StructuredEntry};
use sunstack_evidence::{
    DeclarationId, EvidenceDeclaration, EvidenceRequirement, MarkdownBindingRefusal,
    MarkdownDocument, MarkdownMarkerBinding, MarkerId, MarkerPrefix,
};

pub(crate) struct AssembledSource {
    pub rules: Vec<LocatedRuleDefinition>,
    pub skipped: Vec<SkippedRule>,
}

pub(crate) fn assemble_source(
    markdown: &str,
    rules_bytes: &[u8],
    source: &RuleSourcePair,
    rule_set: &Path,
    prefix: &MarkerPrefix,
    cap: usize,
    inputs: &BTreeMap<sunstack_evidence::SourceName, InputDeclaration>,
) -> Result<AssembledSource, DiagnosticList> {
    let entries = decode_source_entries(rules_bytes, &source.rules)?;
    let mut entry_groups = BTreeMap::<String, Vec<_>>::new();
    for entry in entries {
        entry_groups
            .entry(entry.key.clone())
            .or_default()
            .push(entry);
    }
    let bindings = MarkdownDocument::parse(markdown).bind_markers(prefix, cap);
    let mut marker_groups = BTreeMap::<String, Vec<_>>::new();
    let mut order = Vec::new();
    for binding in bindings {
        let marker = match &binding {
            MarkdownMarkerBinding::Bound { binding } => &binding.marker,
            MarkdownMarkerBinding::Refused { marker, .. } => marker,
        };
        let key = marker.id.as_str().to_owned();
        if !marker_groups.contains_key(&key) {
            order.push(key.clone());
        }
        marker_groups.entry(key).or_default().push(binding);
    }
    let mut seen = BTreeSet::new();
    let mut result = AssembledSource {
        rules: vec![],
        skipped: vec![],
    };
    for key in order.into_iter().chain(entry_groups.keys().cloned()) {
        if !seen.insert(key.clone()) {
            continue;
        }
        let mut diagnostics = DiagnosticList::default();
        let markers = marker_groups.get(&key);
        let authored = entry_groups.get(&key);
        let mut decoded = None;
        if let Some(entries) = authored {
            for entry in entries {
                match &entry.decoded {
                    Ok(value) => decoded = Some((value.clone(), entry.location.clone())),
                    Err(errors) => diagnostics.extend(errors.clone()),
                }
            }
        }
        if let Some(markers) = markers {
            for marker in markers {
                if let MarkdownMarkerBinding::Refused { marker, reason } = marker {
                    let code = match reason {
                        MarkdownBindingRefusal::ForbiddenHost => {
                            RuleDiagnosticCode::ForbiddenMarkerHost
                        }
                        MarkdownBindingRefusal::SpacedHeading => {
                            RuleDiagnosticCode::SpacedHeadingMarker
                        }
                        MarkdownBindingRefusal::DuplicateId => RuleDiagnosticCode::DuplicateMarker,
                        MarkdownBindingRefusal::QuestionTooLarge { .. } => {
                            RuleDiagnosticCode::QuestionTooLarge
                        }
                    };
                    diagnostics.extend(
                        RuleDiagnostic::new(code)
                            .definition(
                                &source.markdown,
                                marker.span.start_line,
                                marker.span.start_column,
                                vec![marker.id.as_str().into()],
                            )
                            .into(),
                    );
                }
            }
            if authored.is_none() {
                let marker = match &markers[0] {
                    MarkdownMarkerBinding::Bound { binding } => &binding.marker,
                    MarkdownMarkerBinding::Refused { marker, .. } => marker,
                };
                diagnostics.extend(
                    RuleDiagnostic::new(RuleDiagnosticCode::MissingRuleEntry)
                        .definition(
                            &source.markdown,
                            marker.span.start_line,
                            marker.span.start_column,
                            vec![marker.id.as_str().into()],
                        )
                        .into(),
                );
            }
        } else if let Some(entries) = authored {
            for entry in entries {
                let valid_key =
                    MarkerId::new(&key).is_ok_and(|id| id.as_str().starts_with(prefix.as_str()));
                let code = if valid_key {
                    RuleDiagnosticCode::OrphanedRuleEntry
                } else {
                    RuleDiagnosticCode::InvalidRuleSourceKey
                };
                diagnostics.extend(
                    RuleDiagnostic::new(code)
                        .definition(
                            &source.rules,
                            entry.location.line,
                            entry.location.column,
                            if valid_key { vec![key.clone()] } else { vec![] },
                        )
                        .into(),
                );
            }
        }
        let rule_id = decoded.as_ref().map(|(entry, _)| entry.name.clone());
        if diagnostics.is_empty()
            && let (Some((entry, location)), Some(markers)) = (decoded, markers)
            && let [MarkdownMarkerBinding::Bound { binding }] = markers.as_slice()
        {
            let mut evidence = vec![EvidenceDeclaration {
                id: DeclarationId::new("target")
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?,
                input: entry.target.input,
                select: entry.target.select,
                requirement: EvidenceRequirement::Required,
                max_items: None,
                max_bytes: None,
            }];
            evidence.extend(entry.context);
            let definition = RuleDefinition {
                rule_id: entry.name,
                evidence,
                target: DeclarationId::new("target")
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?,
                applicability: vec![],
                questions: BTreeMap::new(),
                verdict: match entry.verdict {
                    RuleSourceVerdict::Judgment {
                        threshold,
                        block_at_or_below,
                    } => VerdictDeclaration::Noul {
                        question: JudgmentQuestion::Noul {
                            instructions: StructuredEntry::Text(binding.question_text.clone()),
                            criteria: None,
                        },
                        threshold,
                        block_at_or_below,
                    },
                    RuleSourceVerdict::Expect { expect } => VerdictDeclaration::Mechanical {
                        explanation: binding.question_text.clone(),
                        check: MechanicalCheck::Expect { expect },
                    },
                    RuleSourceVerdict::AstGrep { ast_grep } => VerdictDeclaration::Mechanical {
                        explanation: binding.question_text.clone(),
                        check: MechanicalCheck::AstGrep { ast_grep },
                    },
                },
                severity: entry.severity,
            };
            match validate_rule_definition(&definition, inputs) {
                Ok(()) => {
                    result.rules.push(LocatedRuleDefinition {
                        source: source.rules.clone(),
                        line: location.line,
                        provenance: Some(RuleSourceProvenance {
                            markdown: source.markdown.clone(),
                            rules: source.rules.clone(),
                            marker_id: binding.marker.id.clone(),
                            marker_span: binding.marker.span.clone(),
                            entry_location: location,
                        }),
                        definition: Arc::new(definition),
                    });
                    continue;
                }
                Err(mut errors) => {
                    locate_at(&mut errors, &source.rules, location.line, location.column);
                    diagnostics.extend(errors);
                }
            }
        }
        result.skipped.push(SkippedRule::Entry {
            rule_set: rule_set.into(),
            markdown: source.markdown.clone(),
            rules: source.rules.clone(),
            key,
            rule_id,
            diagnostics,
        });
    }
    if result.rules.is_empty() && result.skipped.is_empty() {
        return Err(RuleDiagnostic::new(RuleDiagnosticCode::EmptyRuleFile)
            .definition(&source.rules, 1, 1, vec![])
            .into());
    }
    Ok(result)
}

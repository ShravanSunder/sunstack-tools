use crate::{candidate_inputs::CandidateMaterial, invocation_admission::LoadedRuleSet, *};
use std::collections::{BTreeMap, BTreeSet};
use sunstack_evidence::{PrivacyProjection, SourceName};

pub(crate) fn project_evidence_uses(
    input: &CandidateRunInput,
    material: &CandidateMaterial,
    set: &LoadedRuleSet,
    report: Option<&RuleInvocationReport>,
    identities: Option<&BTreeMap<SourceName, BTreeSet<String>>>,
    sources: &crate::input_binding::AdmittedSources,
) -> Result<(Vec<EvidenceUse>, PrivacyProjection), DiagnosticList> {
    let rule = set
        .rules
        .iter()
        .find(|rule| rule.definition.rule_id == material.rule.rule_id)
        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
    let subject = rule
        .definition
        .evidence
        .iter()
        .find(|declaration| declaration.id == rule.definition.target)
        .ok_or_else(|| DiagnosticList::rule(RuleDiagnosticCode::InvalidTarget))?;
    let mut names = rule
        .definition
        .evidence
        .iter()
        .map(|declaration| declaration.input.clone())
        .collect::<BTreeSet<_>>();
    for entry in set
        .definition
        .applicability
        .iter()
        .chain(&rule.definition.applicability)
    {
        match entry {
            ApplicabilityEntry::Decision { evidence, .. } => {
                names.extend(evidence.iter().map(|declaration| declaration.input.clone()))
            }
            ApplicabilityEntry::Condition { test } => match test {
                ConditionTest::PayloadFieldEquals { input, .. }
                | ConditionTest::PayloadFieldIn { input, .. }
                | ConditionTest::PayloadFieldPresent { input, .. }
                | ConditionTest::PatternPresent { input, .. } => {
                    names.insert(input.clone());
                }
                ConditionTest::OriginFieldEquals { .. } => {}
            },
        }
    }
    names.remove(&subject.input);
    let grants = material.view.grants.iter().copied().collect();
    let mut privacy = PrivacyProjection::default();
    let mut uses = Vec::new();
    for name in names {
        let mut binding = input.sources[material.entry_position]
            .evidence
            .get(&name)
            .or_else(|| input.evidence.get(&name))
            .cloned()
            .unwrap_or(EvidenceBinding::Unavailable {
                reason: UnavailableReason::NotRecorded,
            });
        let identity = binding_identity(
            &binding,
            identities.and_then(|identities| identities.get(&name)),
            report,
            sources.get(&name),
        )?;
        if let EvidenceBinding::Bound { binding } = &mut binding {
            match binding {
                InputBinding::Payload { value } => {
                    let (projected, projection) =
                        sunstack_evidence::project_structured_json(value, &grants)
                            .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                    *value = projected;
                    for (kind, count) in projection.left_out {
                        *privacy.left_out.entry(kind).or_default() += count;
                    }
                }
                InputBinding::Conversation {
                    final_assistant_text: Some(text),
                    ..
                } => {
                    let (projected, projection) = sunstack_evidence::project_text(text, &grants)
                        .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                    *text = projected;
                    for (kind, count) in projection.left_out {
                        *privacy.left_out.entry(kind).or_default() += count;
                    }
                }
                InputBinding::Document { .. }
                | InputBinding::ChangeSet { .. }
                | InputBinding::Conversation {
                    final_assistant_text: None,
                    ..
                } => {}
            }
        }
        uses.push(EvidenceUse {
            input: name,
            binding,
            content_identity: identity,
        });
    }
    Ok((uses, privacy))
}

fn binding_identity(
    binding: &EvidenceBinding,
    identities: Option<&BTreeSet<String>>,
    report: Option<&RuleInvocationReport>,
    source: Option<&crate::input_binding::AdmittedSource>,
) -> Result<Option<String>, DiagnosticList> {
    match binding {
        EvidenceBinding::Unavailable { .. } => return Ok(None),
        EvidenceBinding::Bound {
            binding: InputBinding::Payload { value },
        } => {
            let bytes = serde_json::to_vec(value)
                .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
            return Ok(Some(sunstack_evidence::content_digest(&bytes)));
        }
        EvidenceBinding::Bound {
            binding: InputBinding::Conversation { .. },
        } => {
            if let Some(crate::input_binding::AdmittedSource {
                id: crate::input_binding::SourceId::Conversation { path, format, .. },
                ..
            }) = source
                && let Some(cursor) = report
                    .into_iter()
                    .flat_map(|report| &report.cursors)
                    .find(|cursor| cursor.format == *format && cursor.transcript_path == *path)
            {
                return sunstack_evidence::MonitoringCursor::decode(&cursor.cursor)
                    .ok()
                    .map(|cursor| match cursor.supplement_digest {
                        Some(supplement) => serde_json::to_vec(&(cursor.prefix_digest, supplement))
                            .map(|bytes| Some(sunstack_evidence::content_digest(&bytes)))
                            .map_err(|_| {
                                DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration)
                            }),
                        None => Ok(Some(cursor.prefix_digest)),
                    })
                    .unwrap_or(Ok(None));
            }
        }
        EvidenceBinding::Bound {
            binding: InputBinding::Document { .. },
        } => {
            if let Some(crate::input_binding::AdmittedSource {
                id: crate::input_binding::SourceId::Document { repository, path },
                ..
            }) = source
            {
                for capture in report.into_iter().flat_map(|report| &report.captures) {
                    match &capture.source {
                        sunstack_evidence::SourceRef::Document {
                            repository: captured_repository,
                            path: captured_path,
                            content_identity,
                        }
                        | sunstack_evidence::SourceRef::RepositoryFile {
                            repository: captured_repository,
                            path: captured_path,
                            content_identity,
                            ..
                        } if captured_repository == repository && captured_path == path => {
                            return Ok(Some(content_identity.clone()));
                        }
                        _ => {}
                    }
                }
            }
        }
        EvidenceBinding::Bound {
            binding: InputBinding::ChangeSet { .. },
        } => {}
    }
    match identities {
        Some(identities) if identities.len() == 1 => Ok(identities.first().cloned()),
        Some(identities) if !identities.is_empty() => {
            let bytes = serde_json::to_vec(identities)
                .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
            Ok(Some(sunstack_evidence::content_digest(&bytes)))
        }
        _ => Ok(None),
    }
}

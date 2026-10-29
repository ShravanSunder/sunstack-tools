use crate::DiagnosticList;
use crate::candidate_narrowing_samples::CandidateNarrowingSampleView;
use crate::{
    candidate_inputs::{append_privacy_counts, invalid_candidate_field},
    candidate_subjects::*,
    *,
};
use std::collections::BTreeSet;
use sunstack_evidence::*;
pub(crate) struct CapturedCandidateCode {
    pub entry_position: usize,
    pub source_name: SourceName,
    pub index: CodeContextIndex,
}
pub(crate) struct CodeDiscoverySettings<'run> {
    pub samples_per_pass: usize,
    pub context_settings: &'run ContextSettings,
    pub cancellation: &'run tokio_util::sync::CancellationToken,
}
pub(crate) fn discover_code_candidates(
    set: &crate::invocation_admission::LoadedRuleSet,
    sources: &[CapturedCandidateCode],
    settings: CodeDiscoverySettings<'_>,
    privacy: &mut PrivacyReport,
    dropped: &mut Vec<DroppedSample>,
    reports: &mut [InputReport],
) -> Result<Vec<CandidateMaterial>, DiagnosticList> {
    let CodeDiscoverySettings {
        samples_per_pass: samples,
        context_settings,
        cancellation,
    } = settings;
    let mut materials = Vec::new();
    for rule in &set.rules {
        let rule_ref = RuleRef {
            rule_set: set.path.clone(),
            rule_id: rule.definition.rule_id.clone(),
        };
        let grants = rule
            .definition
            .private_access
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        for source in sources {
            if cancellation.is_cancelled() {
                break;
            }
            let receipt = reports
                .get_mut(source.entry_position)
                .and_then(|report| {
                    report
                        .discovery
                        .iter_mut()
                        .find(|discovery| discovery.rule == rule_ref)
                })
                .ok_or_else(|| invalid_candidate_field(vec!["sources".into()]))?;
            receipt.status = RuleDiscoveryStatus::Stopped;
            let target = rule
                .definition
                .evidence
                .iter()
                .find(|declaration| declaration.id == rule.definition.target)
                .ok_or_else(|| invalid_candidate_field(vec!["definitions".into()]))?;
            if matches!(source.index.input, SourceInput::Code { .. })
                && matches!(
                    target.select,
                    EvidenceSelection::MarkdownUnits(_)
                        | EvidenceSelection::FrontmatterField { .. }
                )
            {
                let discovery = source
                    .index
                    .discover_documents(
                        &target.select,
                        rule.definition.mechanical_pass.as_ref(),
                        samples,
                    )
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                let coverage = discovery
                    .coverage
                    .unwrap_or_else(|| source.index.coverage.clone());
                for sample in discovery.rejected_sample {
                    let (projected, counts) = sample
                        .project(&grants)
                        .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                    append_privacy_counts(privacy, PrivacyRoute::Samples, counts);
                    dropped.push(DroppedSample {
                        rule: rule_ref.clone(),
                        pass: SamplePass::Mechanical,
                        occurrence: sample.occurrence,
                        excerpt: projected.visible_text(),
                    });
                }
                for matched in discovery.matches {
                    let (projected, counts) = matched
                        .project(&grants)
                        .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                    let occurrence_identity = matched
                        .occurrence
                        .occurrence_identity()
                        .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                    let narrowing_sample = if rule.definition.narrowing.is_some() {
                        Some(
                            CandidateNarrowingSampleView::new(
                                source.entry_position,
                                &matched.occurrence,
                                None,
                                &occurrence_identity,
                                projected.visible_text(),
                                counts.clone(),
                            )
                            .ok_or_else(|| invalid_candidate_field(vec!["sources".into()]))?,
                        )
                    } else {
                        None
                    };
                    append_privacy_counts(privacy, PrivacyRoute::Code, counts);
                    materials.push(CandidateMaterial {
                        candidate_id: CandidateId {
                            rule: rule_ref.clone(),
                            occurrence: occurrence_identity,
                        },
                        entry_position: source.entry_position,
                        source_name: source.source_name.clone(),
                        rule: rule_ref.clone(),
                        occurrence: matched.occurrence,
                        view: PrivacyView {
                            grants: rule.definition.private_access.clone(),
                        },
                        selected: CandidateSelection::Document {
                            selected: std::sync::Arc::new(projected),
                            coverage: coverage.clone(),
                            grants: grants.clone(),
                        },
                        narrowing_sample,
                    });
                }
                receipt.status = if discovery.completed {
                    RuleDiscoveryStatus::Completed
                } else {
                    RuleDiscoveryStatus::Stopped
                };
                continue;
            }
            let discovery = source
                .index
                .discover(rule.definition.mechanical_pass.as_ref(), samples)
                .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
            for sample in discovery.rejected_sample {
                let projected = project_code_context(&sample.context, &grants)
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                append_privacy_counts(privacy, PrivacyRoute::Samples, projected.privacy);
                dropped.push(DroppedSample {
                    rule: rule_ref.clone(),
                    pass: SamplePass::Mechanical,
                    occurrence: sample.occurrence,
                    excerpt: projected.context.matched.text,
                });
            }
            for matched in discovery.matches {
                let projected = project_code_context(&matched.context, &grants)
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                let occurrence_identity = matched
                    .occurrence
                    .occurrence_identity()
                    .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
                let narrowing_sample = if rule.definition.narrowing.is_some() {
                    Some(
                        CandidateNarrowingSampleView::new(
                            source.entry_position,
                            &matched.occurrence,
                            None,
                            &occurrence_identity,
                            projected.context.matched.text.clone(),
                            projected.privacy.clone(),
                        )
                        .ok_or_else(|| invalid_candidate_field(vec!["sources".into()]))?,
                    )
                } else {
                    None
                };
                append_privacy_counts(privacy, PrivacyRoute::Code, projected.privacy);
                let selected = if projected.match_is_private {
                    CandidateSelection::LeftOutForPrivacy
                } else {
                    code_selection(
                        projected.context,
                        &grants,
                        context_settings,
                        &projected.redaction_ranges,
                    )?
                };
                materials.push(CandidateMaterial {
                    candidate_id: CandidateId {
                        rule: rule_ref.clone(),
                        occurrence: occurrence_identity,
                    },
                    entry_position: source.entry_position,
                    source_name: source.source_name.clone(),
                    rule: rule_ref.clone(),
                    occurrence: matched.occurrence,
                    view: PrivacyView {
                        grants: rule.definition.private_access.clone(),
                    },
                    selected,
                    narrowing_sample,
                });
            }
            receipt.status = if discovery.completed {
                RuleDiscoveryStatus::Completed
            } else {
                RuleDiscoveryStatus::Stopped
            };
        }
    }
    Ok(materials)
}

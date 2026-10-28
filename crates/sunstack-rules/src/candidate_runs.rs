use crate::{candidate_inputs::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

pub async fn evaluate_candidates(
    input: CandidateRunInput,
    context: RuleExecutionContext,
) -> CandidateRunOutcome {
    match run_candidates(input, context).await {
        Ok(outcome) => outcome,
        Err(diagnostics) => CandidateRunOutcome::Invalid { diagnostics },
    }
}
async fn run_candidates(
    input: CandidateRunInput,
    context: RuleExecutionContext,
) -> Result<CandidateRunOutcome, DiagnosticList> {
    let started = Instant::now();
    if input.sources.is_empty()
        || input.definition_root.as_os_str().is_empty()
        || input.backend != *context.backend.selection()
        || input.request_budget.max_requests == 0
        || input.request_budget.max_questions == 0
        || input.request_budget.max_request_bytes == 0
        || input.request_budget.max_input_tokens == 0
    {
        return Err(invalid_candidate_field(vec![]));
    }
    input.attempts.validate().map_err(|diagnostic| {
        DiagnosticList::from(sunstack_decisions::DiagnosticList::from(diagnostic))
    })?;
    let CandidateDefinitions::RuleSets { rule_sets } = &input.definitions;
    if rule_sets.is_empty() {
        return Err(invalid_candidate_field(vec!["definitions".into()]));
    }
    let cwd = std::env::current_dir().map_err(|_| invalid_candidate_field(vec![]))?;
    let root = crate::input_binding::resolved_path(&cwd, &input.definition_root)?;
    let paths = rule_sets.clone();
    let max_bytes = input.limits.max_file_bytes.get();
    let cancellation = context.cancellation.clone();
    let sets = tokio::task::spawn_blocking(move || {
        let mut loader = crate::definition_loading::DefinitionLoader::new(max_bytes, cancellation);
        let mut seen = BTreeSet::new();
        paths
            .into_iter()
            .map(|path| {
                let path = crate::input_binding::resolved_path(&root, &path)?;
                if !seen.insert(path.clone()) {
                    return Err(DiagnosticList::rule(RuleDiagnosticCode::DuplicateRuleSet));
                }
                loader.load_set(path)
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError))??;
    let mut source_capture = capture_candidate_sources(&input, &context).await?;
    for report in &mut source_capture.reports {
        for set in &sets {
            for rule in &set.rules {
                report.discovery.push(RuleDiscovery {
                    rule: RuleRef {
                        rule_set: set.path.clone(),
                        rule_id: rule.definition.rule_id.clone(),
                    },
                    status: RuleDiscoveryStatus::NotStarted,
                });
            }
        }
    }
    let mut privacy = PrivacyReport::default();
    for (private_kind, count) in &source_capture.privacy.left_out {
        privacy.left_out.push(PrivacyCount {
            route: PrivacyRoute::Records,
            private_kind: *private_kind,
            count: *count,
        });
    }
    let mut dropped_samples = Vec::new();
    let mut materials = Vec::new();
    let mut candidates = Vec::new();
    let mut bound_sets = Vec::new();
    let mut policy = RulePolicy::default();
    for set in &sets {
        let selected = discover_rule_candidates(
            set,
            &source_capture.captured,
            input.samples_per_pass,
            &mut privacy,
            &mut dropped_samples,
            &mut source_capture.reports,
            &context.cancellation,
        )?;
        let mut binding_groups = Vec::new();
        for material in selected {
            if matches!(
                material.selected,
                CandidateRecordSelection::LeftOutForPrivacy
            ) {
                candidates.push(CandidateEntry {
                    candidate_id: material.candidate_id,
                    rule: material.rule,
                    occurrence: material.occurrence,
                    outcome: CandidateOutcome::NotProcessed {
                        reason: NotProcessedReason::LeftOutForPrivacy,
                    },
                    answers: vec![],
                    privacy_view: material.view,
                    elapsed_ms: None,
                });
                continue;
            }
            let rule = set
                .rules
                .iter()
                .find(|rule| rule.definition.rule_id == material.rule.rule_id)
                .ok_or_else(|| invalid_candidate_field(vec!["definitions".into()]))?;
            let target = rule
                .definition
                .evidence
                .iter()
                .find(|evidence| evidence.id == rule.definition.target)
                .ok_or_else(|| invalid_candidate_field(vec!["definitions".into()]))?;
            let group = binding_groups.len();
            binding_groups.push(BindingGroup {
                bindings: BTreeMap::from([(
                    target.input.clone(),
                    InputBinding::Conversation {
                        transcript_path: material.path.clone(),
                        format: material.format.clone(),
                        final_assistant_text: None,
                    },
                )]),
            });
            policy.off.insert(
                GroupKey {
                    rule_set: set.path.clone(),
                    group,
                },
                set.rules
                    .iter()
                    .filter(|other| other.definition.rule_id != material.rule.rule_id)
                    .map(|other| other.definition.rule_id.clone())
                    .collect(),
            );
            materials.push((material, group));
        }
        if !binding_groups.is_empty() {
            bound_sets.push(BoundRuleSet {
                path: set.path.clone(),
                binding_groups,
            });
        }
    }
    let mut usage = RunUsage::default();
    let mut cancelled = context.cancellation.is_cancelled();
    if !materials.is_empty() {
        let invocation = RuleInvocationInput {
            definition_root: input.definition_root.clone(),
            definitions: InvocationDefinitions::RuleSets {
                rule_sets: bound_sets,
            },
            repositories: input.repositories.clone(),
            discovery: sunstack_evidence::EvidenceDiscovery::None {},
            package_bindings: vec![],
            payload_schemas: input.payload_schemas.clone(),
            origin: InvocationOrigin::Direct {},
            cursors: vec![],
            execution: input.execution.clone(),
            limits: input.limits.clone(),
            backend: input.backend.clone(),
            attempts: input.attempts.clone(),
            request_budget: input.request_budget.clone(),
        };
        let mut admitted = crate::invocation_admission::admit_invocation(
            &invocation,
            &context.adapters,
            &context.cancellation,
        )
        .await?;
        for group in &mut admitted.groups {
            let (material, _) = materials
                .iter()
                .find(|(material, position)| {
                    material.rule.rule_set == group.set.path && *position == group.position
                })
                .ok_or_else(|| invalid_candidate_field(vec!["definitions".into()]))?;
            for source in group.sources.values_mut() {
                source.candidate = Some(match &material.selected {
                    CandidateRecordSelection::Ready { selected } => selected.clone(),
                    CandidateRecordSelection::LeftOutForPrivacy => {
                        return Err(invalid_candidate_field(vec!["sources".into()]));
                    }
                });
            }
        }
        let mut request_evidence = BTreeMap::new();
        let mut candidate_elapsed_ms = BTreeMap::new();
        let native = crate::rule_execution::evaluate_with_evidence(
            invocation,
            policy,
            context,
            &mut request_evidence,
            &mut candidate_elapsed_ms,
            Some(admitted),
        )
        .await;
        let report = match &native {
            RuleInvocationOutcome::CompleteReport { report }
            | RuleInvocationOutcome::IncompleteReport { report } => Some(report),
            RuleInvocationOutcome::Cancelled {
                progress: InvocationCancellationProgress::ReportAdmitted { partial_report },
                ..
            } => {
                cancelled = true;
                Some(partial_report.as_ref())
            }
            RuleInvocationOutcome::Cancelled { .. } => {
                cancelled = true;
                None
            }
            RuleInvocationOutcome::Invalid { .. }
            | RuleInvocationOutcome::PreparationFailed { .. } => None,
        };
        if let Some(report) = report {
            usage = crate::candidate_projection::project_candidate_usage(report);
        }
        for (material, group) in materials {
            let set = sets
                .iter()
                .find(|set| set.path == material.rule.rule_set)
                .ok_or_else(|| invalid_candidate_field(vec!["definitions".into()]))?;
            let elapsed_ms = candidate_elapsed_ms
                .get(&GroupKey {
                    rule_set: material.rule.rule_set.clone(),
                    group,
                })
                .copied();
            let mut entry =
                crate::candidate_projection::project_candidate(material, group, set, report)?;
            entry.elapsed_ms = elapsed_ms;
            candidates.push(entry);
        }
    }
    usage.wall_time_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let completeness =
        crate::candidate_reports::completeness(&candidates, &source_capture.reports, cancelled);
    let partial = matches!(completeness, RunCompleteness::Partial { .. });
    let report = CandidateRunReport {
        completeness,
        candidates,
        dropped_samples,
        inputs: source_capture.reports,
        privacy,
        usage,
    };
    Ok(if cancelled {
        CandidateRunOutcome::Cancelled { report }
    } else if partial {
        CandidateRunOutcome::Incomplete { report }
    } else {
        CandidateRunOutcome::CandidateReport { report }
    })
}

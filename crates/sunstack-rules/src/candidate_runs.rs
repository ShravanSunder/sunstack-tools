use crate::{candidate_inputs::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

pub async fn evaluate_candidates(
    input: CandidateRunInput,
    mut context: RuleExecutionContext,
) -> CandidateRunOutcome {
    context.cancellation = context.cancellation.child_token();
    let _cancellation_guard = context.cancellation.clone().drop_guard();
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
    let mut inputs = BTreeSet::new();
    for (position, entry) in input.sources.iter().enumerate() {
        let identity = entry
            .source
            .source_identity()
            .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
        if !inputs.insert(identity) {
            return Err(invalid_candidate_field(vec![
                "sources".into(),
                position.to_string(),
                "source".into(),
            ]));
        }
    }
    input.attempts.validate().map_err(|diagnostic| {
        DiagnosticList::from(sunstack_decisions::DiagnosticList::from(diagnostic))
    })?;
    let definitions = crate::candidate_definitions::load_definitions(&input, &context).await?;
    let sets = &definitions.sets;
    let outside = crate::candidate_admission::admit_candidate_evidence(&input, &definitions)?;
    let plan = crate::candidate_admission::candidate_evidence_plan(&input, &outside);
    let mut source_capture = capture_candidate_sources(&input, &context, plan).await?;
    for report in &mut source_capture.reports {
        for set in sets {
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
    let mut admitted_groups = Vec::new();
    let mut policy = RulePolicy::default();
    let mut router_groups = Vec::new();
    for set in sets {
        let mut selected = discover_rule_candidates(
            set,
            &source_capture.captured,
            input.samples_per_pass,
            &mut privacy,
            &mut dropped_samples,
            &mut source_capture.reports,
            &context.cancellation,
        )?;
        selected.extend(crate::candidate_code_inputs::discover_code_candidates(
            set,
            &source_capture.code,
            input.samples_per_pass,
            &mut privacy,
            &mut dropped_samples,
            &mut source_capture.reports,
            &context.cancellation,
        )?);
        let mut binding_groups = Vec::new();
        for material in selected {
            if matches!(material.selected, CandidateSelection::LeftOutForPrivacy) {
                candidates.push(CandidateEntry {
                    candidate_id: material.candidate_id,
                    rule: material.rule,
                    occurrence: material.occurrence,
                    outcome: CandidateOutcome::NotProcessed {
                        reason: NotProcessedReason::LeftOutForPrivacy,
                    },
                    answers: vec![],
                    evidence: vec![],
                    context: None,
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
            if let Some(router) = &definitions.router {
                let group = router_groups.len();
                let sources = crate::candidate_routing::admit_router_candidate(
                    router,
                    sets,
                    &outside,
                    &material,
                    group,
                    &mut admitted_groups,
                    &mut policy,
                )?;
                router_groups.push(sources);
                materials.push((material, group));
                continue;
            }
            let group = binding_groups.len();
            let mut scoped_bindings = input.evidence.clone();
            scoped_bindings.extend(input.sources[material.entry_position].evidence.clone());
            let mut bindings = scoped_bindings
                .into_iter()
                .filter(|(name, _)| set.definition.inputs.contains_key(name))
                .filter_map(|(name, binding)| match binding {
                    EvidenceBinding::Bound { binding } => Some((name, binding)),
                    EvidenceBinding::Unavailable { .. } => None,
                })
                .collect::<BTreeMap<_, _>>();
            let subject_binding = material.subject_binding()?;
            bindings.insert(target.input.clone(), subject_binding);
            binding_groups.push(BindingGroup { bindings });
            let key = (set.path.clone(), material.entry_position);
            let mut sources = outside
                .entries
                .get(&key)
                .cloned()
                .ok_or_else(|| invalid_candidate_field(vec!["sources".into()]))?;
            for source in sources.values_mut() {
                source.candidate = Some(crate::input_binding::CandidateSourceView::Outside {
                    grants: material.view.grants.iter().copied().collect(),
                });
            }
            sources.insert(target.input.clone(), material.admitted_subject()?);
            admitted_groups.push(crate::invocation_admission::AdmittedGroup {
                set: set.clone(),
                position: group,
                binding: GroupBinding::Bound,
                sources,
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
            discovery: input.discovery.clone(),
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
        let admitted = crate::candidate_admission::CandidateAdmission {
            invocation: crate::invocation_admission::AdmittedInvocation {
                groups: admitted_groups,
                router: definitions.router.as_ref().map(|router| {
                    crate::invocation_admission::AdmittedRouter {
                        definition: router.definition.clone(),
                        groups: router_groups,
                    }
                }),
                evidence: source_capture.plan.clone(),
            },
            session: source_capture.session,
        };
        let mut request_evidence = BTreeMap::new();
        let mut candidate_data = crate::candidate_execution_data::CandidateExecutionData::default();
        let native = crate::rule_execution::evaluate_with_evidence(
            invocation,
            policy,
            context,
            &mut request_evidence,
            &mut candidate_data,
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
        for (private_kind, count) in &candidate_data.outside_privacy.left_out {
            privacy.left_out.push(PrivacyCount {
                route: PrivacyRoute::OutsideEvidence,
                private_kind: *private_kind,
                count: *count,
            });
        }
        if let Some(report) = report {
            usage = crate::candidate_projection::project_candidate_usage(report);
        }
        for (material, group) in materials {
            let set = sets
                .iter()
                .find(|set| set.path == material.rule.rule_set)
                .ok_or_else(|| invalid_candidate_field(vec!["definitions".into()]))?;
            let elapsed_ms = candidate_data
                .elapsed_ms
                .get(&GroupKey {
                    rule_set: material.rule.rule_set.clone(),
                    group,
                })
                .copied();
            let identities = candidate_data.evidence_identities.get(&GroupKey {
                rule_set: material.rule.rule_set.clone(),
                group,
            });
            let evidence_key = (
                definitions.router.as_ref().map_or_else(
                    || material.rule.rule_set.clone(),
                    |router| router.path.clone(),
                ),
                material.entry_position,
            );
            let source_bindings = outside
                .entries
                .get(&evidence_key)
                .ok_or_else(|| invalid_candidate_field(vec!["sources".into()]))?;
            let (evidence, outside_projection) =
                crate::candidate_evidence_reporting::project_evidence_uses(
                    &input,
                    &material,
                    set,
                    report,
                    identities,
                    source_bindings,
                    definitions.router.as_ref(),
                )?;
            for (private_kind, count) in outside_projection.left_out {
                privacy.left_out.push(PrivacyCount {
                    route: PrivacyRoute::OutsideEvidence,
                    private_kind,
                    count,
                });
            }

            let failures = candidate_data.evidence_failures.get(&GroupKey {
                rule_set: material.rule.rule_set.clone(),
                group,
            });
            let unavailable = outside
                .unavailable
                .get(&evidence_key)
                .cloned()
                .unwrap_or_default();
            let mut entry = crate::candidate_projection::project_candidate(
                material,
                group,
                set,
                report,
                definitions.router.as_ref(),
            )?;
            crate::candidate_evidence_outcomes::apply_evidence_failure(
                &mut entry,
                failures.map(Vec::as_slice).unwrap_or_default(),
                &unavailable,
            )?;
            entry.elapsed_ms = elapsed_ms;
            entry.evidence = evidence;
            candidates.push(entry);
        }
    } else {
        source_capture.session.close();
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

use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use sunstack_evidence::*;

pub(crate) struct CapturedCandidateSource {
    pub entry_position: usize,
    pub source_name: SourceName,
    pub path: PathBuf,
    pub format: TranscriptFormatName,
    pub records: Vec<PositionedRecord>,
    pub coverage: CaptureCoverage,
}
pub(crate) struct CandidateSourceCapture {
    pub captured: Vec<CapturedCandidateSource>,
    pub reports: Vec<InputReport>,
    pub privacy: PrivacyProjection,
    pub session: EvidenceSession,
    pub plan: EvidencePlanInput,
}
pub(crate) fn invalid_candidate_field(path: Vec<String>) -> crate::DiagnosticList {
    RuleDiagnostic::new(RuleDiagnosticCode::InvalidConfiguration)
        .definition(Path::new("<candidate_run>"), 1, 1, path)
        .into()
}
pub(crate) async fn capture_candidate_sources(
    input: &CandidateRunInput,
    context: &RuleExecutionContext,
    mut plan: EvidencePlanInput,
) -> Result<CandidateSourceCapture, crate::DiagnosticList> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut recipes = builtin_session_recipes(&home)
        .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?
        .into_iter()
        .map(|recipe| (recipe.provider.clone(), recipe))
        .collect::<BTreeMap<_, _>>();
    let mut overridden = BTreeSet::new();
    for recipe in &input.recipes {
        if !overridden.insert(recipe.provider.clone()) {
            return Err(invalid_candidate_field(vec!["recipes".into()]));
        }
        recipes.insert(recipe.provider.clone(), recipe.clone());
    }
    // Validate all providers before acquiring any source or constructing a backend.
    for (position, entry) in input.sources.iter().enumerate() {
        if let SourceInput::SessionPair { provider, .. } = &entry.source
            && (!matches!(provider.as_str(), "claude_code" | "codex")
                || !recipes.contains_key(provider))
        {
            return Err(invalid_candidate_field(vec![
                "sources".into(),
                position.to_string(),
                "source".into(),
                "provider".into(),
            ]));
        }
    }
    let mut captured = Vec::new();
    let mut reports = Vec::new();
    let mut located_sources = Vec::new();
    let sources = &mut plan.sources;
    for (position, entry) in input.sources.iter().enumerate() {
        let input_id = entry
            .source
            .source_identity()
            .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?;
        let located = match &entry.source {
            SourceInput::SessionPair { provider, session } => {
                let recipe = recipes
                    .get(provider)
                    .ok_or_else(|| invalid_candidate_field(vec!["recipes".into()]))?
                    .clone();
                let session = session.clone();
                tokio::task::spawn_blocking(move || locate_session(&recipe, &session))
                    .await
                    .map_err(|_| {
                        crate::DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError)
                    })?
                    .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?
            }
            SourceInput::CustomEntry { .. } => SessionLookup::Unlocatable {
                reason: UnlocatableReason::Unreadable,
            },
        };
        match located {
            SessionLookup::Located { path, format }
                if context.adapters.event_reader(&format).is_ok()
                    && located_sources.len() < input.limits.max_files.get() =>
            {
                let name = SourceName::new(format!("candidate_input_{position}"))
                    .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?;
                sources.insert(
                    name.clone(),
                    SourceSpec::Conversation {
                        transcript_path: path.clone(),
                        format: format.clone(),
                        final_assistant_text: None,
                    },
                );
                located_sources.push((position, name, path.clone(), format.clone()));
                reports.push(InputReport {
                    input_id,
                    location: InputLocation::Located { path, format },
                    records: 0,
                    unfit: vec![],
                    capture: InputCaptureStatus::Incomplete,
                    discovery: vec![],
                });
            }
            SessionLookup::Located { .. } => reports.push(InputReport {
                input_id,
                location: InputLocation::Unlocatable {
                    reason: UnlocatableReason::Unreadable,
                },
                records: 0,
                unfit: vec![],
                capture: InputCaptureStatus::Incomplete,
                discovery: vec![],
            }),
            SessionLookup::Unlocatable { reason } => reports.push(InputReport {
                input_id,
                location: InputLocation::Unlocatable { reason },
                records: 0,
                unfit: vec![],
                capture: InputCaptureStatus::Incomplete,
                discovery: vec![],
            }),
        }
    }
    let mut session = EvidenceSession::open(
        plan.clone(),
        context.adapters.clone(),
        context.cancellation.clone(),
    )
    .await
    .map_err(|mut error| {
        if error.field_path.is_empty() {
            match error.code {
                EvidenceCode::DiscoveryNotEnabled => error.field_path.push("discovery".into()),
                EvidenceCode::UnknownPayloadSchema | EvidenceCode::PayloadSchemaInvalid => {
                    error.field_path.push("payload_schemas".into())
                }
                _ => {}
            }
        }
        crate::DiagnosticList::from(error.diagnostics())
    })?;
    for (position, name, path, format) in located_sources {
        match session.capture_source(&name).await {
            Ok(CapturedSource::Transcript(capture)) => {
                let events = capture
                    .transcript_lines()
                    .ok_or_else(|| invalid_candidate_field(vec!["sources".into()]))?;
                let projected = project_positioned(&reports[position].input_id, events)
                    .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?;
                reports[position].capture = if capture.tail == CaptureTail::Complete {
                    InputCaptureStatus::Complete
                } else {
                    InputCaptureStatus::Incomplete
                };
                reports[position].records = projected.records.len();
                let mut coverage = capture.coverage.clone();
                coverage.gaps = projected.gaps.clone();
                reports[position].unfit = projected.gaps;
                captured.push(CapturedCandidateSource {
                    entry_position: position,
                    source_name: name,
                    path,
                    format,
                    records: projected.records,
                    coverage,
                });
            }
            Ok(_) | Err(_) => {
                reports[position].location = InputLocation::Unlocatable {
                    reason: UnlocatableReason::Unreadable,
                }
            }
        }
    }
    let mut privacy = PrivacyProjection::default();
    for report in &mut reports {
        let projected = project_coverage_gaps(&report.unfit, &BTreeSet::new())
            .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?;
        report.unfit = projected.gaps;
        for (kind, count) in projected.privacy.left_out {
            *privacy.left_out.entry(kind).or_default() += count;
        }
    }
    Ok(CandidateSourceCapture {
        captured,
        reports,
        privacy,
        session,
        plan,
    })
}

pub(crate) struct CandidateMaterial {
    pub candidate_id: CandidateId,
    pub entry_position: usize,
    pub source_name: SourceName,
    pub rule: RuleRef,
    pub occurrence: SourceOccurrence,
    pub view: PrivacyView,
    pub selected: CandidateRecordSelection,
    pub path: PathBuf,
    pub format: TranscriptFormatName,
}

pub(crate) enum CandidateRecordSelection {
    Ready {
        selected: Arc<crate::input_binding::CandidateRecordView>,
    },
    LeftOutForPrivacy,
}

pub(crate) fn discover_rule_candidates(
    set: &crate::invocation_admission::LoadedRuleSet,
    sources: &[CapturedCandidateSource],
    samples: usize,
    privacy: &mut PrivacyReport,
    dropped: &mut Vec<DroppedSample>,
    reports: &mut [InputReport],
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<Vec<CandidateMaterial>, crate::DiagnosticList> {
    let mut candidates = Vec::new();
    for located_rule in &set.rules {
        let rule = &located_rule.definition;
        if matches!(rule.verdict, VerdictDeclaration::Mechanical { .. }) {
            return Err(
                RuleDiagnostic::new(RuleDiagnosticCode::InvalidConfiguration)
                    .definition(
                        &located_rule.source,
                        located_rule.line,
                        1,
                        vec!["verdict".into()],
                    )
                    .into(),
            );
        }
        let rule_ref = RuleRef {
            rule_set: set.path.clone(),
            rule_id: rule.rule_id.clone(),
        };
        let grants = rule.private_access.iter().copied().collect::<BTreeSet<_>>();
        for private_kind in &grants {
            privacy.granted.push(PrivacyGrant {
                rule: rule_ref.clone(),
                private_kind: *private_kind,
            });
        }
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
            let mut coverage = source.coverage.clone();
            let projected_gaps = project_coverage_gaps(&coverage.gaps, &grants)
                .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?;
            coverage.gaps = projected_gaps.gaps;
            let discovery = source
                .records
                .as_slice()
                .discover(rule.mechanical_pass.as_ref(), samples)
                .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?;
            let mut projected_records = BTreeMap::new();
            for record in &source.records {
                let projected = project_record(record, &grants)
                    .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?;
                append_privacy_counts(privacy, PrivacyRoute::Records, projected.privacy.clone());
                projected_records.insert(record.record_id.clone(), projected);
            }
            for rejected in discovery.rejected_sample {
                let projected = projected_records
                    .get(&rejected.record.record_id)
                    .ok_or_else(|| invalid_candidate_field(vec!["sources".into()]))?;
                append_privacy_counts(privacy, PrivacyRoute::Samples, projected.privacy.clone());
                let excerpt = match &projected.record {
                    Some(record) => serde_json::to_string(&record.body)
                        .map_err(|_| invalid_candidate_field(vec!["sources".into()]))?,
                    None => "[left out for privacy]".into(),
                };
                dropped.push(DroppedSample {
                    rule: rule_ref.clone(),
                    pass: SamplePass::Mechanical,
                    occurrence: rejected.occurrence,
                    excerpt,
                });
            }
            for matched in discovery.matches {
                let projected = projected_records
                    .get(&matched.record.record_id)
                    .ok_or_else(|| invalid_candidate_field(vec!["sources".into()]))?;
                let private_match = match_is_private(&matched.record, &matched.occurrence, &grants)
                    .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?;
                let selected = match &projected.record {
                    Some(record) if !private_match => CandidateRecordSelection::Ready {
                        selected: Arc::new(crate::input_binding::CandidateRecordView {
                            record: record.clone(),
                            coverage: coverage.clone(),
                        }),
                    },
                    _ => CandidateRecordSelection::LeftOutForPrivacy,
                };
                candidates.push(CandidateMaterial {
                    entry_position: source.entry_position,
                    source_name: source.source_name.clone(),
                    candidate_id: CandidateId {
                        rule: rule_ref.clone(),
                        occurrence: matched
                            .occurrence
                            .occurrence_identity()
                            .map_err(|error| crate::DiagnosticList::from(error.diagnostics()))?,
                    },
                    rule: rule_ref.clone(),
                    occurrence: matched.occurrence,
                    view: PrivacyView {
                        grants: rule.private_access.clone(),
                    },
                    selected,
                    path: source.path.clone(),
                    format: source.format.clone(),
                });
            }
            receipt.status = RuleDiscoveryStatus::Completed;
        }
    }
    Ok(candidates)
}
fn append_privacy_counts(
    report: &mut PrivacyReport,
    route: PrivacyRoute,
    projection: PrivacyProjection,
) {
    for (private_kind, count) in projection.left_out {
        report.left_out.push(PrivacyCount {
            route,
            private_kind,
            count,
        });
    }
}

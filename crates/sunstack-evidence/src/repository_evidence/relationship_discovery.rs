use crate::*;
use crate::{
    repository_evidence::relationship_resolution::*, repository_evidence::relationship_syntax::*,
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Instant,
};
use tokio_util::sync::CancellationToken;

pub struct RelationshipDiscoveryRequest<'capture> {
    pub captures: &'capture [CapturedChangeSelection],
    pub seeds: &'capture [SelectedSourceUnit],
    pub query: &'capture RelationshipQuery,
    pub package_bindings: &'capture [PackageSourceBinding],
    pub limits: &'capture DiscoveryLimits,
    pub cancellation: CancellationToken,
}
#[derive(Clone, Debug)]
pub struct RelationshipDiscovery {
    pub seeds: Vec<SelectedSourceUnit>,
    pub units: Vec<SelectedSourceUnit>,
    pub relationships: Vec<RelationshipCandidate>,
    pub coverage: DiscoveryCoverage,
    pub import_context: std::collections::BTreeMap<SourceUnitKey, Vec<SourceSpan>>,
    pub unknown_recency_context: Vec<SourceRef>,
    pub unit_visits: usize,
    pub relationship_candidates: usize,
    pub files_parsed: usize,
    pub bytes_scanned: usize,
}
pub fn discover_relationships(
    request: RelationshipDiscoveryRequest<'_>,
) -> EvidenceResult<RelationshipDiscovery> {
    if request.seeds.is_empty() {
        return Err(EvidenceCode::MissingRelatedContext.into());
    }
    if request.query.directions.is_empty() || request.query.repository_ids.is_empty() {
        return Err(EvidenceCode::InvalidSelector.into());
    }
    let started = Instant::now();
    let mut cutoffs = Vec::new();
    let mut unavailable = DiagnosticList::default();
    for repository in &request.query.repository_ids {
        if !request
            .captures
            .iter()
            .any(|capture| &capture.repository == repository)
        {
            let mut diagnostic = Diagnostic::new(EvidenceCode::SourceUnavailable);
            diagnostic.location = DiagnosticLocation::Field {
                field_path: vec!["repositories".into(), repository.as_str().into()],
            };
            unavailable.push(diagnostic);
        }
    }
    let mut windows = Vec::new();
    let mut files = Vec::new();
    let mut seen_sources = Vec::new();
    for capture in request
        .captures
        .iter()
        .filter(|capture| request.query.repository_ids.contains(&capture.repository))
    {
        match &capture.coverage {
            DiscoveryCoverage::Limited {
                cutoffs: capture_cutoffs,
                query_bounds,
            } => {
                cutoffs.extend(capture_cutoffs.clone());
                windows.extend(query_bounds.windows.clone());
            }
            DiscoveryCoverage::Unavailable { diagnostics } => {
                unavailable.items.extend(diagnostics.items.clone())
            }
            DiscoveryCoverage::CompleteWithinDeclaredScope { query_bounds } => {
                windows.extend(query_bounds.windows.clone())
            }
        }
        for file in &capture.sources {
            if request
                .seeds
                .iter()
                .any(|seed| seed.unit.key.repository == capture.repository)
                && ((request.query.source_units.source_sides == SourceSides::AfterOnly
                    && file.side == SourceSide::Before)
                    || (request.query.source_units.source_sides == SourceSides::BeforeOnly
                        && file.side == SourceSide::After))
            {
                continue;
            }
            if request.cancellation.is_cancelled() {
                return Err(EvidenceCode::MissingContext.into());
            }
            if started.elapsed().as_millis() >= request.limits.max_local_time_ms.get() as u128 {
                add_cutoff(
                    &mut cutoffs,
                    DiscoveryLimit::LocalTime,
                    request
                        .seeds
                        .iter()
                        .map(|unit| unit.unit.key.clone())
                        .collect(),
                );
                break;
            }
            if seen_sources.contains(&file.source) {
                continue;
            }
            seen_sources.push(file.source.clone());
            if let Some(parsed) = parse_relationship_file(file, &capture.snapshot)? {
                files.push(parsed);
            }
        }
    }
    let mut all_units = BTreeMap::new();
    let mut import_context = BTreeMap::new();
    for file in &files {
        for declaration in &file.declarations {
            all_units.insert(declaration.unit.unit.key.clone(), declaration.unit.clone());
            import_context.insert(
                declaration.unit.unit.key.clone(),
                file.imports
                    .iter()
                    .map(|import| import.span.clone())
                    .collect(),
            );
        }
    }
    let mut edges = Vec::new();
    let include_calls = request.query.directions.iter().any(|direction| {
        matches!(
            direction,
            RelationDirection::Callees | RelationDirection::Callers
        )
    });
    let include_imports = request.query.directions.iter().any(|direction| {
        matches!(
            direction,
            RelationDirection::Imports | RelationDirection::ImportedBy
        )
    });
    'files: for file in &files {
        for call in file.calls.iter().filter(|_| include_calls) {
            if edges.len() >= request.limits.max_relationship_candidates.get() {
                add_cutoff(
                    &mut cutoffs,
                    DiscoveryLimit::RelationshipCandidates,
                    vec![call.source_unit.clone()],
                );
                break 'files;
            }
            let targets = call_endpoints(file, &call.target_hint, &files, request.package_bindings);
            if call.dynamic || targets.is_empty() {
                edges.push(RelationshipCandidate::Unresolved {
                    source_unit: call.source_unit.clone(),
                    target_hint: call.target_hint.clone(),
                    relation_kind: RelationDirection::Callees,
                    basis_spans: vec![call.span.clone()],
                    reason: if call.dynamic {
                        RelationshipUncertainty::DynamicDispatch
                    } else {
                        RelationshipUncertainty::MissingSource
                    },
                });
                continue;
            }
            for endpoint in targets.iter().copied() {
                let ambiguous = endpoint_is_ambiguous(&endpoint, &targets);
                if edges.len() >= request.limits.max_relationship_candidates.get() {
                    add_cutoff(
                        &mut cutoffs,
                        DiscoveryLimit::RelationshipCandidates,
                        vec![call.source_unit.clone()],
                    );
                    break 'files;
                }
                if call.shadowed {
                    edges.push(RelationshipCandidate::Contradicted {
                        source_unit: call.source_unit.clone(),
                        possible_target: endpoint.declaration.unit.unit.key.clone(),
                        basis_spans: vec![
                            call.span.clone(),
                            endpoint.declaration.unit.unit.span.clone(),
                        ],
                        contradiction: RelationshipContradiction::LexicalShadow,
                    });
                } else {
                    edges.push(candidate(
                        &call.source_unit,
                        &call.span,
                        endpoint,
                        RelationDirection::Callees,
                        ambiguous,
                    ));
                }
            }
        }
        for import in file.imports.iter().filter(|_| include_imports) {
            let targets = import_endpoints(file, import, &files, request.package_bindings);
            for source in &file.declarations {
                if targets.is_empty() {
                    if edges.len() >= request.limits.max_relationship_candidates.get() {
                        add_cutoff(
                            &mut cutoffs,
                            DiscoveryLimit::RelationshipCandidates,
                            vec![source.unit.unit.key.clone()],
                        );
                        break 'files;
                    }
                    edges.push(RelationshipCandidate::Unresolved {
                        source_unit: source.unit.unit.key.clone(),
                        target_hint: import.module_path.clone(),
                        relation_kind: RelationDirection::Imports,
                        basis_spans: vec![import.span.clone()],
                        reason: if import.unsupported {
                            RelationshipUncertainty::UnsupportedImport
                        } else {
                            RelationshipUncertainty::UnresolvedPackage
                        },
                    });
                }
                for endpoint in &targets {
                    if edges.len() >= request.limits.max_relationship_candidates.get() {
                        add_cutoff(
                            &mut cutoffs,
                            DiscoveryLimit::RelationshipCandidates,
                            vec![source.unit.unit.key.clone()],
                        );
                        break 'files;
                    }
                    edges.push(candidate(
                        &source.unit.unit.key,
                        &import.span,
                        ResolvedEndpoint {
                            declaration: endpoint.declaration,
                            import: endpoint.import,
                            mapping: endpoint.mapping,
                        },
                        RelationDirection::Imports,
                        endpoint_is_ambiguous(endpoint, &targets),
                    ));
                }
            }
        }
    }
    let mut selected: BTreeMap<_, _> = request
        .seeds
        .iter()
        .map(|unit| (unit.unit.key.clone(), unit.clone()))
        .collect();
    let mut queue: VecDeque<_> = request
        .seeds
        .iter()
        .map(|unit| (unit.unit.key.clone(), 0usize))
        .collect();
    let mut visited = BTreeSet::new();
    let mut relationships = Vec::new();
    let mut visits = 0usize;
    while let Some((unit, depth)) = queue.pop_front() {
        if request.cancellation.is_cancelled() {
            return Err(EvidenceCode::MissingContext.into());
        }
        if depth >= request.query.requested_hops.get() {
            continue;
        }
        if started.elapsed().as_millis() >= request.limits.max_local_time_ms.get() as u128 {
            add_cutoff(&mut cutoffs, DiscoveryLimit::LocalTime, vec![unit]);
            break;
        }
        for direction in &request.query.directions {
            if !visited.insert((unit.clone(), *direction, depth)) {
                continue;
            }
            if visits >= request.limits.max_unit_visits.get() {
                add_cutoff(&mut cutoffs, DiscoveryLimit::UnitVisits, vec![unit.clone()]);
                continue;
            }
            visits += 1;
            let mut fan_out = 0usize;
            for edge in &edges {
                let Some((mut relation, neighbor)) = follow_edge(edge, &unit, *direction) else {
                    continue;
                };
                if neighbor.is_some() {
                    if fan_out >= request.limits.max_fan_out.get() {
                        add_cutoff(&mut cutoffs, DiscoveryLimit::FanOut, vec![unit.clone()]);
                        break;
                    }
                    fan_out += 1;
                }
                if let RelationshipCandidate::Candidate { relation_kind, .. }
                | RelationshipCandidate::Unresolved { relation_kind, .. } = &mut relation
                {
                    *relation_kind = *direction;
                }
                if !relationships.contains(&relation) {
                    relationships.push(relation);
                }
                if let Some(neighbor) = neighbor
                    && let Some(located) = all_units.get(neighbor)
                {
                    selected.insert(neighbor.clone(), located.clone());
                    queue.push_back((neighbor.clone(), depth + 1));
                }
            }
        }
    }
    let query_bounds = QueryBounds {
        repository_ids: request.query.repository_ids.clone(),
        requested_hops: request.query.requested_hops.get(),
        windows,
    };
    let coverage = if !unavailable.items.is_empty() {
        DiscoveryCoverage::Unavailable {
            diagnostics: unavailable,
        }
    } else if cutoffs.is_empty() {
        DiscoveryCoverage::CompleteWithinDeclaredScope { query_bounds }
    } else {
        DiscoveryCoverage::Limited {
            query_bounds,
            cutoffs,
        }
    };
    let files_parsed = files.len();
    let bytes_scanned = files.iter().map(|file| file.file.text.len()).sum();
    Ok(RelationshipDiscovery {
        seeds: request.seeds.to_vec(),
        units: selected.into_values().collect(),
        relationships,
        coverage,
        import_context,
        unknown_recency_context: Vec::new(),
        unit_visits: visits,
        relationship_candidates: edges.len(),
        files_parsed,
        bytes_scanned,
    })
}

fn add_cutoff(
    cutoffs: &mut Vec<DiscoveryCutoff>,
    limit: DiscoveryLimit,
    frontier: Vec<SourceUnitKey>,
) {
    if let Some(cutoff) = cutoffs.iter_mut().find(|cutoff| cutoff.limit == limit) {
        for unit in frontier {
            if !cutoff.frontier.contains(&unit) {
                cutoff.frontier.push(unit);
            }
        }
    } else {
        cutoffs.push(DiscoveryCutoff { limit, frontier });
    }
}
fn candidate(
    source: &SourceUnitKey,
    span: &SourceSpan,
    endpoint: ResolvedEndpoint<'_>,
    relation_kind: RelationDirection,
    ambiguous: bool,
) -> RelationshipCandidate {
    let target = &endpoint.declaration.unit.unit;
    let mut basis_spans = vec![span.clone(), target.span.clone()];
    let mut facts = vec![RelationshipFact::Declaration {
        span: target.span.clone(),
    }];
    if relation_kind == RelationDirection::Callees {
        facts.push(RelationshipFact::StaticCall { span: span.clone() });
    }
    if let Some(import) = endpoint.import {
        basis_spans.push(import.span.clone());
        facts.push(RelationshipFact::ImportBinding {
            span: import.span.clone(),
        });
    }
    if let Some(mapping) = endpoint.mapping {
        facts.push(RelationshipFact::PackageMapping {
            importing_repository: mapping.importing_repository_id.clone(),
            target_repository: mapping.target_repository_id.clone(),
        });
    }
    let mut uncertainties = vec![RelationshipUncertainty::SyntaxOnly];
    if ambiguous {
        uncertainties.push(RelationshipUncertainty::AmbiguousDeclaration);
    }
    if target.syntax_quality == SyntaxQuality::RecoveredWithErrors {
        uncertainties.push(RelationshipUncertainty::RecoveredSyntax);
    }
    RelationshipCandidate::Candidate {
        source_unit: source.clone(),
        target_unit: target.key.clone(),
        call_site_span: span.clone(),
        relation_kind,
        basis_spans,
        facts,
        uncertainties,
    }
}
fn follow_edge<'edge>(
    edge: &'edge RelationshipCandidate,
    unit: &SourceUnitKey,
    direction: RelationDirection,
) -> Option<(RelationshipCandidate, Option<&'edge SourceUnitKey>)> {
    let (base, reverse) = match direction {
        RelationDirection::Callers => (RelationDirection::Callees, true),
        RelationDirection::Callees => (RelationDirection::Callees, false),
        RelationDirection::Imports => (RelationDirection::Imports, false),
        RelationDirection::ImportedBy => (RelationDirection::Imports, true),
    };
    match edge {
        RelationshipCandidate::Candidate {
            source_unit,
            target_unit,
            relation_kind,
            ..
        } if *relation_kind == base => {
            if reverse && target_unit == unit {
                Some((edge.clone(), Some(source_unit)))
            } else if !reverse && source_unit == unit {
                Some((edge.clone(), Some(target_unit)))
            } else {
                None
            }
        }
        RelationshipCandidate::Contradicted {
            source_unit,
            possible_target,
            ..
        } if base == RelationDirection::Callees
            && ((!reverse && source_unit == unit) || (reverse && possible_target == unit)) =>
        {
            Some((edge.clone(), None))
        }
        RelationshipCandidate::Unresolved {
            source_unit,
            relation_kind,
            ..
        } if !reverse && *relation_kind == base && source_unit == unit => {
            Some((edge.clone(), None))
        }
        _ => None,
    }
}

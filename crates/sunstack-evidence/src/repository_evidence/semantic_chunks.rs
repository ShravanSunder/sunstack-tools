use crate::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub fn build_semantic_groups(
    discovery: &RelationshipDiscovery,
    settings: &ChunkSettings,
    minimum_related_units: usize,
) -> EvidenceResult<Vec<ContextGroupOutcome>> {
    if discovery.seeds.is_empty() {
        return Ok(vec![ContextGroupOutcome::Unavailable {
            diagnostics: EvidenceError::from(EvidenceCode::MissingRelatedContext).diagnostics(),
        }]);
    }
    match settings.grouping {
        ChunkGrouping::SingleGroup => Ok(vec![build_group(
            discovery,
            settings,
            minimum_related_units,
        )?]),
        ChunkGrouping::PerSeed => discovery
            .seeds
            .iter()
            .map(|seed| {
                build_group(
                    &seed_neighborhood(discovery, seed),
                    settings,
                    minimum_related_units,
                )
            })
            .collect(),
    }
}

fn build_group(
    discovery: &RelationshipDiscovery,
    settings: &ChunkSettings,
    minimum_related_units: usize,
) -> EvidenceResult<ContextGroupOutcome> {
    let seeds: BTreeSet<_> = discovery
        .seeds
        .iter()
        .map(|unit| unit.unit.key.clone())
        .collect();
    let units: BTreeMap<_, _> = discovery
        .units
        .iter()
        .map(|unit| (unit.unit.key.clone(), unit))
        .collect();
    let mut diagnostics = DiagnosticList::default();
    if units.keys().filter(|key| !seeds.contains(*key)).count() < minimum_related_units {
        diagnostics.push(EvidenceError::from(EvidenceCode::MissingRelatedContext).diagnostic());
    }
    match &discovery.coverage {
        DiscoveryCoverage::Limited { .. } => {
            diagnostics.push(Diagnostic::new(EvidenceCode::DiscoveryLimit))
        }
        DiscoveryCoverage::Unavailable {
            diagnostics: unavailable,
        } => {
            for diagnostic in &unavailable.items {
                diagnostics.push(diagnostic.clone());
            }
        }
        DiscoveryCoverage::CompleteWithinDeclaredScope { .. } => {}
    }
    if discovery
        .relationships
        .iter()
        .any(|relationship| match relationship {
            RelationshipCandidate::Unresolved { .. } => true,
            RelationshipCandidate::Candidate { uncertainties, .. } => {
                uncertainties.iter().any(|uncertainty| {
                    matches!(
                        uncertainty,
                        RelationshipUncertainty::AmbiguousDeclaration
                            | RelationshipUncertainty::RecoveredSyntax
                    )
                })
            }
            RelationshipCandidate::Contradicted { .. } => false,
        })
    {
        diagnostics.push(EvidenceError::from(EvidenceCode::RelationshipUnresolved).diagnostic());
    }
    if discovery
        .units
        .iter()
        .any(|unit| unit.unit.syntax_quality == SyntaxQuality::RecoveredWithErrors)
    {
        diagnostics.push(EvidenceError::from(EvidenceCode::RequiredGroupIncomplete).diagnostic());
    }
    let mut ordered = units.into_values().collect::<Vec<_>>();
    ordered.sort_by_key(|unit| (!seeds.contains(&unit.unit.key), unit.unit.key.clone()));
    let mut bytes = 0usize;
    let mut excerpts = Vec::new();
    let mut omitted = 0usize;
    for selected in ordered {
        let membership = if seeds.contains(&selected.unit.key) {
            &settings.seed_content
        } else {
            &settings.related_content
        };
        let requested = match membership {
            UnitContent::WholeUnit {} => selected.text.len(),
            UnitContent::BoundedExcerpt { max_bytes } => selected.text.len().min(max_bytes.get()),
        };
        let mut end = requested;
        while !selected.text.is_char_boundary(end) {
            end -= 1;
        }
        if bytes.saturating_add(end) > settings.max_chunk_bytes.get() {
            omitted += 1;
            continue;
        }
        let text = selected.text[..end].to_string();
        let mut span = selected.unit.span.clone();
        span.byte_end = span.byte_start + end;
        let line_count = text.bytes().filter(|byte| *byte == b'\n').count();
        span.end_line = span.start_line + line_count;
        span.end_column = if line_count == 0 {
            span.start_column + text.chars().count()
        } else {
            text.rsplit('\n')
                .next()
                .map_or(1, |line| line.chars().count() + 1)
        };
        let context = match &selected.unit.kind {
            SourceUnitKind::Function { signature }
            | SourceUnitKind::Method { signature }
            | SourceUnitKind::Class { signature } => ExcerptContext::Code {
                enclosing_signature: signature.clone(),
                import_context: discovery
                    .import_context
                    .get(&selected.unit.key)
                    .cloned()
                    .unwrap_or_default(),
            },
            SourceUnitKind::Paragraph | SourceUnitKind::Section => ExcerptContext::Markdown,
        };
        bytes += text.len();
        excerpts.push(Excerpt {
            unit: selected.unit.key.clone(),
            span,
            text,
            membership: membership.clone(),
            context,
            content_identity: selected.unit.content_identity.clone(),
        });
    }
    let omissions = if omitted == 0 {
        vec![]
    } else {
        diagnostics.push(EvidenceError::from(EvidenceCode::RequiredGroupIncomplete).diagnostic());
        vec![Omission {
            reason: OmissionReason::Limit,
            count: omitted,
        }]
    };
    let unit_keys = discovery
        .units
        .iter()
        .map(|selected| selected.unit.key.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let unknown_recency_context = discovery
        .unknown_recency_context
        .iter()
        .filter(|source| {
            excerpts
                .iter()
                .any(|excerpt| &excerpt.span.source == *source)
        })
        .cloned()
        .collect::<Vec<_>>();
    let identity = serde_json::to_vec(&(
        &unit_keys,
        &discovery.relationships,
        &excerpts,
        &unknown_recency_context,
    ))
    .map_err(|_| EvidenceCode::InvalidContextGroup)?;
    let group = SemanticChunk {
        key: content_digest(&identity),
        units: unit_keys,
        relationships: discovery.relationships.clone(),
        excerpts,
        omitted_optional: omissions.clone(),
        unknown_recency_context,
        coverage: discovery.coverage.clone(),
    };
    if diagnostics.items.is_empty() {
        Ok(ContextGroupOutcome::Complete {
            group,
            coverage: discovery.coverage.clone(),
        })
    } else {
        Ok(ContextGroupOutcome::Partial {
            group,
            coverage: discovery.coverage.clone(),
            omissions,
            diagnostics,
        })
    }
}

fn seed_neighborhood(
    discovery: &RelationshipDiscovery,
    seed: &SelectedSourceUnit,
) -> RelationshipDiscovery {
    let hops = match &discovery.coverage {
        DiscoveryCoverage::CompleteWithinDeclaredScope { query_bounds }
        | DiscoveryCoverage::Limited { query_bounds, .. } => query_bounds.requested_hops,
        DiscoveryCoverage::Unavailable { .. } => 0,
    };
    let mut selected = BTreeSet::from([seed.unit.key.clone()]);
    let mut queue = VecDeque::from([(seed.unit.key.clone(), 0)]);
    let mut visited = BTreeSet::new();
    let mut relationships = Vec::new();
    while let Some((unit, depth)) = queue.pop_front() {
        if depth >= hops || !visited.insert((unit.clone(), depth)) {
            continue;
        }
        for relationship in &discovery.relationships {
            let neighbor = match relationship {
                RelationshipCandidate::Candidate {
                    source_unit,
                    target_unit,
                    relation_kind,
                    ..
                } => match relation_kind {
                    RelationDirection::Callers | RelationDirection::ImportedBy
                        if target_unit == &unit =>
                    {
                        Some(source_unit)
                    }
                    RelationDirection::Callees | RelationDirection::Imports
                        if source_unit == &unit =>
                    {
                        Some(target_unit)
                    }
                    _ => continue,
                },
                RelationshipCandidate::Contradicted {
                    source_unit,
                    possible_target,
                    ..
                } if source_unit == &unit || possible_target == &unit => None,
                RelationshipCandidate::Unresolved { source_unit, .. } if source_unit == &unit => {
                    None
                }
                _ => continue,
            };
            if !relationships.contains(relationship) {
                relationships.push(relationship.clone());
            }
            if let Some(neighbor) = neighbor {
                selected.insert(neighbor.clone());
                queue.push_back((neighbor.clone(), depth + 1));
            }
        }
    }
    let coverage = match &discovery.coverage {
        DiscoveryCoverage::Limited {
            query_bounds,
            cutoffs,
        } => {
            let cutoffs = cutoffs
                .iter()
                .filter(|cutoff| {
                    !matches!(
                        cutoff.limit,
                        DiscoveryLimit::UnitVisits | DiscoveryLimit::FanOut
                    ) || cutoff.frontier.is_empty()
                        || cutoff.frontier.iter().any(|unit| selected.contains(unit))
                })
                .cloned()
                .collect::<Vec<_>>();
            if cutoffs.is_empty() {
                DiscoveryCoverage::CompleteWithinDeclaredScope {
                    query_bounds: query_bounds.clone(),
                }
            } else {
                DiscoveryCoverage::Limited {
                    query_bounds: query_bounds.clone(),
                    cutoffs,
                }
            }
        }
        other => other.clone(),
    };
    RelationshipDiscovery {
        seeds: vec![seed.clone()],
        units: discovery
            .units
            .iter()
            .filter(|unit| selected.contains(&unit.unit.key))
            .cloned()
            .collect(),
        relationships,
        coverage,
        import_context: discovery.import_context.clone(),
        unknown_recency_context: discovery.unknown_recency_context.clone(),
        unit_visits: discovery.unit_visits,
        relationship_candidates: discovery.relationship_candidates,
        files_parsed: discovery.files_parsed,
        bytes_scanned: discovery.bytes_scanned,
    }
}

#[cfg(test)]
#[path = "semantic_chunks_test.rs"]
mod tests;

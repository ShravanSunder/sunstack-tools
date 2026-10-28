use crate::*;
use std::collections::BTreeSet;
impl CodeContextIndex {
    pub(crate) fn containing_unit(&self, span: &SourceSpan) -> Option<&SelectedSourceUnit> {
        self.units
            .iter()
            .filter(|unit| {
                unit.unit.span.source == span.source
                    && unit.unit.key.byte_start <= span.byte_start
                    && span.byte_end <= unit.unit.key.byte_end
            })
            .min_by_key(|unit| unit.unit.key.byte_end - unit.unit.key.byte_start)
    }
    pub(crate) fn touched_units(&self) -> BTreeSet<SourceUnitKey> {
        self.units
            .iter()
            .filter(|unit| {
                self.changes
                    .iter()
                    .flat_map(|hunk| [hunk.before.as_ref(), hunk.after.as_ref()])
                    .flatten()
                    .any(|change| same_source_overlap(&change.span, &unit.unit.span))
            })
            .map(|unit| unit.unit.key.clone())
            .collect()
    }
    pub(crate) fn eligible_units(&self) -> BTreeSet<SourceUnitKey> {
        let touched = self.touched_units();
        let mut reachable = self
            .units
            .iter()
            .filter(|unit| {
                self.units.iter().any(|parent| {
                    touched.contains(&parent.unit.key)
                        && parent.unit.span.source == unit.unit.span.source
                        && parent.unit.key.byte_start <= unit.unit.key.byte_start
                        && unit.unit.key.byte_end <= parent.unit.key.byte_end
                })
            })
            .map(|unit| unit.unit.key.clone())
            .collect::<BTreeSet<_>>();
        loop {
            let before = reachable.len();
            for relation in &self.relationships {
                if let RelationshipCandidate::Candidate {
                    source_unit,
                    target_unit,
                    relation_kind: RelationDirection::Callees,
                    ..
                } = relation
                    && reachable.contains(source_unit)
                {
                    reachable.insert(target_unit.clone());
                }
            }
            if reachable.len() == before {
                break;
            }
        }
        reachable
    }
    pub fn candidate_context(
        &self,
        occurrence: &SourceOccurrence,
    ) -> EvidenceResult<CandidateContext> {
        let (unit, span) = match occurrence {
            SourceOccurrence::CodeMatch {
                input_id,
                unit,
                span,
            }
            | SourceOccurrence::ContextMatch {
                input_id,
                unit,
                span,
                ..
            } if *input_id == self.input_id => (unit, span),
            _ => return Err(EvidenceCode::InvalidSelector.into()),
        };
        let file = self
            .capture
            .sources
            .iter()
            .find(|file| file.source == span.source)
            .ok_or(EvidenceCode::MissingContext)?;
        let matched = CodeExcerpt {
            span: span.clone(),
            text: file
                .text
                .get(span.byte_start..span.byte_end)
                .ok_or(EvidenceCode::InvalidSelector)?
                .into(),
        };
        let identity = occurrence.occurrence_identity()?;
        let mut selected = BTreeSet::new();
        let mut seeds = Vec::new();
        if let Some(unit) = unit {
            selected.insert(unit.clone());
            seeds.push(unit.clone());
            if matches!(occurrence, SourceOccurrence::ContextMatch { .. }) {
                for root in self.touched_units() {
                    let mut reachable = BTreeSet::from([root.clone()]);
                    loop {
                        let before = reachable.len();
                        for relation in &self.relationships {
                            if let RelationshipCandidate::Candidate {
                                source_unit,
                                target_unit,
                                relation_kind: RelationDirection::Callees,
                                ..
                            } = relation
                                && reachable.contains(source_unit)
                            {
                                reachable.insert(target_unit.clone());
                            }
                        }
                        if before == reachable.len() {
                            break;
                        }
                    }
                    let contains = self
                        .units
                        .iter()
                        .find(|candidate| candidate.unit.key == root)
                        .is_some_and(|parent| {
                            parent.unit.span.source == span.source
                                && parent.unit.key.byte_start <= span.byte_start
                                && span.byte_end <= parent.unit.key.byte_end
                        });
                    if (contains || reachable.contains(unit)) && selected.insert(root.clone()) {
                        seeds.push(root);
                    }
                }
            }
            for key in seeds.clone() {
                let current = self
                    .units
                    .iter()
                    .find(|candidate| candidate.unit.key == key)
                    .ok_or(EvidenceCode::MissingContext)?;
                for hunk in &self.changes {
                    if [hunk.before.as_ref(), hunk.after.as_ref()]
                        .into_iter()
                        .flatten()
                        .any(|change| same_source_overlap(&change.span, &current.unit.span))
                    {
                        for other in &self.units {
                            if [hunk.before.as_ref(), hunk.after.as_ref()]
                                .into_iter()
                                .flatten()
                                .any(|change| same_source_overlap(&change.span, &other.unit.span))
                                && selected.insert(other.unit.key.clone())
                            {
                                seeds.push(other.unit.key.clone());
                            }
                        }
                    }
                }
            }
        }
        let mut related = Vec::new();
        for seed in &seeds {
            for relation in &self.relationships {
                let projected = match relation {
                    RelationshipCandidate::Candidate {
                        source_unit,
                        target_unit,
                        relation_kind,
                        facts,
                        uncertainties,
                        ..
                    } => {
                        let (owner, target) = if matches!(
                            relation_kind,
                            RelationDirection::Callers | RelationDirection::ImportedBy
                        ) {
                            (target_unit, source_unit)
                        } else {
                            (source_unit, target_unit)
                        };
                        if owner != seed {
                            continue;
                        }
                        Some((
                            target.clone(),
                            *relation_kind,
                            facts.clone(),
                            uncertainties.clone(),
                            vec![],
                        ))
                    }
                    RelationshipCandidate::Contradicted {
                        source_unit,
                        possible_target,
                        contradiction,
                        ..
                    } => {
                        if source_unit == seed {
                            Some((
                                possible_target.clone(),
                                RelationDirection::Callees,
                                vec![],
                                vec![],
                                vec![*contradiction],
                            ))
                        } else if possible_target == seed {
                            Some((
                                source_unit.clone(),
                                RelationDirection::Callers,
                                vec![],
                                vec![],
                                vec![*contradiction],
                            ))
                        } else {
                            None
                        }
                    }
                    RelationshipCandidate::Unresolved {
                        source_unit,
                        target_hint,
                        relation_kind,
                        basis_spans,
                        reason,
                    } => {
                        if source_unit == seed {
                            let call_site = basis_spans
                                .first()
                                .ok_or(EvidenceCode::MissingRelatedContext)?
                                .clone();
                            let bytes = serde_json::to_vec(&(seed, relation))
                                .map_err(|_| EvidenceCode::InvalidSelector)?;
                            related.push(RelatedPiece::Unresolved {
                                piece_id: PieceId::new(content_digest(&bytes))?,
                                seed: seed.clone(),
                                connection: *relation_kind,
                                name_as_written: target_hint.clone(),
                                call_site,
                                reason: *reason,
                            });
                        }
                        None
                    }
                };
                if let Some((target, connection, facts, uncertainties, contradictions)) = projected
                {
                    selected.insert(target.clone());
                    let bytes = serde_json::to_vec(&(
                        seed,
                        &target,
                        connection,
                        &facts,
                        &uncertainties,
                        &contradictions,
                    ))
                    .map_err(|_| EvidenceCode::InvalidSelector)?;
                    related.push(RelatedPiece::Resolved {
                        piece_id: PieceId::new(content_digest(&bytes))?,
                        seed: seed.clone(),
                        connection,
                        unit: target,
                        facts,
                        uncertainties,
                        contradictions,
                    });
                }
            }
        }
        let units = self
            .units
            .iter()
            .filter(|unit| selected.contains(&unit.unit.key))
            .map(|unit| {
                let bytes = serde_json::to_vec(&unit.unit.key)
                    .map_err(|_| EvidenceCode::InvalidSelector)?;
                Ok(ContextUnit {
                    piece_id: PieceId::new(format!("unit:{}", content_digest(&bytes)))?,
                    unit: unit.unit.clone(),
                    text: unit.text.clone(),
                })
            })
            .collect::<EvidenceResult<Vec<_>>>()?;
        let changes = self
            .changes
            .iter()
            .filter(|hunk| {
                [hunk.before.as_ref(), hunk.after.as_ref()]
                    .into_iter()
                    .flatten()
                    .any(|change| {
                        same_source_overlap(&change.span, span)
                            || units
                                .iter()
                                .any(|unit| same_source_overlap(&change.span, &unit.unit.span))
                    })
            })
            .cloned()
            .collect();
        let mut imports = Vec::new();
        for key in &selected {
            for import in self.imports.get(key).into_iter().flatten() {
                if imports
                    .iter()
                    .any(|excerpt: &CodeExcerpt| excerpt.span == *import)
                {
                    continue;
                }
                if let Some(file) = self
                    .capture
                    .sources
                    .iter()
                    .find(|file| file.source == import.source)
                {
                    imports.push(CodeExcerpt {
                        span: import.clone(),
                        text: file
                            .text
                            .get(import.byte_start..import.byte_end)
                            .ok_or(EvidenceCode::InvalidSelector)?
                            .into(),
                    });
                }
            }
        }
        Ok(CandidateContext {
            context_id: CodeContextId::new(content_digest(
                &serde_json::to_vec(&(identity, &self.capture.snapshot))
                    .map_err(|_| EvidenceCode::InvalidSelector)?,
            ))?,
            input_id: self.input_id.clone(),
            repository: self.repository.clone(),
            matched,
            changes,
            units,
            redactions: vec![],
            related,
            imports,
            coverage: self.coverage.clone(),
        })
    }
}
pub(crate) fn same_source_overlap(left: &SourceSpan, right: &SourceSpan) -> bool {
    left.source == right.source
        && if left.byte_start == left.byte_end {
            right.byte_start <= left.byte_start && left.byte_start < right.byte_end
        } else {
            left.byte_start < right.byte_end && right.byte_start < left.byte_end
        }
}

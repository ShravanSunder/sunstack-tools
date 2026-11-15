use super::*;

impl EvidenceBuild {
    pub(super) fn project_fields(
        &self,
        kind: &EvidenceKindName,
        definition: &EvidenceKindDefinition,
        fields: &BTreeMap<FieldName, FieldValue>,
    ) -> crate::ModelView {
        let limit = definition
            .max_chars
            .map_or(self.limits.evidence_max_chars, |value| value.get() as usize);
        crate::project_kind_model_view(&self.project, kind, definition, fields, limit)
    }
    pub(super) fn project_selection_fields(
        &self,
        kind: &EvidenceKindName,
        definition: &EvidenceKindDefinition,
        fields: &BTreeMap<FieldName, FieldValue>,
        max_chars: usize,
    ) -> crate::ModelView {
        let available = fields
            .iter()
            .filter(|(_, value)| !matches!(value, FieldValue::Unavailable { .. }))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        crate::project_kind_model_view(&self.project, kind, definition, &available, max_chars)
    }
    fn unresolved_ranges(
        &self,
        kind: &EvidenceKindName,
        file: &MatchedFile,
    ) -> Vec<(MatchRange, SelectionDeferral)> {
        let mut ranges = self
            .unresolved_selections
            .iter()
            .filter(|(candidate, _)| {
                candidate.kind == *kind && candidate.span.source == file.source
            })
            .map(|(candidate, reason)| {
                (
                    MatchRange {
                        start: candidate.span.byte_start,
                        end: candidate.span.byte_end,
                    },
                    *reason,
                )
            })
            .collect::<Vec<_>>();
        let definition = &self.kinds[kind];
        if let Some(parents) = &definition.within {
            let unknown = parents
                .as_slice()
                .iter()
                .flat_map(|parent| self.unresolved_ranges(parent, file))
                .collect::<Vec<_>>();
            for range in &file.kind_ranges(kind, definition) {
                let exists = self.catalog.pieces.iter().any(|piece| {
                    parents.as_slice().contains(&piece.kind)
                        && piece.location.span.source == file.source
                        && match_within(
                            range,
                            &MatchRange {
                                start: piece.location.span.byte_start,
                                end: piece.location.span.byte_end,
                            },
                        )
                });
                if !exists
                    && let Some(reason) = unknown
                        .iter()
                        .filter(|(parent, _)| match_within(range, parent))
                        .map(|(_, reason)| *reason)
                        .max()
                {
                    ranges.push((range.clone(), reason));
                }
            }
        }
        ranges
    }
    pub(super) fn defer_fields(
        &self,
        definition: &EvidenceKindDefinition,
        file: &MatchedFile,
        range: &MatchRange,
        fields: &mut BTreeMap<FieldName, FieldValue>,
    ) {
        if self.unresolved_selections.is_empty() {
            return;
        }
        let mut deferred_inside = None;
        for (name, source) in &definition.fields {
            if let FieldSource::ScopedInside { kind, scope } = source {
                let scope_range = super::section_scoped_fields::scoped_range(
                    file,
                    scope,
                    range,
                    &self.catalog.pieces,
                );
                let outline_deferral = scope
                    .kinds()
                    .iter()
                    .flat_map(|kind| {
                        self.unresolved_ranges(kind, file).into_iter().filter_map(
                            |(unknown, reason)| {
                                (match_within(range, &unknown)
                                    || (matches!(scope, crate::SectionScope::Own { .. })
                                        && scope_range.as_ref().is_ok_and(|scope_range| {
                                            unknown.start > scope_range.start
                                                && unknown.start < scope_range.end
                                        })))
                                .then_some(reason)
                            },
                        )
                    })
                    .max();
                let piece_deferral = scope_range.ok().and_then(|scope_range| {
                    self.unresolved_ranges(kind, file)
                        .into_iter()
                        .filter(|(unknown, _)| match_within(unknown, &scope_range))
                        .map(|(_, reason)| reason)
                        .max()
                });
                if let Some(reason) = outline_deferral.max(piece_deferral) {
                    fields.insert(
                        name.clone(),
                        FieldValue::Unavailable {
                            reason: reason.field_unavailable(),
                        },
                    );
                }
                continue;
            }
            let kind = match source {
                FieldSource::SectionPrefix { kind } => kind,
                FieldSource::SameFile { of, .. }
                | FieldSource::Inside { of }
                | FieldSource::Remainder { exclude: Some(of) }
                | FieldSource::Enclosing { of, .. } => match of {
                    MatchSource::Kind { kind } => kind,
                    MatchSource::Rule { .. } => continue,
                },
                _ => continue,
            };
            if let Some(reason) = self
                .unresolved_ranges(kind, file)
                .into_iter()
                .filter(|(unknown, _)| match source {
                    FieldSource::Inside { .. } | FieldSource::Remainder { .. } => {
                        match_within(unknown, range)
                    }
                    FieldSource::Enclosing { .. } | FieldSource::SectionPrefix { .. } => {
                        match_within(range, unknown)
                    }
                    _ => true,
                })
                .map(|(_, reason)| reason)
                .max()
            {
                fields.insert(
                    name.clone(),
                    FieldValue::Unavailable {
                        reason: reason.field_unavailable(),
                    },
                );
                if matches!(source, FieldSource::Inside { .. }) {
                    deferred_inside = deferred_inside.max(Some(reason));
                }
            }
        }
        // Remainder subtracts every Inside field; an unknown subtraction is not empty text.
        if let Some(reason) = deferred_inside {
            for (name, source) in &definition.fields {
                if matches!(source, FieldSource::Remainder { .. }) {
                    let reason = fields
                        .get(name)
                        .and_then(FieldValue::selection_deferral)
                        .map_or(reason, |existing| existing.max(reason));
                    fields.insert(
                        name.clone(),
                        FieldValue::Unavailable {
                            reason: reason.field_unavailable(),
                        },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "deferred_fields_test.rs"]
mod tests;

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
    fn unresolved_ranges(&self, kind: &EvidenceKindName, file: &MatchedFile) -> Vec<MatchRange> {
        let mut ranges = self
            .unresolved_selections
            .iter()
            .filter(|candidate| candidate.kind == *kind && candidate.span.source == file.source)
            .map(|candidate| MatchRange {
                start: candidate.span.byte_start,
                end: candidate.span.byte_end,
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
                if !exists && unknown.iter().any(|parent| match_within(range, parent)) {
                    ranges.push(range.clone());
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
        let mut pending_inside = false;
        for (name, source) in &definition.fields {
            let of = match source {
                FieldSource::SameFile { of, .. }
                | FieldSource::Inside { of }
                | FieldSource::Remainder { exclude: Some(of) }
                | FieldSource::Enclosing { of, .. } => of,
                _ => continue,
            };
            let MatchSource::Kind { kind } = of else {
                continue;
            };
            if self
                .unresolved_ranges(kind, file)
                .iter()
                .any(|unknown| match source {
                    FieldSource::Inside { .. } | FieldSource::Remainder { .. } => {
                        match_within(unknown, range)
                    }
                    FieldSource::Enclosing { .. } => match_within(range, unknown),
                    _ => true,
                })
            {
                fields.insert(
                    name.clone(),
                    FieldValue::Unavailable {
                        reason: FieldUnavailable::SelectionPending,
                    },
                );
                pending_inside |= matches!(source, FieldSource::Inside { .. });
            }
        }
        // Remainder subtracts every Inside field; an unknown subtraction is not empty text.
        if pending_inside {
            for (name, source) in &definition.fields {
                if matches!(source, FieldSource::Remainder { .. }) {
                    fields.insert(
                        name.clone(),
                        FieldValue::Unavailable {
                            reason: FieldUnavailable::SelectionPending,
                        },
                    );
                }
            }
        }
    }
}

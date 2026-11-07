use super::*;

impl EvidenceBuild {
    pub(super) fn project_fields(
        &self,
        definition: &EvidenceKindDefinition,
        file: &MatchedFile,
        range: &MatchRange,
        fields: &BTreeMap<FieldName, FieldValue>,
    ) -> crate::ModelView {
        let limit = definition
            .max_chars
            .map_or(self.limits.evidence_max_chars, |value| value.get() as usize);
        if fields.values().any(FieldValue::is_selection_pending) {
            let upper_bound = fields
                .iter()
                .map(|(name, value)| {
                    if !value.is_selection_pending() {
                        let known = BTreeMap::from([(name.clone(), value.clone())]);
                        return match project_model_view(&known, usize::MAX) {
                            crate::ModelView::Bounded { fields, .. } => {
                                fields.values().map(crate::model_value_chars).sum()
                            }
                            _ => usize::MAX,
                        };
                    }
                    let Some(source) = definition.fields.get(name) else {
                        return usize::MAX;
                    };
                    let (of, full) = match source {
                        FieldSource::Inside { of } => (of, true),
                        FieldSource::SameFile { of, text }
                        | FieldSource::Enclosing { of, text } => {
                            (of, *text == crate::ContextText::Full)
                        }
                        FieldSource::Remainder { .. } => {
                            return file.text[range.start..range.end].chars().count();
                        }
                        _ => return usize::MAX,
                    };
                    let mut possible = source_ranges(of, file, &self.catalog.pieces);
                    if let MatchSource::Kind { kind } = of {
                        possible.extend(self.unresolved_ranges(kind, file));
                    }
                    possible.sort_by_key(|span| (span.start, span.end));
                    possible.dedup_by_key(|span| (span.start, span.end));
                    possible
                        .iter()
                        .filter(|span| match source {
                            FieldSource::Inside { .. } => match_within(span, range),
                            FieldSource::Enclosing { .. } => match_within(range, span),
                            _ => true,
                        })
                        .map(|span| {
                            let text = &file.text[span.start..span.end];
                            if full {
                                text.chars().count()
                            } else {
                                text.lines().next().unwrap_or("").chars().count()
                            }
                        })
                        .sum()
                })
                .fold(0usize, usize::saturating_add);
            // Unknown siblings can change cuts (including cut-fields metadata) in the shared view.
            if upper_bound > limit {
                return crate::ModelView::SelectionPending;
            }
        }
        project_model_view(fields, limit)
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
            for range in file
                .index
                .by_rule
                .get(&definition.matcher.rule_id(kind))
                .into_iter()
                .flatten()
            {
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

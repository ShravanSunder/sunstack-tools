use super::*;

impl EvidenceBuild {
    pub(super) fn document_field(
        &mut self,
        owner: &Path,
        relative: &Path,
        rule: &crate::EvidenceRuleId,
        projection: crate::DocumentText,
        source: &mut dyn CheckedFileSource,
    ) -> FieldValue {
        let path = owner.parent().unwrap_or(Path::new("")).join(relative);
        let read = || FieldValue::Unavailable {
            reason: FieldUnavailable::Unreadable,
        };
        let resolved = match source.resolve_file(&path) {
            Ok(resolved) => resolved,
            Err(CheckedReadError::NotContained) => {
                return FieldValue::Unavailable {
                    reason: FieldUnavailable::OutsideRoot,
                };
            }
            Err(_) => return read(),
        };
        let Ok(text) = read_cached_content(
            source,
            &mut self.contents,
            &mut self.total_bytes,
            &resolved,
            self.limits,
        ) else {
            return read();
        };
        let Some(language) = self.project.language_for_rule(rule) else {
            return read();
        };
        let index = self.project.match_file_as_language(
            &resolved,
            &text,
            self.limits.max_matches,
            language,
        );
        if index.failed_languages.contains(&language) || index.limited {
            return read();
        }
        let Ok(repository) = RepositoryId::new("checked_repository") else {
            return read();
        };
        let file = MatchedFile {
            source: SourceRef::Document {
                repository,
                path: resolved,
                content_identity: content_digest(text.as_bytes()),
            },
            text,
            index,
            native_units: BTreeMap::new(),
        };
        let ranges = file.index.by_rule.get(rule).cloned().unwrap_or_default();
        let values = ranges
            .iter()
            .map(|range| {
                let raw = &file.text[range.start..range.end];
                projection.project(raw)
            })
            .collect::<Result<Vec<_>, _>>();
        let values = match values {
            Ok(values) => values,
            Err(reason) => return FieldValue::Unavailable { reason },
        };
        FieldValue::Texts {
            values,
            spans: ranges
                .iter()
                .map(|range| source_span(&file, range))
                .collect(),
        }
    }
}

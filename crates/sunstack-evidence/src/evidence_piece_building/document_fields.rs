use super::*;

impl EvidenceBuild {
    pub(super) fn document_field(
        &mut self,
        owner: &Path,
        relative: &crate::DocumentCandidates,
        rule: &crate::EvidenceRuleId,
        source: &mut dyn CheckedFileSource,
    ) -> FieldValue {
        let read = || FieldValue::Unavailable {
            reason: FieldUnavailable::Unreadable,
        };
        let mut candidates = Vec::new();
        let mut selected = None;
        for candidate in relative.paths() {
            let path = owner.parent().unwrap_or(Path::new("")).join(candidate);
            let resolved = match source.resolve_file(&path) {
                Ok(resolved) => resolved,
                Err(CheckedReadError::NotContained) => {
                    return FieldValue::Unavailable {
                        reason: FieldUnavailable::OutsideRoot,
                    };
                }
                Err(CheckedReadError::Missing { path }) => {
                    candidates.push(path);
                    continue;
                }
                Err(_) => return read(),
            };
            selected = Some(resolved);
            break;
        }
        let Some(resolved) = selected else {
            return FieldValue::MissingDocument { candidates };
        };
        let chosen_path = resolved.clone();
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
            .map(|range| file.text[range.start..range.end].to_owned())
            .collect();
        let spans = ranges
            .iter()
            .map(|range| source_span(&file, range))
            .collect();
        if relative.records_resolution() {
            FieldValue::Document {
                path: chosen_path,
                values,
                spans,
            }
        } else {
            FieldValue::Texts { values, spans }
        }
    }
}

#[cfg(test)]
#[path = "document_fields_test.rs"]
mod tests;

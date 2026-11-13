use super::*;

impl EvidenceBuild {
    pub(super) fn finalize(
        &mut self,
        kind: &EvidenceKindName,
        mut pieces: Vec<EvidencePiece>,
        source: &mut dyn CheckedFileSource,
    ) {
        let definition = self.kinds[kind].clone();
        for piece in &mut pieces {
            for (field, field_source) in &definition.fields {
                if let FieldSource::Document { path, rule, text } = field_source {
                    let value =
                        self.document_field(&piece.location.file, path, rule, *text, source);
                    piece.fields.insert(field.clone(), value);
                    continue;
                }
                let FieldSource::Target {
                    regex,
                    resolve,
                    opening_chars,
                    as_kind,
                    exclude,
                } = field_source
                else {
                    continue;
                };
                let names = Regex::new(regex)
                    .ok()
                    .into_iter()
                    .flat_map(|pattern| {
                        self.matched
                            .get(&piece.location.file)
                            .map(|file| {
                                target_names(
                                    &pattern,
                                    file,
                                    &MatchRange {
                                        start: piece.location.span.byte_start,
                                        end: piece.location.span.byte_end,
                                    },
                                    exclude.as_ref(),
                                )
                            })
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>();
                if names.iter().any(String::is_empty) {
                    piece.fields.insert(
                        field.clone(),
                        FieldValue::Unavailable {
                            reason: FieldUnavailable::EmptyTarget,
                        },
                    );
                    continue;
                }
                let values = names
                    .into_iter()
                    .map(|name| {
                        let use_path = TargetReference::uses_path(&name, resolve);
                        let candidates = if use_path {
                            vec![]
                        } else {
                            self.matched
                                .keys()
                                .filter(|candidate| {
                                    candidate.file_name().is_some_and(|name| name == "SKILL.md")
                                        && candidate
                                            .parent()
                                            .and_then(Path::file_name)
                                            .is_some_and(|part| part == name.as_str())
                                })
                                .cloned()
                                .collect::<Vec<_>>()
                        };
                        let reference = if use_path {
                            let syntax = TargetReferenceSyntax::for_language(
                                evidence_kind_language(&self.project, kind, &definition),
                            );
                            TargetReference::parse(&name, syntax)?
                        } else {
                            TargetReference {
                                file: PathBuf::from(&name),
                                fragment: None,
                            }
                        };
                        let file_name = &reference.file;
                        let fragment = reference.fragment;
                        let path = if use_path {
                            if file_name.as_os_str().is_empty() {
                                piece.location.file.clone()
                            } else {
                                piece
                                    .location
                                    .file
                                    .parent()
                                    .unwrap_or(Path::new(""))
                                    .join(file_name)
                            }
                        } else {
                            candidates
                                .first()
                                .cloned()
                                .unwrap_or_else(|| PathBuf::from(&name).join("SKILL.md"))
                        };
                        let mut target = LoadTarget {
                            name,
                            path: path.clone(),
                            exists: false,
                            anchor: None,
                            candidates: vec![],
                            opening: None,
                            fields: None,
                            opening_cut: false,
                            opening_last_line: None,
                        };
                        if !use_path && candidates.len() != 1 {
                            target.candidates = candidates;
                            return Ok(target);
                        }
                        let resolved = match source.resolve_file(&path) {
                            Ok(resolved) => resolved,
                            Err(CheckedReadError::Missing { .. }) => return Ok(target),
                            Err(CheckedReadError::NotContained) => {
                                return Err(FieldUnavailable::OutsideRoot);
                            }
                            Err(_) => return Err(FieldUnavailable::Unreadable),
                        };
                        target.path = resolved.clone();
                        let content = if let Some(file) = self.matched.get(&resolved) {
                            Ok(file.text.clone())
                        } else {
                            read_cached_content(
                                source,
                                &mut self.contents,
                                &mut self.total_bytes,
                                &resolved,
                                self.limits,
                            )
                        };
                        let text = content.map_err(|_| FieldUnavailable::Unreadable)?;
                        target.exists = true;
                        target.anchor = fragment.map(|fragment| {
                            use crate::TargetAnchor;
                            if is_html_path(&resolved) {
                                if crate::document_evidence::html_content::html_has_id(
                                    &text, &fragment,
                                ) {
                                    TargetAnchor::Found { fragment }
                                } else {
                                    TargetAnchor::Missing { fragment }
                                }
                            } else if crate::document_evidence::markdown_document::is_markdown_path(
                                &resolved,
                            ) {
                                crate::MarkdownDocument::parse(&text).resolve_anchor(
                                    fragment,
                                    self.project.document_settings.markdown.heading_anchors,
                                )
                            } else {
                                TargetAnchor::Unsupported {
                                    fragment,
                                    reason:
                                        crate::TargetAnchorUnsupportedReason::UnsupportedDocument,
                                }
                            }
                        });
                        let opening: String = text.chars().take(*opening_chars).collect();
                        target.opening_cut = text.chars().count() > *opening_chars;
                        target.opening_last_line =
                            (!opening.is_empty()).then(|| opening.lines().count());
                        target.opening = Some(opening);
                        if use_path {
                            if let Some(as_kind) = as_kind {
                                target.fields =
                                    self.path_target_fields(&resolved, &text, as_kind)?;
                            }
                        } else {
                            target.exists = true;
                            if let Some(as_kind) = as_kind {
                                target.fields = self
                                    .catalog
                                    .pieces
                                    .iter()
                                    .find(|candidate| {
                                        candidate.kind == *as_kind
                                            && candidate.location.file == resolved
                                    })
                                    .map(|candidate| candidate.fields.clone());
                            }
                        }
                        Ok(target)
                    })
                    .collect::<Result<Vec<_>, FieldUnavailable>>();
                piece.fields.insert(
                    field.clone(),
                    match values {
                        Ok(values) => FieldValue::Targets { values },
                        Err(reason) => FieldValue::Unavailable { reason },
                    },
                );
            }
            piece.model_view = self.project_fields(kind, &definition, &piece.fields);
        }
        for count in &mut self.catalog.file_counts {
            if let Some(kind_count) = count.kinds.get_mut(kind) {
                kind_count.pieces = self
                    .catalog
                    .pieces
                    .iter()
                    .chain(&pieces)
                    .filter(|piece| piece.kind == *kind && piece.location.file == count.file)
                    .count();
            }
        }
        self.catalog.pieces.extend(pieces);
    }
}

impl EvidenceBuild {
    fn path_target_fields(
        &self,
        path: &Path,
        text: &str,
        as_kind: &EvidenceKindName,
    ) -> Result<Option<BTreeMap<FieldName, FieldValue>>, FieldUnavailable> {
        if let Some(piece) = self
            .catalog
            .pieces
            .iter()
            .find(|piece| piece.kind == *as_kind && piece.location.file == path)
        {
            return Ok(Some(piece.fields.clone()));
        }
        let source = SourceRef::Document {
            repository: RepositoryId::new("checked_repository")
                .map_err(|_| FieldUnavailable::Unreadable)?,
            path: path.into(),
            content_identity: content_digest(text.as_bytes()),
        };
        let definition = self
            .kinds
            .get(as_kind)
            .ok_or(FieldUnavailable::Unreadable)?;
        let language = evidence_kind_language(&self.project, as_kind, definition)
            .ok_or(FieldUnavailable::Unreadable)?;
        let index =
            self.project
                .match_file_as_language(path, text, self.limits.max_matches, language);
        let mut remaining_matches = self
            .limits
            .max_matches
            .saturating_sub(index.by_rule.values().map(Vec::len).sum());
        let native_units = collect_native_units(
            path,
            text,
            &self.kinds,
            &source,
            &mut remaining_matches,
            &self.project.document_settings.html,
        )
        .map_err(|_| FieldUnavailable::Unreadable)?;
        let file = MatchedFile {
            text: text.into(),
            index,
            source,
            native_units,
        };
        if file.native_unavailable(as_kind) {
            return Err(FieldUnavailable::Unreadable);
        }
        let mut pieces = Vec::new();
        for kind in &self.plan.order {
            let definition = &self.kinds[kind];
            for (position, range) in file.kind_ranges(kind, definition).iter().enumerate() {
                if let Some(parents) = &definition.within
                    && !pieces.iter().any(|piece: &EvidencePiece| {
                        parents.as_slice().contains(&piece.kind)
                            && match_within(
                                range,
                                &MatchRange {
                                    start: piece.location.span.byte_start,
                                    end: piece.location.span.byte_end,
                                },
                            )
                    })
                {
                    continue;
                }
                let fields = local_fields(
                    kind,
                    definition,
                    &file,
                    range,
                    position,
                    &pieces,
                    &self.project.document_settings.html,
                );
                if kind == as_kind {
                    return Ok(Some(fields));
                }
                let span = source_span(&file, range);
                pieces.push(EvidencePiece {
                    kind: kind.clone(),
                    location: PieceLocation {
                        file: path.into(),
                        first_line: span.start_line,
                        last_line: span.end_line,
                        span,
                    },
                    text: file.text[range.start..range.end].into(),
                    model_view: self.project_fields(kind, definition, &fields),
                    fields,
                });
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
#[path = "target_loading_test.rs"]
mod tests;

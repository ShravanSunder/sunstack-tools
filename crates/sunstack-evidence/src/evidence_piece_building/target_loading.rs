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
                let values = names
                    .into_iter()
                    .map(|name| {
                        let use_path = match resolve {
                            EvidenceTargetResolution::Path => true,
                            EvidenceTargetResolution::Skill => false,
                            EvidenceTargetResolution::PathOrSkill => name.ends_with(".md"),
                        };
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
                        let path = if use_path {
                            piece
                                .location
                                .file
                                .parent()
                                .unwrap_or(Path::new(""))
                                .join(&name)
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
                        let Some(resolved) = source.resolve_file(&path) else {
                            return Ok(target);
                        };
                        target.path = resolved.clone();
                        if use_path {
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
                            let opening: String = text.chars().take(*opening_chars).collect();
                            target.opening_cut = text.chars().count() > *opening_chars;
                            target.opening_last_line =
                                (!opening.is_empty()).then(|| opening.lines().count());
                            target.opening = Some(opening);
                            if let Some(as_kind) = as_kind {
                                target.fields = self.path_target_fields(&resolved, &text, as_kind);
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
            piece.model_view = project_model_view(
                &piece.fields,
                definition
                    .max_chars
                    .map_or(self.limits.evidence_max_chars, |n| n.get() as usize),
            );
        }
        for count in &mut self.catalog.file_counts {
            if let Some(kind_count) = count.kinds.get_mut(kind) {
                kind_count.pieces = pieces
                    .iter()
                    .filter(|piece| piece.location.file == count.file)
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
    ) -> Option<BTreeMap<FieldName, FieldValue>> {
        if let Some(piece) = self
            .catalog
            .pieces
            .iter()
            .find(|piece| piece.kind == *as_kind && piece.location.file == path)
        {
            return Some(piece.fields.clone());
        }
        let source = SourceRef::Document {
            repository: RepositoryId::new("checked_repository").ok()?,
            path: path.into(),
            content_identity: content_digest(text.as_bytes()),
        };
        let language = self
            .project
            .language_for_rule(&self.kinds.get(as_kind)?.rule)?;
        let file = MatchedFile {
            text: text.into(),
            index: self.project.match_file_as_language(
                path,
                text,
                self.limits.max_matches,
                language,
            ),
            source,
        };
        let mut pieces = Vec::new();
        for kind in &self.plan.order {
            let definition = &self.kinds[kind];
            for (position, range) in file
                .index
                .by_rule
                .get(&definition.rule)
                .into_iter()
                .flatten()
                .enumerate()
            {
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
                let fields = local_fields(definition, &file, range, position, &pieces);
                if kind == as_kind {
                    return Some(fields);
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
                    model_view: project_model_view(&fields, self.limits.evidence_max_chars),
                    fields,
                });
            }
        }
        None
    }
}

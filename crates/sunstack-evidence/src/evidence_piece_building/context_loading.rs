use super::*;
use std::collections::BTreeSet;

impl EvidenceBuild {
    pub(super) fn scan_files(&mut self, files: &[PathBuf], source: &mut dyn CheckedFileSource) {
        let mut paths = files.to_vec();
        paths.sort();
        paths.dedup();
        let limits = self.limits;
        let mut remaining = limits.max_matches.saturating_sub(
            self.catalog
                .matches
                .values()
                .flat_map(|index| index.by_rule.values())
                .map(Vec::len)
                .sum::<usize>(),
        );
        for path in paths {
            if self.catalog.matches.contains_key(&path) {
                continue;
            }
            match read_cached_content(
                source,
                &mut self.contents,
                &mut self.total_bytes,
                &path,
                limits,
            ) {
                Err(error) => self.catalog.failures.push(EvidenceFileFailure {
                    file: path,
                    reason: error.to_string(),
                }),
                Ok(text) => {
                    let index = self.project.match_file(&path, &text, remaining);
                    for language in &index.failed_languages {
                        self.catalog.failures.push(EvidenceFileFailure {
                            file: path.clone(),
                            reason: format!("invalid {language:?} evidence"),
                        });
                    }
                    remaining = remaining
                        .saturating_sub(index.by_rule.values().map(Vec::len).sum::<usize>());
                    if index.limited {
                        self.catalog.failures.push(EvidenceFileFailure {
                            file: path.clone(),
                            reason: "max_matches".into(),
                        });
                    }
                    let Ok(repository) = RepositoryId::new("checked_repository") else {
                        continue;
                    };
                    let source_ref = SourceRef::Document {
                        repository,
                        path: path.clone(),
                        content_identity: content_digest(text.as_bytes()),
                    };
                    let counts = self
                        .kinds
                        .iter()
                        .map(|(name, kind)| {
                            (
                                name.clone(),
                                KindCount {
                                    matches: index
                                        .by_rule
                                        .get(&kind.matcher.rule_id(name))
                                        .map_or(0, Vec::len),
                                    limited: index.limited,
                                    selection: kind
                                        .select
                                        .as_ref()
                                        .map(|_| SelectionCount::default()),
                                    ..KindCount::default()
                                },
                            )
                        })
                        .collect();
                    self.catalog.file_counts.push(FileCount {
                        file: path.clone(),
                        kinds: counts,
                    });
                    self.catalog.matches.insert(path.clone(), index.clone());
                    self.matched.insert(
                        path,
                        MatchedFile {
                            text,
                            index,
                            source: source_ref,
                        },
                    );
                }
            }
        }
    }

    pub(super) fn pieces_for_kind(
        &self,
        kind: &EvidenceKindName,
        files: &BTreeSet<PathBuf>,
    ) -> Vec<EvidencePiece> {
        let definition = &self.kinds[kind];
        let mut pieces = Vec::new();
        for (path, file) in self
            .matched
            .iter()
            .filter(|(path, _)| files.contains(*path))
        {
            for (match_position, range) in file
                .index
                .by_rule
                .get(&definition.matcher.rule_id(kind))
                .into_iter()
                .flatten()
                .enumerate()
            {
                if let Some(parents) = &definition.within
                    && !self.catalog.pieces.iter().any(|piece| {
                        parents.as_slice().contains(&piece.kind)
                            && piece.location.file == *path
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
                    &definition.matcher.rule_id(kind),
                    definition,
                    file,
                    range,
                    match_position,
                    &self.catalog.pieces,
                );
                let span = source_span(file, range);
                let location = PieceLocation {
                    file: path.clone(),
                    first_line: span.start_line,
                    last_line: file.text.as_bytes()[..range.end.saturating_sub(1).max(range.start)]
                        .iter()
                        .filter(|b| **b == b'\n')
                        .count()
                        + 1,
                    span,
                };
                pieces.push(EvidencePiece {
                    kind: kind.clone(),
                    location,
                    text: file.text[range.start..range.end].into(),
                    model_view: project_model_view(
                        &fields,
                        definition
                            .max_chars
                            .map_or(self.limits.evidence_max_chars, |n| n.get() as usize),
                    ),
                    fields,
                });
            }
        }
        pieces
    }

    pub(super) fn finished_catalog(&self) -> EvidenceCatalog {
        let mut catalog = self.catalog.clone();
        catalog
            .file_counts
            .sort_by(|left, right| left.file.cmp(&right.file));
        catalog.pieces.sort_by(|left, right| {
            (
                &left.kind,
                &left.location.file,
                left.location.span.byte_start,
            )
                .cmp(&(
                    &right.kind,
                    &right.location.file,
                    right.location.span.byte_start,
                ))
        });
        for (file, content) in &self.contents {
            if let Err(
                error
                @ (CheckedReadError::TotalBytesExceeded | CheckedReadError::FileCountExceeded),
            ) = content
                && !catalog.failures.iter().any(|failure| failure.file == *file)
            {
                catalog.failures.push(EvidenceFileFailure {
                    file: file.clone(),
                    reason: error.to_string(),
                });
            }
        }
        catalog
    }

    /// Add demanded context after checked evidence is settled, retaining its acquisition budgets.
    pub fn add_context_files(
        &mut self,
        files: &[PathBuf],
        source: &mut dyn CheckedFileSource,
    ) -> EvidenceCatalog {
        let additional = files
            .iter()
            .filter(|file| {
                !self.catalog.matches.contains_key(*file)
                    && !self
                        .catalog
                        .failures
                        .iter()
                        .any(|failure| &failure.file == *file)
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        self.scan_files(&additional.iter().cloned().collect::<Vec<_>>(), source);
        for kind in self.plan.order.clone() {
            // Context is not selected rule evidence; reads kinds themselves cannot use select.
            if self.kinds[&kind].select.is_none() {
                let pieces = self.pieces_for_kind(&kind, &additional);
                self.finalize(&kind, pieces, source);
            }
        }
        self.finished_catalog()
    }
}

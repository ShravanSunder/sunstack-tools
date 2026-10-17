use super::*;

impl Preparation {
    pub(crate) async fn native_select(
        &mut self,
        query: &NativeQuery,
    ) -> EvidenceResult<SelectionReadiness> {
        let (repository, paths, selection) = match &query.selector {
            EvidenceSelector::RelatedUnits {
                query: relationship_query,
                chunk_settings,
                minimum_related_units,
            } => {
                return self
                    .related_select(
                        relationship_query,
                        chunk_settings,
                        *minimum_related_units,
                        &EvidenceDeclaration {
                            id: query.id.clone(),
                            input: SourceName::new("native_query")?,
                            requirement: EvidenceRequirement::Required,
                            select: EvidenceSelection::WholeDocument {},
                            max_items: None,
                            max_bytes: None,
                        },
                    )
                    .await;
            }
            EvidenceSelector::Paragraph {
                repository_id,
                source_path,
                target_id,
            } => (
                repository_id,
                vec![QueryPath::Exact {
                    path: source_path.clone(),
                }],
                EvidenceSelection::Paragraph {
                    target_id: target_id.clone(),
                },
            ),
            EvidenceSelector::FileText {
                repository_id,
                source_path,
            } => (
                repository_id,
                vec![QueryPath::Exact {
                    path: source_path.clone(),
                }],
                EvidenceSelection::WholeDocument {},
            ),
            EvidenceSelector::LiteralText {
                repository_id,
                paths,
                text,
            } => (
                repository_id,
                paths.clone(),
                EvidenceSelection::LiteralText { text: text.clone() },
            ),
            EvidenceSelector::RegexText {
                repository_id,
                paths,
                pattern,
            } => (
                repository_id,
                paths.clone(),
                EvidenceSelection::RegexText {
                    pattern: pattern.clone(),
                },
            ),
            EvidenceSelector::AstPattern {
                repository_id,
                paths,
                language,
                pattern,
            } => (
                repository_id,
                paths.clone(),
                EvidenceSelection::AstPattern {
                    language: language.clone(),
                    pattern: pattern.clone(),
                },
            ),
        };
        let binding = self
            .input
            .repositories
            .iter()
            .find(|binding| &binding.key == repository)
            .ok_or(EvidenceCode::InvalidSelector)?
            .clone();
        let include_ignored = self.input.limits.include_ignored;
        let max_files = self.input.limits.max_files.get();
        let paths = tokio::task::spawn_blocking(move || {
            resolve_query_paths(&binding, &paths, include_ignored, max_files)
        })
        .await
        .map_err(|_| EvidenceCode::SourceUnavailable)??;
        let mut all_matches = Vec::new();
        let mut last = None;
        for path in paths {
            let document = self.document(repository, path).await?;
            let coverage = CaptureCoverage {
                source: document.source.clone(),
                cursor_basis: CursorBasis::NoCursorSupplied,
                tail: CaptureTail::Complete,
                gaps: vec![],
                bytes_read: document.text.len() as u64,
                bytes_parsed: document.text.len() as u64,
            };
            let doc = document.clone();
            let query_selection = selection.clone();
            let max_matches = self.input.limits.max_matches.get();
            let content = tokio::task::spawn_blocking(move || {
                select_document(&doc, &query_selection, max_matches)
            })
            .await
            .map_err(|_| EvidenceCode::InvalidSelector)??;
            match content {
                SelectionContent::Matches { matches } => all_matches.extend(matches),
                other => last = Some((other, coverage.clone())),
            };
            self.measurements.files_parsed += 1;
            self.measurements.bytes_scanned += document.text.len();
            if last.is_none() {
                last = Some((SelectionContent::Matches { matches: vec![] }, coverage));
            }
        }
        let (content, coverage) = last.ok_or(EvidenceCode::MissingContext)?;
        let content = if matches!(content, SelectionContent::Matches { .. }) {
            if all_matches.len() > self.input.limits.max_matches.get() {
                return Err(EvidenceCode::SelectorLimit.into());
            }
            SelectionContent::Matches {
                matches: all_matches,
            }
        } else {
            content
        };
        self.admit_selection(
            content,
            vec![],
            coverage,
            &EvidenceDeclaration {
                id: query.id.clone(),
                input: SourceName::new("native_query")?,
                requirement: EvidenceRequirement::Required,
                select: selection,
                max_items: None,
                max_bytes: None,
            },
        )
    }
}
fn resolve_query_paths(
    binding: &RepositoryBinding,
    queries: &[QueryPath],
    include_ignored: bool,
    max_files: usize,
) -> EvidenceResult<Vec<PathBuf>> {
    let mut paths = BTreeSet::new();
    for query in queries {
        match query {
            QueryPath::Exact { path } => {
                if !path_is_allowed(binding, path)? {
                    return Err(EvidenceCode::PathOutsideRoot.into());
                }
                paths.insert(path.clone());
            }
            QueryPath::Glob { pattern } => {
                let matcher = globset::GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .build()
                    .map_err(|_| EvidenceCode::InvalidSelector)?
                    .compile_matcher();
                for entry in ignore::WalkBuilder::new(&binding.root)
                    .hidden(false)
                    .git_ignore(!include_ignored)
                    .git_global(!include_ignored)
                    .git_exclude(!include_ignored)
                    .parents(false)
                    .build()
                {
                    let entry = entry.map_err(|_| EvidenceCode::SourceUnavailable)?;
                    if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                        continue;
                    }
                    let path = entry
                        .path()
                        .strip_prefix(&binding.root)
                        .map_err(|_| EvidenceCode::PathOutsideRoot)?;
                    if matcher.is_match(path) && path_is_allowed(binding, path)? {
                        paths.insert(path.into());
                    }
                    if paths.len() > max_files {
                        return Err(EvidenceCode::EvidenceLimit.into());
                    }
                }
            }
        }
    }
    Ok(paths.into_iter().collect())
}

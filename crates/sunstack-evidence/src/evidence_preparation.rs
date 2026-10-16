mod native_queries;
use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub enum CapturedSource {
    Document(Arc<CapturedDocument>),
    Transcript(Arc<ConversationCapture>),
    Payload(Arc<AdmittedPayload>),
}
#[derive(Default)]
pub struct SourceSnapshot {
    pub sources: BTreeMap<SourceName, CapturedSource>,
}
struct Preparation {
    input: EvidencePlanInput,
    registry: AdapterRegistry,
    cancellation: CancellationToken,
    snapshot: SourceSnapshot,
    selections: Vec<SelectionResult>,
    captures: Vec<CaptureCoverage>,
    cursors: Vec<ReturnedCursor>,
    measurements: EvidenceMeasurements,
    diagnostics: DiagnosticList,
    total_read: usize,
    document_cache: BTreeMap<(RepositoryId, PathBuf), Arc<CapturedDocument>>,
}
/// One invocation freezes shared captures. Provider configuration is neither accepted nor needed.
pub async fn prepare_evidence(
    input: EvidencePlanInput,
    registry: AdapterRegistry,
    cancellation: CancellationToken,
) -> EvidencePlanOutcome {
    let started = Instant::now();
    if cancellation.is_cancelled() {
        return EvidencePlanOutcome::Cancelled {
            progress: EvidenceCancellationProgress::BeforePlanAdmission,
            diagnostics: DiagnosticList::default(),
            measurements: EvidenceMeasurements::default(),
        };
    }
    if let Err(error) = crate::evidence_admission::admission(&input, &registry) {
        return EvidencePlanOutcome::Refused {
            diagnostics: error.diagnostics(),
        };
    }
    let mut preparation = Preparation {
        input,
        registry,
        cancellation,
        snapshot: SourceSnapshot::default(),
        selections: Vec::new(),
        captures: Vec::new(),
        cursors: Vec::new(),
        measurements: EvidenceMeasurements::default(),
        diagnostics: DiagnosticList::default(),
        total_read: 0,
        document_cache: BTreeMap::new(),
    };
    if let Err(error) = preparation.admit_payloads().await {
        return EvidencePlanOutcome::Refused {
            diagnostics: error.diagnostics(),
        };
    }
    for declaration in preparation.input.declarations.clone() {
        if preparation.cancellation.is_cancelled() {
            break;
        }
        let result = preparation.select(&declaration).await;
        match result {
            Ok(readiness) => preparation.selections.push(SelectionResult {
                id: declaration.id,
                readiness,
            }),
            Err(error) => {
                if declaration.requirement == EvidenceRequirement::Required {
                    preparation.diagnostics.push(error.diagnostic());
                }
                preparation.selections.push(SelectionResult {
                    id: declaration.id,
                    readiness: SelectionReadiness::Refused {
                        diagnostics: error.diagnostics(),
                    },
                });
            }
        }
    }
    if !preparation.input.native_queries.is_empty() {
        // Native file queries use the same captured-source and selector implementations.
        for query in preparation.input.native_queries.clone() {
            if preparation.cancellation.is_cancelled() {
                break;
            }
            match preparation.native_select(&query).await {
                Ok(readiness) => preparation.selections.push(SelectionResult {
                    id: query.id,
                    readiness,
                }),
                Err(error) => {
                    preparation.diagnostics.push(error.diagnostic());
                    preparation.selections.push(SelectionResult {
                        id: query.id,
                        readiness: SelectionReadiness::Refused {
                            diagnostics: error.diagnostics(),
                        },
                    });
                }
            }
        }
    }
    preparation.measurements.local_elapsed_ms =
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if preparation.cancellation.is_cancelled() {
        return EvidencePlanOutcome::Cancelled {
            progress: EvidenceCancellationProgress::PlanAdmitted {
                partial: PartialEvidencePlan {
                    selections: preparation.selections,
                    captures: preparation.captures,
                    cursors: preparation.cursors,
                },
            },
            diagnostics: preparation.diagnostics,
            measurements: preparation.measurements,
        };
    }
    if preparation.diagnostics.items.is_empty() {
        EvidencePlanOutcome::Prepared {
            selections: preparation.selections,
            captures: preparation.captures,
            cursors: preparation.cursors,
            measurements: preparation.measurements,
        }
    } else {
        EvidencePlanOutcome::Incomplete {
            selections: preparation.selections,
            captures: preparation.captures,
            cursors: preparation.cursors,
            measurements: preparation.measurements,
            diagnostics: preparation.diagnostics,
        }
    }
}
impl Preparation {
    async fn admit_payloads(&mut self) -> EvidenceResult<()> {
        let paths = self.input.payload_schemas.clone();
        let catalog = tokio::task::spawn_blocking(move || {
            let mut schemas = BTreeMap::new();
            for entry in paths {
                let bytes =
                    std::fs::read(entry.path).map_err(|_| EvidenceCode::PayloadSchemaInvalid)?;
                let schema = serde_json::from_slice(&bytes)
                    .map_err(|_| EvidenceCode::PayloadSchemaInvalid)?;
                if schemas.insert(entry.name, schema).is_some() {
                    return Err(EvidenceCode::PayloadSchemaInvalid.into());
                }
            }
            PayloadSchemaCatalog::compile(schemas)
        })
        .await
        .map_err(|_| EvidenceCode::PayloadSchemaInvalid)??;
        for (name, spec) in &self.input.sources {
            if let SourceSpec::Payload { schema, value } = spec {
                let admitted = catalog.validate(schema, value.clone())?;
                self.snapshot
                    .sources
                    .insert(name.clone(), CapturedSource::Payload(Arc::new(admitted)));
            }
        }
        Ok(())
    }
    async fn source(&mut self, name: &SourceName) -> EvidenceResult<CapturedSource> {
        if let Some(source) = self.snapshot.sources.get(name) {
            return Ok(source.clone());
        }
        let spec = self
            .input
            .sources
            .get(name)
            .ok_or(EvidenceCode::InvalidSelector)?
            .clone();
        // Aliases reuse captures rather than rereading the same bound source.
        for (alias, source) in &self.snapshot.sources {
            let same =
                self.input
                    .sources
                    .get(alias)
                    .is_some_and(|existing| match (existing, &spec) {
                        (
                            SourceSpec::Conversation {
                                transcript_path: first,
                                format: first_format,
                                final_assistant_text: first_final,
                            },
                            SourceSpec::Conversation {
                                transcript_path: second,
                                format: second_format,
                                final_assistant_text: second_final,
                            },
                        ) => {
                            first_format == second_format
                                && first_final == second_final
                                && first
                                    .canonicalize()
                                    .ok()
                                    .zip(second.canonicalize().ok())
                                    .is_some_and(|(first, second)| first == second)
                        }
                        _ => existing == &spec,
                    });
            if same {
                let source = source.clone();
                self.snapshot.sources.insert(name.clone(), source.clone());
                return Ok(source);
            }
        }
        let captured = match spec {
            SourceSpec::Document { repository, path } => {
                CapturedSource::Document(self.document(&repository, path).await?)
            }
            SourceSpec::Conversation {
                transcript_path,
                format,
                final_assistant_text,
            } => {
                let canonical = transcript_path
                    .canonicalize()
                    .map_err(|_| EvidenceCode::TranscriptUnavailable)?;
                let cursor = self
                    .input
                    .cursors
                    .iter()
                    .find(|cursor| {
                        cursor.format == format
                            && cursor.transcript_path.canonicalize().ok().as_ref()
                                == Some(&canonical)
                    })
                    .map(|cursor| cursor.cursor.clone());
                let remaining = self
                    .input
                    .limits
                    .max_total_bytes
                    .get()
                    .saturating_sub(self.total_read);
                let capture = capture_conversation(
                    ConversationCaptureRequest {
                        transcript_path,
                        format,
                        cursor,
                        final_assistant_text,
                        max_bytes: self.input.limits.max_transcript_bytes.get().min(remaining),
                    },
                    self.registry.clone(),
                    self.cancellation.clone(),
                )
                .await?;
                self.total_read = self
                    .total_read
                    .saturating_add(capture.coverage.bytes_read as usize);
                self.measurements.transcript_bytes_read += capture.coverage.bytes_read;
                self.measurements.transcript_bytes_parsed += capture.coverage.bytes_parsed;
                self.captures.push(capture.coverage.clone());
                self.cursors.push(ReturnedCursor {
                    transcript_path: capture.source.path.clone(),
                    format: capture.format.clone(),
                    cursor: capture.cursor.encode()?,
                });
                CapturedSource::Transcript(Arc::new(capture))
            }
            SourceSpec::Payload { .. } => return Err(EvidenceCode::UnknownPayloadSchema.into()),
            SourceSpec::ChangeSet { .. } => return Err(EvidenceCode::InvalidSelector.into()),
        };
        self.snapshot.sources.insert(name.clone(), captured.clone());
        Ok(captured)
    }
    async fn document(
        &mut self,
        repository: &RepositoryId,
        path: PathBuf,
    ) -> EvidenceResult<Arc<CapturedDocument>> {
        let binding = self
            .input
            .repositories
            .iter()
            .find(|binding| &binding.key == repository)
            .ok_or(EvidenceCode::InvalidSelector)?
            .clone();
        if !path_is_allowed(&binding, &path)? {
            return Err(EvidenceCode::PathOutsideRoot.into());
        }
        let root = binding
            .root
            .canonicalize()
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        let canonical = root
            .join(&path)
            .canonicalize()
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        if !canonical.starts_with(&root) {
            return Err(EvidenceCode::PathOutsideRoot.into());
        }
        let cache_key = (repository.clone(), canonical);
        if let Some(document) = self.document_cache.get(&cache_key) {
            return Ok(document.clone());
        }
        if self.measurements.files_read >= self.input.limits.max_files.get() {
            return Err(EvidenceCode::EvidenceLimit.into());
        }
        self.measurements.files_considered += 1;
        let remaining = self
            .input
            .limits
            .max_total_bytes
            .get()
            .saturating_sub(self.total_read);
        let document = capture_document(
            binding,
            path,
            self.input.limits.max_file_bytes.get().min(remaining),
            self.cancellation.clone(),
        )
        .await?;
        self.total_read += document.text.len();
        self.measurements.files_read += 1;
        let document = Arc::new(document);
        self.document_cache.insert(cache_key, document.clone());
        Ok(document)
    }
    async fn select(
        &mut self,
        declaration: &EvidenceDeclaration,
    ) -> EvidenceResult<SelectionReadiness> {
        let source = self.source(&declaration.input).await?;
        let (content, omissions, coverage) = match source {
            CapturedSource::Document(document) => {
                let doc = document.clone();
                let selection = declaration.select.clone();
                let max_matches = self.input.limits.max_matches.get();
                let content = tokio::task::spawn_blocking(move || {
                    select_document(&doc, &selection, max_matches)
                })
                .await
                .map_err(|_| EvidenceCode::InvalidSelector)??;
                self.measurements.files_parsed += 1;
                self.measurements.bytes_scanned += document.text.len();
                let coverage = CaptureCoverage {
                    source: document.source.clone(),
                    cursor_basis: CursorBasis::NoCursorSupplied,
                    tail: CaptureTail::Complete,
                    gaps: vec![],
                    bytes_read: document.text.len() as u64,
                    bytes_parsed: document.text.len() as u64,
                };
                (content, vec![], coverage)
            }
            CapturedSource::Transcript(capture) => {
                let selection = declaration
                    .select
                    .conversation()
                    .ok_or(EvidenceCode::InvalidSelector)?;
                let projection = select_conversation(&capture, &selection)?;
                (
                    SelectionContent::ConversationRecords {
                        source: capture.coverage.source.clone(),
                        records: projection.records,
                    },
                    projection.omissions,
                    projection.coverage,
                )
            }
            CapturedSource::Payload(payload) => {
                let pointer = match &declaration.select {
                    EvidenceSelection::WholePayload => "",
                    EvidenceSelection::PayloadField { pointer } => pointer,
                    _ => return Err(EvidenceCode::InvalidSelector.into()),
                };
                let content = payload.select(declaration.input.clone(), pointer)?;
                (
                    content,
                    vec![],
                    CaptureCoverage {
                        source: SourceRef::Payload {
                            group_position: 0,
                            input: declaration.input.clone(),
                        },
                        cursor_basis: CursorBasis::NoCursorSupplied,
                        tail: CaptureTail::Complete,
                        gaps: vec![],
                        bytes_read: 0,
                        bytes_parsed: 0,
                    },
                )
            }
        };
        self.admit_selection(content, omissions, coverage, declaration)
    }
    fn admit_selection(
        &mut self,
        content: SelectionContent,
        omissions: Vec<Omission>,
        coverage: CaptureCoverage,
        declaration: &EvidenceDeclaration,
    ) -> EvidenceResult<SelectionReadiness> {
        let items = match &content {
            SelectionContent::Matches { matches } => matches.len(),
            SelectionContent::ConversationRecords { records, .. } => records.len(),
            _ => 1,
        };
        let bytes = serde_json::to_vec(&content)
            .map_err(|_| EvidenceCode::EvidenceLimit)?
            .len();
        if declaration
            .max_items
            .is_some_and(|limit| items > limit.get())
            || declaration
                .max_bytes
                .is_some_and(|limit| bytes > limit.get())
        {
            return Err(EvidenceCode::SelectorLimit.into());
        }
        self.measurements.bytes_included += bytes;
        Ok(SelectionReadiness::Ready {
            content,
            omissions,
            coverage: Box::new(coverage),
        })
    }
}

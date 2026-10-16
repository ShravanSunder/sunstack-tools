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
pub(crate) struct Preparation {
    input: EvidencePlanInput,
    registry: AdapterRegistry,
    cancellation: CancellationToken,
    snapshot: SourceSnapshot,
    captures: Vec<CaptureCoverage>,
    cursors: Vec<ReturnedCursor>,
    measurements: EvidenceMeasurements,
    total_read: usize,
    document_cache: BTreeMap<(RepositoryId, PathBuf), Arc<CapturedDocument>>,
}
/// Direct evidence calls use the same invocation-local session as native rules callers.
pub async fn prepare_evidence(
    input: EvidencePlanInput,
    registry: AdapterRegistry,
    cancellation: CancellationToken,
) -> EvidencePlanOutcome {
    if cancellation.is_cancelled() {
        return EvidencePlanOutcome::Cancelled {
            progress: EvidenceCancellationProgress::BeforePlanAdmission,
            diagnostics: DiagnosticList::default(),
            measurements: EvidenceMeasurements::default(),
        };
    }
    let declarations = input.declarations.clone();
    let queries = input.native_queries.clone();
    let mut session = match EvidenceSession::open(input, registry, cancellation.clone()).await {
        Ok(session) => session,
        Err(error) => {
            return EvidencePlanOutcome::Refused {
                diagnostics: error.diagnostics(),
            };
        }
    };
    let mut selections = Vec::new();
    let mut diagnostics = DiagnosticList::default();
    for declaration in declarations {
        if cancellation.is_cancelled() {
            break;
        }
        let result = session.select(&declaration).await;
        if declaration.requirement == EvidenceRequirement::Required
            && let SelectionReadiness::Refused {
                diagnostics: refused,
            } = &result.readiness
        {
            for diagnostic in &refused.items {
                diagnostics.push(diagnostic.clone());
            }
        }
        selections.push(result);
    }
    for query in queries {
        if cancellation.is_cancelled() {
            break;
        }
        let result = session.select_native(&query).await;
        if let SelectionReadiness::Refused {
            diagnostics: refused,
        } = &result.readiness
        {
            for diagnostic in &refused.items {
                diagnostics.push(diagnostic.clone());
            }
        }
        selections.push(result);
    }
    let EvidenceSessionSummary {
        captures,
        cursors,
        measurements,
    } = session.close();
    if cancellation.is_cancelled() {
        EvidencePlanOutcome::Cancelled {
            progress: EvidenceCancellationProgress::PlanAdmitted {
                partial: PartialEvidencePlan {
                    selections,
                    captures,
                    cursors,
                },
            },
            diagnostics,
            measurements,
        }
    } else if diagnostics.items.is_empty() {
        EvidencePlanOutcome::Prepared {
            selections,
            captures,
            cursors,
            measurements,
        }
    } else {
        EvidencePlanOutcome::Incomplete {
            selections,
            captures,
            cursors,
            measurements,
            diagnostics,
        }
    }
}
impl Preparation {
    pub(crate) async fn open(
        mut input: EvidencePlanInput,
        registry: AdapterRegistry,
        cancellation: CancellationToken,
    ) -> EvidenceResult<Self> {
        // Freeze lexical source identities at admission; later aliases never consult live paths.
        for source in input.sources.values_mut() {
            match source {
                SourceSpec::Document { path, .. } => {
                    *path = lexical_path(path)
                        .filter(|path| !path.is_absolute() && !path.as_os_str().is_empty())
                        .ok_or(EvidenceCode::PathOutsideRoot)?;
                }
                SourceSpec::Conversation {
                    transcript_path, ..
                } => {
                    let absolute = if transcript_path.is_absolute() {
                        transcript_path.clone()
                    } else {
                        std::env::current_dir()
                            .map_err(|_| EvidenceCode::TranscriptUnavailable)?
                            .join(&*transcript_path)
                    };
                    *transcript_path =
                        lexical_path(&absolute).ok_or(EvidenceCode::TranscriptUnavailable)?;
                }
                SourceSpec::Payload { .. } | SourceSpec::ChangeSet { .. } => {}
            }
        }
        crate::evidence_admission::admission(&input, &registry)?;
        let mut preparation = Self {
            input,
            registry,
            cancellation,
            snapshot: SourceSnapshot::default(),
            captures: Vec::new(),
            cursors: Vec::new(),
            measurements: EvidenceMeasurements::default(),
            total_read: 0,
            document_cache: BTreeMap::new(),
        };
        preparation.admit_payloads().await?;
        Ok(preparation)
    }
    pub(crate) fn close(self, started: Instant) -> EvidenceSessionSummary {
        let mut measurements = self.measurements;
        measurements.local_elapsed_ms =
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        EvidenceSessionSummary {
            captures: self.captures,
            cursors: self.cursors,
            measurements,
        }
    }
    pub(crate) fn measurements(&self) -> &EvidenceMeasurements {
        &self.measurements
    }

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
            let same = self.input.sources.get(alias) == Some(&spec);
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
                let progress = crate::acquisition_progress::AcquisitionProgress::shared();
                let result = crate::conversation_capture::capture_conversation_accounted(
                    crate::conversation_capture::ConversationAcquisition {
                        request: ConversationCaptureRequest {
                            transcript_path,
                            format,
                            cursor,
                            final_assistant_text,
                            max_bytes: self.input.limits.max_transcript_bytes.get().min(remaining),
                        },
                        registry: self.registry.clone(),
                        cancellation: self.cancellation.clone(),
                        progress: progress.clone(),
                    },
                )
                .await;
                self.total_read += progress.consumed_bytes();
                self.measurements.transcript_bytes_read +=
                    progress.consumed_transcript_bytes() as u64;
                let capture = result?;
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
        let path = lexical_path(&path)
            .filter(|path| !path.is_absolute() && !path.as_os_str().is_empty())
            .ok_or(EvidenceCode::PathOutsideRoot)?;
        let cache_key = (repository.clone(), path.clone());
        if let Some(document) = self.document_cache.get(&cache_key) {
            return Ok(document.clone());
        }
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
        let progress = crate::acquisition_progress::AcquisitionProgress::shared();
        let result = crate::source_capture::capture_document_accounted(
            crate::source_capture::DocumentCapture {
                binding,
                path,
                max_bytes: self.input.limits.max_file_bytes.get().min(remaining),
                cancellation: self.cancellation.clone(),
                progress: progress.clone(),
            },
        )
        .await;
        self.total_read += progress.consumed_bytes();
        self.measurements.files_read += progress.consumed_repository_files();
        let document = result?;
        let document = Arc::new(document);
        self.document_cache.insert(cache_key, document.clone());
        Ok(document)
    }
    pub(crate) async fn select(
        &mut self,
        declaration: &EvidenceDeclaration,
    ) -> EvidenceResult<SelectionReadiness> {
        crate::evidence_admission::admit_declaration(declaration, &self.input.sources)?;
        if self.cancellation.is_cancelled() {
            return Err(EvidenceCode::MissingContext.into());
        }
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

fn lexical_path(path: &std::path::Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            other => normalized.push(other),
        }
    }
    Some(normalized)
}

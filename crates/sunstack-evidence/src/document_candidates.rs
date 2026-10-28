use crate::*;
use std::{collections::BTreeSet, sync::Arc, time::Instant};

#[derive(Clone, Debug)]
pub struct DiscoveredDocumentMatch {
    pub occurrence: SourceOccurrence,
    pub content: SelectionContent,
    pub source: SourceRef,
}
impl DiscoveredDocumentMatch {
    pub fn visible_text(&self) -> String {
        match &self.content {
            SelectionContent::MarkdownUnits { units } => units
                .first()
                .map(|unit| unit.visible_text.clone())
                .unwrap_or_default(),
            SelectionContent::FrontmatterField { result } => match result.as_ref() {
                FrontmatterFieldContent::Present { value, .. } => match value {
                    StructuredJson::String(text) => text.clone(),
                    _ => serde_json::to_string(value).unwrap_or_default(),
                },
                _ => String::new(),
            },
            _ => String::new(),
        }
    }
    pub fn project(
        &self,
        grants: &BTreeSet<PrivateKind>,
    ) -> EvidenceResult<(Self, PrivacyProjection)> {
        let mut selected = self.clone();
        let mut privacy = PrivacyProjection::default();
        match &mut selected.content {
            SelectionContent::MarkdownUnits { units } => {
                for unit in units {
                    let (text, counts) = project_text(&unit.text, grants)?;
                    unit.text = text;
                    privacy.accumulate(counts);
                    let (text, counts) = project_text(&unit.visible_text, grants)?;
                    unit.visible_text = text;
                    privacy.accumulate(counts);
                }
            }
            SelectionContent::FrontmatterField { result } => {
                if let FrontmatterFieldContent::Present { value, .. } = result.as_mut() {
                    let (projected, counts) = project_structured_json(value, grants)?;
                    *value = projected;
                    privacy.accumulate(counts);
                }
            }
            _ => return Err(EvidenceCode::InvalidSelector.into()),
        }
        Ok((selected, privacy))
    }
}
impl MechanicalPass {
    pub fn validate_document(&self) -> EvidenceResult<()> {
        fn text_only(predicate: &MechanicalPredicate) -> bool {
            match predicate {
                MechanicalPredicate::Regex { field, .. }
                | MechanicalPredicate::Literal { field, .. } => field.as_str() == "/text",
                MechanicalPredicate::Any { predicates }
                | MechanicalPredicate::All { predicates } => {
                    !predicates.is_empty() && predicates.iter().all(text_only)
                }
                MechanicalPredicate::Ast { .. } => false,
            }
        }
        self.validate()?;
        if self.context_parts.is_some()
            || self.source_sides.is_some()
            || !text_only(&self.predicate)
        {
            return Err(EvidenceError {
                code: EvidenceCode::InvalidSelector,
                field_path: vec!["mechanical_pass".into(), "predicate".into()],
            });
        }
        Ok(())
    }
}
fn keeps(predicate: &MechanicalPredicate, text: &str) -> EvidenceResult<bool> {
    Ok(match predicate {
        MechanicalPredicate::Regex { pattern, .. } => regex::Regex::new(pattern)
            .map_err(|_| EvidenceCode::InvalidSelector)?
            .is_match(text),
        MechanicalPredicate::Literal { text: needle, .. } => text.contains(needle),
        MechanicalPredicate::Any { predicates } => {
            let mut found = false;
            for predicate in predicates {
                found |= keeps(predicate, text)?;
            }
            found
        }
        MechanicalPredicate::All { predicates } => {
            let mut found = true;
            for predicate in predicates {
                found &= keeps(predicate, text)?;
            }
            found
        }
        MechanicalPredicate::Ast { .. } => return Err(EvidenceCode::InvalidSelector.into()),
    })
}
impl CodeContextIndex {
    /// Select document subjects from this index's captured bytes before filtering them.
    pub fn discover_documents(
        &self,
        target: &EvidenceSelection,
        pass: Option<&MechanicalPass>,
        samples: usize,
    ) -> EvidenceResult<Discovery<DiscoveredDocumentMatch>> {
        if !matches!(self.input, SourceInput::Code { .. }) {
            return Err(EvidenceCode::InvalidSelector.into());
        }
        if let Some(pass) = pass {
            pass.validate_document()?;
        }
        let mut result = Discovery::default();
        let mut rejected = Vec::new();
        let started = Instant::now();
        let mut selected_count = 0;
        for file in &self.capture.sources {
            if self.cancellation.is_cancelled()
                || started.elapsed().as_millis() >= self.limits.max_local_time_ms.get() as u128
            {
                result.completed = false;
                break;
            }
            let SourceRef::RepositoryFile { path, .. } = &file.source else {
                continue;
            };
            if crate::source_units::source_language(path) != Some(SourceLanguage::Markdown) {
                continue;
            }
            let SourceInput::Code { paths, .. } = &self.input else {
                return Err(EvidenceCode::InvalidSelector.into());
            };
            let binding = RepositoryBinding {
                key: self.repository.clone(),
                root: Default::default(),
                input_paths: paths.clone(),
                change_scope: ChangeScope::Full {},
            };
            if !path_is_allowed(&binding, path)? {
                continue;
            }
            let remaining = self.limits.max_matches.get().saturating_sub(selected_count);
            let mut selected = Vec::new();
            match target {
                EvidenceSelection::MarkdownUnits(selection) => {
                    let (units, omitted) = MarkdownDocument::parse(&file.text).select_units(
                        &file.source,
                        selection,
                        remaining,
                    )?;
                    if omitted {
                        result.completed = false;
                    }
                    for unit in units {
                        selected.push(DiscoveredDocumentMatch {
                            occurrence: SourceOccurrence::DocumentUnit {
                                input_id: self.input_id.clone(),
                                unit_kind: unit.kind,
                                occurrence: unit.occurrence.clone(),
                            },
                            source: file.source.clone(),
                            content: SelectionContent::MarkdownUnits { units: vec![unit] },
                        });
                    }
                }
                EvidenceSelection::FrontmatterField { field } => {
                    if remaining == 0 {
                        result.completed = false;
                        break;
                    }
                    let document = CapturedDocument {
                        interval: CaptureInterval {
                            started_at: self.capture.anchor_utc,
                            completed_at: self.capture.anchor_utc,
                        },
                        source: file.source.clone(),
                        text: Arc::from(file.text.as_str()),
                    };
                    let content =
                        crate::markdown_frontmatter::select_frontmatter(&document, field)?;
                    let SelectionContent::FrontmatterField {
                        result: field_content,
                    } = &content
                    else {
                        return Err(EvidenceCode::InvalidSelector.into());
                    };
                    let state = match field_content.as_ref() {
                        FrontmatterFieldContent::Present { span, .. } => {
                            FrontmatterFieldLocation::Present { span: span.clone() }
                        }
                        FrontmatterFieldContent::Absent {
                            frontmatter_span, ..
                        } => FrontmatterFieldLocation::Absent {
                            frontmatter_span: frontmatter_span.clone(),
                        },
                        FrontmatterFieldContent::BlockUnavailable { reason, .. } => {
                            FrontmatterFieldLocation::BlockUnavailable { reason: *reason }
                        }
                    };
                    selected.push(DiscoveredDocumentMatch {
                        occurrence: SourceOccurrence::DocumentField {
                            input_id: self.input_id.clone(),
                            source: file.source.clone(),
                            field: field.clone(),
                            state: Box::new(state),
                        },
                        source: file.source.clone(),
                        content,
                    });
                }
                _ => return Err(EvidenceCode::InvalidSelector.into()),
            }
            selected_count += selected.len();
            for item in selected {
                if pass
                    .map(|pass| keeps(&pass.predicate, &item.visible_text()))
                    .transpose()?
                    .unwrap_or(true)
                {
                    result.matches.push(item);
                } else {
                    rejected.push(item);
                }
            }
            if !result.completed {
                break;
            }
        }
        let count = samples.min(rejected.len());
        for position in 0..count {
            result
                .rejected_sample
                .push(rejected[position * rejected.len() / count].clone());
        }
        let mut coverage = self.coverage.clone();
        if !result.completed {
            let (query_bounds, mut cutoffs) = match coverage {
                DiscoveryCoverage::CompleteWithinDeclaredScope { query_bounds } => {
                    (query_bounds, vec![])
                }
                DiscoveryCoverage::Limited {
                    query_bounds,
                    cutoffs,
                } => (query_bounds, cutoffs),
                DiscoveryCoverage::Unavailable { .. } => {
                    result.coverage = Some(coverage);
                    return Ok(result);
                }
            };
            cutoffs.push(DiscoveryCutoff {
                limit: if self.cancellation.is_cancelled()
                    || started.elapsed().as_millis() >= self.limits.max_local_time_ms.get() as u128
                {
                    DiscoveryLimit::LocalTime
                } else {
                    DiscoveryLimit::Matches
                },
                frontier: vec![],
            });
            coverage = DiscoveryCoverage::Limited {
                query_bounds,
                cutoffs,
            };
        }
        result.coverage = Some(coverage);
        Ok(result)
    }
}

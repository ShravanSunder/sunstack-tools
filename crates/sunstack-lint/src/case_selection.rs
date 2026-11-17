use crate::{IncompletePart, IncompleteReason, NeedsSelectionAnswer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};
use sunstack_evidence::{
    EvidenceCatalog, EvidenceKindName, EvidencePiece, FieldUnavailable, FieldValue,
};
use sunstack_rules::{
    FilterFolder, FilterIncomplete, FilterName, FilterRegistry, ReadsPiece, RuleId, RulebookRule,
};

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CaseBinding {
    pub id: String,
    #[serde(flatten)]
    pub subject: CaseSubjectBinding,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CaseSubjectBinding {
    Rule {
        rule: RuleId,
        file: PathBuf,
        lines: [usize; 2],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bytes: Option<[usize; 2]>,
        text_sha256: String,
    },
    Filter {
        filter: FilterName,
        folder: PathBuf,
        reads_sha256: String,
    },
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaseExecutionOwner {
    Rule {
        rule: RuleId,
        file: PathBuf,
        lines: [usize; 2],
        bytes: [usize; 2],
    },
    Filter {
        filter: FilterName,
        folder: PathBuf,
    },
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaseBindingStatus {
    Selected {
        owner: CaseExecutionOwner,
    },
    Unmatched,
    Ambiguous {
        owners: Vec<CaseExecutionOwner>,
    },
    Stale,
    SelectionPending {
        evidence_kind: EvidenceKindName,
        file: PathBuf,
    },
    Incomplete {
        reason: IncompleteReason,
    },
    Unavailable {
        reason: sunstack_rules::WorkUnavailableReason,
    },
    Cancelled,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CaseBindingResult {
    pub binding: CaseBinding,
    pub outcome: CaseBindingStatus,
}

#[cfg_attr(feature = "typescript", derive(specta::Type))]
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CaseSelectionReport {
    pub bindings: Vec<CaseBindingResult>,
    pub excluded: Vec<CaseExecutionOwner>,
}

pub(crate) struct CaseSelectionPlan {
    pub report: CaseSelectionReport,
    pub allowed_rule_pieces: BTreeSet<(usize, usize)>,
    pub allowed_filters: BTreeSet<(FilterName, PathBuf)>,
}

fn is_repository_relative(path: &Path) -> bool {
    !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
}

pub(crate) fn validate_case_bindings(bindings: &[CaseBinding]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for binding in bindings {
        if binding.id.trim().is_empty() || !ids.insert(&binding.id) {
            return Err(format!(
                "case selection id is empty or duplicated: {:?}",
                binding.id
            ));
        }
        let path = match &binding.subject {
            CaseSubjectBinding::Rule { file, .. } => file,
            CaseSubjectBinding::Filter { folder, .. } => folder,
        };
        if !is_repository_relative(path) {
            return Err(format!(
                "case selection path must be repository-relative: {path:?}"
            ));
        }
        if let CaseSubjectBinding::Rule {
            bytes: Some([start, end]),
            ..
        } = &binding.subject
            && start >= end
        {
            return Err(format!(
                "case selection {} needs byte_start < byte_end",
                binding.id
            ));
        }
    }
    Ok(())
}

fn rule_owner(rule: &RulebookRule, piece: &EvidencePiece) -> CaseExecutionOwner {
    CaseExecutionOwner::Rule {
        rule: rule.name.clone(),
        file: piece.location.file.clone(),
        lines: [piece.location.first_line, piece.location.last_line],
        bytes: [piece.location.span.byte_start, piece.location.span.byte_end],
    }
}

fn filter_owner(folder: &FilterFolder) -> CaseExecutionOwner {
    CaseExecutionOwner::Filter {
        filter: folder.filter.clone(),
        folder: folder.folder.clone(),
    }
}

// Rule out a global summary only when this binding has complete local evidence.
fn file_kind_is_settled(
    file: &Path,
    kind: Option<&EvidenceKindName>,
    catalog: &EvidenceCatalog,
) -> bool {
    let Some(kind) = kind else {
        return false;
    };
    let Some(index) = catalog.matches.get(file) else {
        return false;
    };
    if index.limited
        || !index.failed_languages.is_empty()
        || catalog.failures.iter().any(|failure| failure.file == file)
    {
        return false;
    }
    let Some(count) = catalog.file_counts.iter().find(|count| count.file == file) else {
        return false;
    };
    count.kinds.contains_key(kind)
        && count.kinds.values().all(|count| {
            !count.limited
                && count.selection.as_ref().is_none_or(|selection| {
                    selection.incomplete == 0
                        && selection.unavailable == 0
                        && selection.uncertain == 0
                        && selection.asked == selection.candidates
                        && selection.selected.checked_add(selection.not_selected)
                            == Some(selection.candidates)
                })
        })
}

fn missing_status(
    file: &PathBuf,
    kind: Option<&EvidenceKindName>,
    catalog: &EvidenceCatalog,
    needs_answer: &[NeedsSelectionAnswer],
    incomplete: &[IncompletePart],
) -> CaseBindingStatus {
    if let Some((evidence_kind, blocked_file)) =
        incomplete.iter().find_map(|part| match &part.reason {
            IncompleteReason::SelectionUnavailable {
                evidence_kind,
                file: blocked_file,
                ..
            } if blocked_file == file && kind.is_none_or(|kind| evidence_kind == kind) => {
                Some((evidence_kind, blocked_file))
            }
            _ => None,
        })
    {
        return CaseBindingStatus::Unavailable {
            reason: sunstack_rules::WorkUnavailableReason::SelectionUnavailable {
                evidence_kind: evidence_kind.clone(),
                file: blocked_file.clone(),
            },
        };
    }
    if let Some(wait) = needs_answer
        .iter()
        .find(|entry| &entry.file == file && kind.is_none_or(|kind| &entry.kind == kind))
    {
        return CaseBindingStatus::SelectionPending {
            evidence_kind: wait.kind.clone(),
            file: wait.file.clone(),
        };
    }
    if let Some(part) = incomplete.iter().find(|part| {
        part.file.as_ref() == Some(file)
            && !matches!(part.reason, IncompleteReason::SelectionUnavailable { .. })
    }) {
        return CaseBindingStatus::Incomplete {
            reason: part.reason.clone(),
        };
    }
    if catalog.failures.iter().any(|failure| &failure.file == file) {
        return CaseBindingStatus::Incomplete {
            reason: IncompleteReason::EvidenceUnavailable,
        };
    }
    let settled = file_kind_is_settled(file, kind, catalog);
    if let Some(part) = incomplete.iter().find(|part| {
        part.file.is_none()
            && (!settled
                || !matches!(
                    part.reason,
                    IncompleteReason::CoveragePartial
                        | IncompleteReason::SelectionFailed
                        | IncompleteReason::SelectionIncomplete
                ))
    }) {
        return CaseBindingStatus::Incomplete {
            reason: part.reason.clone(),
        };
    }
    CaseBindingStatus::Unmatched
}

fn filter_digest(
    folder: &FilterFolder,
    filters: &FilterRegistry,
    catalog: &EvidenceCatalog,
) -> Result<String, IncompleteReason> {
    let reason = |reason| IncompleteReason::Filter {
        filter: folder.filter.clone(),
        reason,
    };
    let plan = filters
        .get(&folder.filter)
        .ok_or_else(|| reason(FilterIncomplete::Undescribed))?;
    let index = match folder.reads {
        ReadsPiece::One(index) => index,
        ReadsPiece::None => return Err(reason(FilterIncomplete::Undescribed)),
        ReadsPiece::Ambiguous => return Err(reason(FilterIncomplete::Ambiguous)),
        ReadsPiece::Unavailable(value) => {
            return Err(reason(FilterIncomplete::ReadsUnavailable { reason: value }));
        }
    };
    let piece = catalog
        .pieces
        .get(index)
        .ok_or_else(|| reason(FilterIncomplete::Undescribed))?;
    let mut fields = std::collections::BTreeMap::new();
    for field in &plan.reads.fields {
        let value = match piece.fields.get(field) {
            Some(FieldValue::Unavailable {
                reason: unavailable,
            }) => {
                return Err(reason(FilterIncomplete::ReadsUnavailable {
                    reason: *unavailable,
                }));
            }
            Some(FieldValue::Absent | FieldValue::MissingDocument { .. }) | None => {
                return Err(reason(FilterIncomplete::ReadsAbsent));
            }
            Some(value) => value.clone(),
        };
        fields.insert(field.clone(), value);
    }
    sunstack_rules::filter_reads_hash(&fields).map_err(reason)
}

pub(crate) struct CaseSelectionInputs<'a> {
    pub rules: &'a [RulebookRule],
    pub pieces: &'a [EvidencePiece],
    pub catalog: &'a EvidenceCatalog,
    pub folders: &'a [FilterFolder],
    pub filters: &'a FilterRegistry,
    pub needs_answer: &'a [NeedsSelectionAnswer],
    pub incomplete: &'a [IncompletePart],
    pub cancelled: bool,
}

pub(crate) fn plan_case_selection(
    bindings: &[CaseBinding],
    inputs: CaseSelectionInputs<'_>,
) -> CaseSelectionPlan {
    let CaseSelectionInputs {
        rules,
        pieces,
        catalog,
        folders,
        filters,
        needs_answer,
        incomplete,
        cancelled,
    } = inputs;
    let mut allowed_rule_pieces = BTreeSet::new();
    let mut allowed_filters = BTreeSet::new();
    let mut results = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let outcome = if cancelled {
            CaseBindingStatus::Cancelled
        } else {
            match &binding.subject {
                CaseSubjectBinding::Rule {
                    rule,
                    file,
                    lines,
                    bytes,
                    text_sha256,
                } => {
                    let matches = rules
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| &candidate.name == rule)
                        .flat_map(|(ri, candidate)| {
                            pieces
                                .iter()
                                .enumerate()
                                .filter(move |(_, piece)| {
                                    piece.kind == candidate.kind
                                        && &piece.location.file == file
                                        && piece.location.first_line == lines[0]
                                        && piece.location.last_line == lines[1]
                                        && bytes.is_none_or(|bytes| {
                                            bytes
                                                == [
                                                    piece.location.span.byte_start,
                                                    piece.location.span.byte_end,
                                                ]
                                        })
                                })
                                .map(move |(pi, piece)| (ri, pi, rule_owner(candidate, piece)))
                        })
                        .collect::<Vec<_>>();
                    let captured_hash = match bytes {
                        Some([start, end]) => {
                            catalog.piece_byte_hashes.get(&(file.clone(), *start, *end))
                        }
                        None => catalog
                            .piece_text_hashes
                            .get(&(file.clone(), lines[0], lines[1])),
                    };
                    // Captured source mismatch settles identity despite later scan/selection gaps.
                    if captured_hash.is_some_and(|hash| hash != text_sha256) {
                        CaseBindingStatus::Stale
                    } else if matches.is_empty() {
                        let kind = rules
                            .iter()
                            .find(|candidate| &candidate.name == rule)
                            .map(|rule| &rule.kind);
                        let missing = missing_status(file, kind, catalog, needs_answer, incomplete);
                        if !matches!(missing, CaseBindingStatus::Unmatched) {
                            missing
                        } else {
                            match captured_hash {
                                None if bytes.is_none() && catalog.matches.contains_key(file) => {
                                    CaseBindingStatus::Stale
                                }
                                None => CaseBindingStatus::Unmatched,
                                Some(_) => CaseBindingStatus::Unmatched,
                            }
                        }
                    } else {
                        match captured_hash {
                            None => CaseBindingStatus::Incomplete {
                                reason: IncompleteReason::EvidenceUnavailable,
                            },
                            Some(_) => match matches.as_slice() {
                                [(ri, pi, owner)] => {
                                    allowed_rule_pieces.insert((*ri, *pi));
                                    CaseBindingStatus::Selected {
                                        owner: owner.clone(),
                                    }
                                }
                                _ => CaseBindingStatus::Ambiguous {
                                    owners: matches
                                        .into_iter()
                                        .map(|(_, _, owner)| owner)
                                        .collect(),
                                },
                            },
                        }
                    }
                }
                CaseSubjectBinding::Filter {
                    filter,
                    folder,
                    reads_sha256,
                } => {
                    let matches = folders
                        .iter()
                        .filter(|candidate| {
                            &candidate.filter == filter && &candidate.folder == folder
                        })
                        .collect::<Vec<_>>();
                    match matches.as_slice() {
                        [] => missing_status(folder, None, catalog, needs_answer, incomplete),
                        [candidate] => match filter_digest(candidate, filters, catalog) {
                            Ok(hash) if &hash == reads_sha256 => {
                                allowed_filters.insert((filter.clone(), folder.clone()));
                                CaseBindingStatus::Selected {
                                    owner: filter_owner(candidate),
                                }
                            }
                            Ok(_) => CaseBindingStatus::Stale,
                            Err(IncompleteReason::Filter {
                                reason:
                                    FilterIncomplete::ReadsUnavailable {
                                        reason:
                                            FieldUnavailable::SelectionPending
                                            | FieldUnavailable::SelectionUnavailable,
                                    },
                                ..
                            }) => {
                                let input = filters
                                    .get(filter)
                                    .map(|plan| folder.join(&plan.reads.file))
                                    .unwrap_or_else(|| folder.clone());
                                match missing_status(
                                    &input,
                                    filters.get(filter).map(|plan| &plan.reads.kind),
                                    catalog,
                                    needs_answer,
                                    incomplete,
                                ) {
                                    CaseBindingStatus::Unmatched => CaseBindingStatus::Incomplete {
                                        reason: IncompleteReason::SelectionIncomplete,
                                    },
                                    status => status,
                                }
                            }
                            Err(reason) => CaseBindingStatus::Incomplete { reason },
                        },
                        _ => CaseBindingStatus::Ambiguous {
                            owners: matches.into_iter().map(filter_owner).collect(),
                        },
                    }
                }
            }
        };
        results.push(CaseBindingResult {
            binding: binding.clone(),
            outcome,
        });
    }
    for (ri, pi) in &allowed_rule_pieces {
        let rule = &rules[*ri];
        let piece = &pieces[*pi];
        if let Some(gate) = &rule.gate {
            for folder in folders.iter().filter(|folder| {
                folder.filter == gate.filter && folder.members.contains(&piece.location.file)
            }) {
                allowed_filters.insert((folder.filter.clone(), folder.folder.clone()));
            }
        }
    }
    let mut excluded = Vec::new();
    for (ri, rule) in rules.iter().enumerate() {
        for (pi, piece) in pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| piece.kind == rule.kind)
        {
            if !allowed_rule_pieces.contains(&(ri, pi)) {
                excluded.push(rule_owner(rule, piece));
            }
        }
    }
    for folder in folders {
        if !allowed_filters.contains(&(folder.filter.clone(), folder.folder.clone())) {
            excluded.push(filter_owner(folder));
        }
    }
    CaseSelectionPlan {
        report: CaseSelectionReport {
            bindings: results,
            excluded,
        },
        allowed_rule_pieces,
        allowed_filters,
    }
}

#[cfg(test)]
#[path = "case_selection_test.rs"]
mod tests;

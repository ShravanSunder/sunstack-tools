use super::{CaseResult, CasesReport, RevisionRun};

/// A prepared case report uses its stable case-array order, not colliding nested request IDs.
pub fn show_prepared_case(
    report: &CasesReport,
    ordinal: usize,
) -> Result<serde_json::Value, String> {
    if report.preparation.is_none() {
        return Err("case readback requires a prepared report".into());
    }
    let case = ordinal
        .checked_sub(1)
        .and_then(|index| report.cases.get(index))
        .ok_or_else(|| format!("unknown case ordinal {ordinal}"))?;
    show_case_detail(report, case)
}

/// Prepared ordinals retain priority; completed selectors gather both case and result matches.
pub fn show_case(report: &CasesReport, selector: &str) -> Result<serde_json::Value, String> {
    let numeric_id = selector.parse::<usize>().ok();
    if report.preparation.is_some()
        && let Some(ordinal) = numeric_id
        && ordinal
            .checked_sub(1)
            .and_then(|index| report.cases.get(index))
            .is_some()
    {
        return show_prepared_case(report, ordinal);
    }
    let mut candidates = report
        .cases
        .iter()
        .filter(|case| case.id == selector)
        .map(SavedCaseCandidate::Case)
        .collect::<Vec<_>>();
    if report.preparation.is_none()
        && let Some(id) = numeric_id
    {
        collect_numeric_results(report, id, &mut candidates);
    }
    match candidates.as_slice() {
        [] => match numeric_id {
            Some(id) if report.preparation.is_some() => Err(format!("unknown case ordinal {id}")),
            Some(id) => Err(format!("unknown result id {id}")),
            None => Err(format!("unknown case id {selector}")),
        },
        [SavedCaseCandidate::Case(case)] => show_case_detail(report, case),
        [SavedCaseCandidate::Result { run, id }] => {
            crate::lint_reporting::saved_reports::show(&run.report, *id)
        }
        _ => {
            let descriptions = candidates
                .iter()
                .flat_map(|candidate| candidate.descriptions(report))
                .collect::<Vec<_>>();
            let kind = if numeric_id.is_some() && report.preparation.is_none() {
                "result"
            } else {
                "case"
            };
            Err(format!(
                "ambiguous {kind} id {selector}; candidates: {}",
                descriptions.join("; ")
            ))
        }
    }
}

enum SavedCaseCandidate<'a> {
    Case(&'a CaseResult),
    Result { run: &'a RevisionRun, id: usize },
}

impl SavedCaseCandidate<'_> {
    fn descriptions(&self, report: &CasesReport) -> Vec<String> {
        match self {
            Self::Case(case) => {
                let result = case.result_id.map_or_else(
                    || "no result id".to_string(),
                    |id| format!("result id {id}"),
                );
                vec![format!(
                    "revision {}, case id {}, {result}",
                    case.definition.revision, case.id
                )]
            }
            Self::Result { run, id } => {
                let cases = report
                    .cases
                    .iter()
                    .filter(|case| {
                        case.definition.revision == run.revision && case.result_id == Some(*id)
                    })
                    .map(|case| format!("case id {}", case.id))
                    .collect::<Vec<_>>();
                if cases.is_empty() {
                    vec![format!(
                        "revision {}, no case id, result id {id}",
                        run.revision
                    )]
                } else {
                    cases
                        .iter()
                        .map(|case| format!("revision {}, {case}, result id {id}", run.revision))
                        .collect()
                }
            }
        }
    }
}

fn collect_numeric_results<'a>(
    report: &'a CasesReport,
    id: usize,
    candidates: &mut Vec<SavedCaseCandidate<'a>>,
) {
    let selected_cases = candidates
        .iter()
        .filter_map(|candidate| match candidate {
            SavedCaseCandidate::Case(case) => Some(*case),
            SavedCaseCandidate::Result { .. } => None,
        })
        .collect::<Vec<_>>();
    let mut linked_cases = vec![false; selected_cases.len()];
    for run in &report.runs {
        for result_id in run
            .report
            .results
            .iter()
            .map(|result| result.id)
            .chain(run.report.filter_answers.iter().map(|result| result.id))
            .chain(run.report.filter_skips.iter().map(|result| result.id))
            .filter(|result_id| *result_id == id)
        {
            let mut merged_with_case = false;
            for (index, case) in selected_cases.iter().enumerate() {
                if !linked_cases[index]
                    && case.definition.revision == run.revision
                    && case.result_id == Some(result_id)
                {
                    linked_cases[index] = true;
                    merged_with_case = true;
                }
            }
            // Only one result row can collapse into each case. Duplicate rows remain candidates.
            if !merged_with_case {
                candidates.push(SavedCaseCandidate::Result { run, id: result_id });
            }
        }
    }
}

fn show_case_detail(report: &CasesReport, case: &CaseResult) -> Result<serde_json::Value, String> {
    let mut detail = serde_json::json!({"case": case, "revision": case.definition.revision});
    if let Some(id) = case.result_id {
        let runs = report
            .runs
            .iter()
            .filter(|run| run.revision == case.definition.revision)
            .collect::<Vec<_>>();
        let run = match runs.as_slice() {
            [] => return Err("case revision evidence is missing".into()),
            [run] => run,
            _ => {
                let descriptions = runs
                    .iter()
                    .map(|run| {
                        format!(
                            "revision {}, case id {}, result id {id} (resolved revision {})",
                            run.revision, case.id, run.resolved_revision
                        )
                    })
                    .collect::<Vec<_>>();
                return Err(format!(
                    "ambiguous case revision evidence; candidates: {}",
                    descriptions.join("; ")
                ));
            }
        };
        detail["detail"] = crate::lint_reporting::saved_reports::show(&run.report, id)?;
    }
    Ok(detail)
}

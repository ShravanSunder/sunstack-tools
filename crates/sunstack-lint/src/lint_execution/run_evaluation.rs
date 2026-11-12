use super::{
    lint_run::{RunState, failed},
    run_coverage::PickedEvidence,
};
use crate::{configuration_loading::config_validation::CheckedRulebook, *};
use sunstack_rules::run_rulebook_with_dispatcher;

pub(super) async fn evaluate_run(
    state: RunState,
    config: &RulesConfig,
    checked: &CheckedRulebook,
    evidence: PickedEvidence,
) -> LintReport {
    let (catalog, pieces, folder_plan, mut dispatcher, acquired) = evidence;
    let request = state.request;
    let cancellation = state.cancellation;
    let preparing = state.preparing;
    let mut report = state.report;
    let folders = super::filter_folders::bind_reads_pieces(
        &folder_plan,
        &checked.filters,
        &checked.assembly.rules,
        &pieces,
        &catalog,
    );
    let filter_answers = sunstack_rules::ask_filters(
        &checked.filters,
        &folders,
        sunstack_rules::FilterEvidence {
            pieces: &catalog.pieces,
            kinds: &config.evidence.kinds,
            project: &checked.project,
            matches: &catalog.matches,
        },
        report.limits.evidence_max_chars as usize,
        &mut dispatcher,
    )
    .await;
    report.filter_answers = filter_answers
        .entries
        .iter()
        .cloned()
        .map(|value| IdentifiedResult {
            id: 0,
            request_ids: vec![],
            value,
        })
        .collect();
    let run = run_rulebook_with_dispatcher(
        &checked.assembly.rules,
        &pieces,
        &catalog.matches,
        &mut dispatcher,
        &filter_answers,
    )
    .await;
    crate::lint_reporting::saved_reports::capture_requests(
        &mut report,
        &run,
        &filter_answers,
        &checked.assembly.rules,
        &pieces,
        &dispatcher.settings.backend,
    );
    let prepared_requests = run.prepared.clone();
    let mut report = crate::lint_reporting::outcome::project_outcome(
        run,
        &checked.assembly.rules,
        &pieces,
        report,
        request.max_warnings,
        cancellation.is_cancelled(),
    );
    for answer in &report.filter_answers {
        if let Some(piece) = filter_answers
            .pieces
            .get(&(answer.filter.clone(), answer.folder.clone()))
        {
            report.piece_evidence.insert(
                answer.id,
                SavedPieceEvidence {
                    piece: piece.clone(),
                    matches: Default::default(),
                },
            );
        }
    }
    if !preparing
        && let Err(error) = crate::lint_reporting::saved_reports::capture_selection_requests(
            &mut report,
            &dispatcher,
        )
    {
        return failed(report, &error);
    }
    if preparing {
        if let Err(error) = crate::lint_reporting::saved_reports::capture_prepared_requests(
            &mut report,
            &dispatcher,
            &prepared_requests,
            &filter_answers,
            &checked.assembly.rules,
            &pieces,
        ) {
            return failed(report, &error);
        }
        if matches!(
            report.status,
            LintStatus::Clean | LintStatus::NothingSelected | LintStatus::Uncertain
        ) {
            report.status = LintStatus::Prepared;
        }
    }
    for saved in report.piece_evidence.values_mut() {
        if let Some(index) = catalog.matches.get(&saved.piece.location.file) {
            saved.matches = index.by_rule.clone();
        }
    }
    for result in &mut report.results {
        result.text_sha256 = catalog
            .piece_text_hashes
            .get(&(result.file.clone(), result.first_line, result.last_line))
            .cloned();
    }
    report.usage = crate::lint_reporting::outcome::selection_usage(&dispatcher, acquired);
    report
}

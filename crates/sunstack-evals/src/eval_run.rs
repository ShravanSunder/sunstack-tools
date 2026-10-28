use crate::*;
use sunstack_rules::{
    CandidateRunInput, CandidateRunOutcome, ContextSettings, RuleExecutionContext, SourceEntry,
};

pub fn decode_eval_run(bytes: &[u8]) -> Result<EvalRunInput, sunstack_rules::DiagnosticList> {
    crate::frozen_decoding::decode_with_forbidden(
        bytes,
        &["definitions", "definition_root", "payload_schemas"],
    )
    .map_err(crate::eval_run_diagnostics::diagnostics)
}
fn invalid(error: EvalError) -> EvalRunOutcome {
    EvalRunOutcome::Invalid {
        diagnostics: crate::eval_run_diagnostics::diagnostics(error),
    }
}
pub async fn run_eval(input: EvalRunInput, context: RuleExecutionContext) -> EvalRunOutcome {
    let staged = match crate::case_store::verify_for_replay(&input.case_set) {
        Ok(staged) => staged,
        Err(error) => return invalid(error),
    };
    let cancellation = context.cancellation.clone();
    let run = CandidateRunInput {
        sources: staged
            .sources
            .iter()
            .map(|source| SourceEntry {
                source: source.source.clone(),
                evidence: source.evidence.clone(),
            })
            .collect(),
        definitions: staged.manifest.entry.clone(),
        definition_root: staged.definition_home.clone(),
        payload_schemas: crate::payload_snapshot::schema_paths(&staged.manifest.payload_schemas),
        evidence: input.evidence,
        recipes: input.recipes,
        repositories: input.repositories,
        discovery: input.discovery,
        limits: input.limits,
        backend: input.backend,
        attempts: input.attempts,
        request_budget: input.request_budget,
        execution: input.execution,
        samples_per_pass: 20,
        context_settings: ContextSettings::default(),
    };
    let outcome = sunstack_rules::evaluate_candidates(run, context).await;
    let mut cancelled = cancellation.is_cancelled();
    let mut incomplete = false;
    let bundles = match outcome {
        CandidateRunOutcome::Invalid { diagnostics } if !cancelled => {
            return EvalRunOutcome::Invalid { diagnostics };
        }
        CandidateRunOutcome::Invalid { .. } => vec![],
        CandidateRunOutcome::Cancelled { report } => {
            cancelled = true;
            vec![report]
        }
        CandidateRunOutcome::Incomplete { report } => {
            incomplete = true;
            vec![report]
        }
        CandidateRunOutcome::CandidateReport { report } => {
            vec![report]
        }
    };
    let frozen = match staged.finish() {
        Ok(frozen) => frozen,
        Err(error) => return invalid(error),
    };
    let report = match score(&bundles, &frozen) {
        Ok(report) => report,
        Err(error) => return invalid(error),
    };
    if cancelled {
        EvalRunOutcome::Cancelled { report }
    } else if incomplete {
        EvalRunOutcome::Incomplete { report }
    } else {
        EvalRunOutcome::EvalReport { report }
    }
}

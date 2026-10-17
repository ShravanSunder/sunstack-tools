use crate::{
    input_binding::resolved_path,
    invocation_admission::{load_invocation, mapped_declarations},
    *,
};
use std::{collections::BTreeSet, path::Path};
use sunstack_evidence::{AdapterRegistry, EvidencePlanInput, EvidenceSession};
use tokio_util::sync::CancellationToken;

pub fn decode_check_rules(bytes: &[u8]) -> Result<CheckRulesInput, DiagnosticList> {
    crate::strict_decoding::decode_definition(bytes, Path::new("<invocation>"), 1)
}

fn refused(diagnostics: DiagnosticList) -> CheckRulesOutcome {
    CheckRulesOutcome::Refused {
        diagnostics,
        checked_rule_count: 0,
        refused_rule_count: 0,
    }
}
fn cancelled() -> CheckRulesOutcome {
    CheckRulesOutcome::Cancelled {
        diagnostics: DiagnosticList::rule(RuleDiagnosticCode::CallerAborted),
        measurements: InvocationMeasurements::default(),
    }
}

pub async fn check_rules(
    input: CheckRulesInput,
    cancellation: CancellationToken,
) -> CheckRulesOutcome {
    let cancellation = cancellation.child_token();
    let _cancellation_guard = cancellation.clone().drop_guard();
    if cancellation.is_cancelled() {
        return cancelled();
    }
    if input.definition_root.as_os_str().is_empty() {
        return refused(DiagnosticList::rule(
            RuleDiagnosticCode::InvalidConfiguration,
        ));
    }
    let owned = input.clone();
    let loading_cancellation = cancellation.clone();
    let loaded = tokio::task::spawn_blocking(move || {
        load_invocation(
            &owned.definition_root,
            &owned.definitions,
            &owned.repositories,
            owned.limits.max_file_bytes.get(),
            loading_cancellation,
        )
    })
    .await;
    if cancellation.is_cancelled() {
        return cancelled();
    }
    let (groups, router, sources) = match loaded {
        Ok(Ok(loaded)) => loaded,
        Ok(Err(diagnostics)) => return refused(diagnostics),
        Err(_) => {
            return CheckRulesOutcome::CheckFailed {
                diagnostics: DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
                measurements: InvocationMeasurements::default(),
            };
        }
    };
    let admitted = async {
        let registry = AdapterRegistry::builtins()
            .map_err(|error| DiagnosticList::from(error.diagnostics()))?;
        let cwd = std::env::current_dir()
            .map_err(|_| DiagnosticList::rule(RuleDiagnosticCode::InvalidConfiguration))?;
        let root = resolved_path(&cwd, &input.definition_root)?;
        let mut schemas = input.payload_schemas.clone();
        for schema in &mut schemas {
            schema.path = resolved_path(&root, &schema.path)?;
        }
        let plan = EvidencePlanInput {
            repositories: input.repositories.clone(),
            sources,
            payload_schemas: schemas,
            cursors: vec![],
            declarations: mapped_declarations(&groups, router.as_ref())?,
            native_queries: vec![],
            package_bindings: vec![],
            limits: input.limits.clone(),
            discovery: sunstack_evidence::EvidenceDiscovery::None {},
        };
        // Opening admits schemas/bindings. No select/compare call can read a reviewed document.
        EvidenceSession::open(plan, registry, cancellation.clone())
            .await
            .map_err(|error| DiagnosticList::from(error.diagnostics()))
    }
    .await;
    if cancellation.is_cancelled() {
        return cancelled();
    }
    if let Err(diagnostics) = admitted {
        return refused(diagnostics);
    }
    let mut seen = BTreeSet::new();
    let mut printed = Vec::new();
    let mut diagnostics = DiagnosticList::default();
    let mut refused_rule_count = 0;
    for group in &groups {
        if !seen.insert(group.set.path.clone()) {
            continue;
        }
        for skipped in &group.set.skipped {
            refused_rule_count += 1;
            diagnostics.extend(skipped.diagnostics().clone());
        }
        for located in &group.set.rules {
            let source = match &located.provenance {
                Some(provenance) => PrintedRuleSource::Markdown {
                    provenance: provenance.clone(),
                },
                None => PrintedRuleSource::Jsonl {
                    file: located.source.clone(),
                    line: located.line,
                },
            };
            printed.push(PrintedRule {
                rule_set: group.set.path.clone(),
                source,
                definition: (*located.definition).clone(),
            });
        }
    }
    if !diagnostics.is_empty() {
        return CheckRulesOutcome::Refused {
            diagnostics,
            checked_rule_count: printed.len(),
            refused_rule_count,
        };
    }
    CheckRulesOutcome::Checked {
        checked_rule_count: printed.len(),
        printed_rules: match input.output {
            CheckRulesOutput::AssembledRules => printed,
            CheckRulesOutput::Diagnostics => vec![],
        },
    }
}

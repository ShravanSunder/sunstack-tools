use crate::*;
use std::{collections::BTreeSet, path::Path};
use tokio_util::sync::CancellationToken;

pub fn decode_check_rules(bytes: &[u8]) -> Result<CheckDefinitionsInput, DiagnosticList> {
    let decoded = crate::strict_decoding::decode_definition(bytes, Path::new("<invocation>"), 1);
    if let Err(diagnostics) = &decoded
        && !diagnostics.items.iter().any(|item| matches!(item, ReportDiagnostic::Rule(item) if item.code == RuleDiagnosticCode::DuplicateField))
        && let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes)
        && let Some(field) = retired_check_field(&value)
    {
        return Err(RuleDiagnostic::new(RuleDiagnosticCode::UnknownField)
            .definition(Path::new("<invocation>"), 1, 1, field).into());
    }
    decoded
}
fn retired_check_field(value: &serde_json::Value) -> Option<Vec<String>> {
    for name in ["repositories", "discovery"] {
        if value.get(name).is_some() {
            return Some(vec![name.into()]);
        }
    }
    let definitions = value.get("definitions")?;
    if definitions.get("binding_groups").is_some() {
        return Some(vec!["definitions".into(), "binding_groups".into()]);
    }
    for (position, set) in definitions.get("rule_sets")?.as_array()?.iter().enumerate() {
        if set.get("binding_groups").is_some() {
            return Some(vec![
                "definitions".into(),
                "rule_sets".into(),
                position.to_string(),
                "binding_groups".into(),
            ]);
        }
    }
    None
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
    input: CheckDefinitionsInput,
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
        crate::definition_checking::load_checked_definitions(&owned, loading_cancellation)
    })
    .await;
    if cancellation.is_cancelled() {
        return cancelled();
    }
    let sets = match loaded {
        Ok(Ok(sets)) => sets,
        Ok(Err(diagnostics)) => return refused(diagnostics),
        Err(_) => {
            return CheckRulesOutcome::CheckFailed {
                diagnostics: DiagnosticList::rule(RuleDiagnosticCode::UnexpectedExecutionError),
                measurements: InvocationMeasurements::default(),
            };
        }
    };
    let mut seen = BTreeSet::new();
    let mut printed = Vec::new();
    let mut diagnostics = DiagnosticList::default();
    let mut refused_rule_count = 0;
    for set in &sets {
        if !seen.insert(set.path.clone()) {
            continue;
        }
        for skipped in &set.skipped {
            refused_rule_count += 1;
            diagnostics.extend(skipped.diagnostics().clone());
        }
        for located in &set.rules {
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
                rule_set: set.path.clone(),
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

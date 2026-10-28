use crate::EvalError;
use std::path::Path;
use sunstack_rules::{DiagnosticList, RuleDiagnostic, RuleDiagnosticCode};
pub(crate) fn diagnostics(error: EvalError) -> DiagnosticList {
    match error {
        EvalError::Definitions { diagnostics } => diagnostics,
        EvalError::ForbiddenField { field } => {
            RuleDiagnostic::new(RuleDiagnosticCode::UnknownField)
                .definition(Path::new("<score_eval>"), 1, 1, vec![field])
                .into()
        }
        error => {
            let mut diagnostic = RuleDiagnostic::new(RuleDiagnosticCode::InvalidConfiguration);
            match error {
                EvalError::Invalid { field } => {
                    diagnostic = diagnostic.definition(
                        Path::new("<case_set>"),
                        1,
                        1,
                        field.split('.').map(str::to_owned).collect(),
                    );
                    diagnostic.guidance = "Repair the eval request or frozen case records.".into();
                }
                EvalError::File { path } => {
                    diagnostic = diagnostic.definition(&path, 1, 1, vec!["case_set".into()]);
                    diagnostic.guidance =
                        "Restore the named frozen case file before replaying.".into();
                }
                EvalError::Integrity { path } => {
                    diagnostic =
                        diagnostic.definition(&path, 1, 1, vec!["content_identity".into()]);
                    diagnostic.guidance =
                        "The frozen file no longer matches its recorded content identity.".into();
                }
                EvalError::SplitLeakage { field } => {
                    diagnostic = diagnostic.definition(
                        Path::new("<case_set>"),
                        1,
                        1,
                        vec!["labels".into(), field],
                    );
                    diagnostic.guidance = "Keep each source group and project in one split.".into();
                }
                EvalError::WrongDefinitions { rule, .. } => {
                    diagnostic =
                        diagnostic.definition(&rule.rule_set, 1, 1, vec!["definitions".into()]);
                    diagnostic.guidance =
                        "Replay the frozen definitions owned by this case set.".into();
                }
                EvalError::Definitions { .. } | EvalError::ForbiddenField { .. } => {}
            }
            diagnostic.into()
        }
    }
}

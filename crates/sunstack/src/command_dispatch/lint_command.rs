use crate::{
    command_dispatch::command_line::LintArguments,
    process_runtime::process_signals::{ProcessSignals, operational_message},
};
use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};
use sunstack_decisions::CancellationToken;
use sunstack_lint::{CheckResult, LintDiagnostic, LintInvalid, LintRequest, ScopeChoice};

fn output(text: &str, exit: u8) -> ExitCode {
    let mut stdout = io::stdout().lock();
    if stdout
        .write_all(text.as_bytes())
        .and_then(|_| stdout.write_all(b"\n"))
        .and_then(|_| stdout.flush())
        .is_err()
    {
        operational_message(b"sunstack: output unavailable\n");
        return ExitCode::from(3);
    }
    ExitCode::from(exit)
}
fn diagnostics(diagnostics: &[LintDiagnostic]) -> ExitCode {
    let mut stderr = io::stderr().lock();
    for diagnostic in diagnostics {
        let _ = writeln!(
            stderr,
            "{}:{}:{}: {}",
            diagnostic.file.as_ref().map_or_else(
                || "sunstack.lint.json".into(),
                |file| file.display().to_string()
            ),
            diagnostic.line.unwrap_or(1),
            diagnostic.column.unwrap_or(1),
            diagnostic.message
        );
    }
    ExitCode::from(2)
}
fn invalid(error: LintInvalid) -> ExitCode {
    diagnostics(&[error.into()])
}
fn json_output(value: &impl serde::Serialize) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(text) => output(&text, 0),
        Err(_) => {
            operational_message(b"sunstack: output encoding failed\n");
            ExitCode::from(3)
        }
    }
}
async fn inspect(arguments: &LintArguments, cancellation: CancellationToken) -> ExitCode {
    let config_path = arguments.config.clone();
    let loaded = tokio::task::spawn_blocking(move || {
        let cwd = std::env::current_dir().map_err(|_| {
            LintInvalid::at(
                "sunstack.lint.json",
                sunstack_lint::LintInvalidCode::NoConfigFound,
                "Cannot resolve the current directory.",
            )
        })?;
        let location = sunstack_lint::discover_config(&cwd, config_path.as_deref())?;
        let config = sunstack_lint::load_config(&location)?;
        Ok::<_, LintInvalid>((cwd, location, config))
    })
    .await;
    let (cwd, location, config) = match loaded {
        Ok(Ok(loaded)) => loaded,
        Ok(Err(error)) => return invalid(error),
        Err(_) => {
            operational_message(b"sunstack: config loading failed\n");
            return ExitCode::from(3);
        }
    };
    let checked = sunstack_lint::check(&config, cancellation.clone()).await;
    if cancellation.is_cancelled() {
        return output("Cancelled", 4);
    }
    match &checked {
        CheckResult::Invalid {
            diagnostics: errors,
        } => diagnostics(errors),
        CheckResult::Valid { rules } => {
            if let Some(file) = &arguments.rules {
                let relative = file.as_ref().map(|file| {
                    sunstack_lint::lint_execution::targets::relative_path(
                        &location.directory,
                        &cwd.join(file),
                    )
                });
                json_output(&sunstack_lint::list_rules(
                    &config,
                    &checked,
                    relative.as_deref(),
                ))
            } else {
                json_output(rules)
            }
        }
    }
}
async fn execute(arguments: LintArguments, cancellation: CancellationToken) -> ExitCode {
    if arguments.init {
        return match tokio::task::spawn_blocking(|| {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            sunstack_lint::init_config(&cwd)
        })
        .await
        {
            Ok(Ok(location)) => output(
                &format!(
                    "Created {}. Add the rulebook, rule entries and evidence project before running lint.",
                    location.file.display()
                ),
                0,
            ),
            Ok(Err(error)) => invalid(error),
            Err(_) => {
                operational_message(b"sunstack: config creation failed\n");
                ExitCode::from(3)
            }
        };
    }
    if arguments.check || arguments.rules.is_some() {
        return inspect(&arguments, cancellation).await;
    }
    let scope = match arguments.since {
        Some(rev) => ScopeChoice::Since { rev },
        None if arguments.all => ScopeChoice::All,
        None => ScopeChoice::Default,
    };
    let evidence_listing = match arguments.evidence {
        Some(Some(kind)) => match sunstack_evidence::EvidenceKindName::new(kind) {
            Ok(kind) => Some(Some(kind)),
            Err(_) => {
                return invalid(LintInvalid::at(
                    "sunstack.lint.json",
                    sunstack_lint::LintInvalidCode::Syntax,
                    "invalid evidence kind",
                ));
            }
        },
        Some(None) => Some(None),
        None => None,
    };
    let credential = crate::process_runtime::provider_credentials::resolve().await;
    let report = sunstack_lint::run(
        LintRequest {
            config_path: arguments.config,
            paths: arguments.paths,
            scope,
            max_warnings: arguments.max_warnings,
            evidence_listing,
        },
        cancellation,
        credential,
    )
    .await;
    if let Some(listing) = &report.evidence_listing {
        return match serde_json::to_string_pretty(listing) {
            Ok(text) => output(&text, report.status.exit_code()),
            Err(_) => output("Unable to encode evidence listing.", 3),
        };
    }
    output(
        sunstack_lint::render(&report, arguments.format).trim_end_matches('\n'),
        report.status.exit_code(),
    )
}
pub async fn run(arguments: LintArguments) -> ExitCode {
    let mut signals = match ProcessSignals::install() {
        Ok(signals) => signals,
        Err(_) => {
            operational_message(b"sunstack: signal setup failed\n");
            return ExitCode::from(3);
        }
    };
    let cancellation = CancellationToken::new();
    signals
        .drive(execute(arguments, cancellation.clone()), &cancellation)
        .await
}

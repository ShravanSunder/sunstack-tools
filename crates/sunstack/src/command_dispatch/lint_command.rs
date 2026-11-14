use crate::{
    command_dispatch::command_line::{LintArguments, LintCommand},
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
    if arguments.root.is_some() && arguments.command.is_some() {
        return output(
            "--root applies to ordinary lint; lint cases owns its historical snapshot root and lint show reads a saved report.",
            2,
        );
    }
    if let Some(LintCommand::Show { file, id }) = &arguments.command {
        let detail = std::fs::read(file)
            .map_err(|error| format!("cannot read saved report {}: {error}", file.display()))
            .and_then(|bytes| {
                sunstack_lint::lint_reporting::saved_reports::show_saved_report(&bytes, id)
            });
        return match detail {
            Ok(detail) => json_output(&detail),
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "sunstack: {error}");
                ExitCode::from(2)
            }
        };
    }
    if let Some(LintCommand::Cases { file, set }) = &arguments.command {
        let executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(_) => return output("Cannot locate sunstack executable.", 3),
        };
        let result = if arguments.prepare.is_some() {
            sunstack_lint::case_runs::prepare_cases(
                file,
                arguments.config.as_deref(),
                set.as_deref(),
                &executable,
                cancellation,
                arguments.selections_from.as_deref(),
            )
            .await
        } else {
            sunstack_lint::case_runs::run_cases(
                file,
                arguments.config.as_deref(),
                set.as_deref(),
                &executable,
                cancellation,
            )
            .await
        };
        return match result {
            Ok(report) => match serde_json::to_string_pretty(&report) {
                Ok(text) => {
                    if let Some(path) = arguments.prepare.as_ref().or(arguments.report.as_ref())
                        && let Err(error) = std::fs::write(path, &text)
                    {
                        let _ =
                            writeln!(io::stderr().lock(), "sunstack: cannot save report: {error}");
                        return ExitCode::from(3);
                    }
                    output(&text, report.exit_code())
                }
                Err(_) => output("Unable to encode case report.", 3),
            },
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "sunstack: {error}");
                ExitCode::from(2)
            }
        };
    }
    if arguments.init {
        return match tokio::task::spawn_blocking(|| {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            sunstack_lint::init_config(&cwd)
        })
        .await
        {
            Ok(Ok(location)) => output(
                &format!(
                    "Created {} and sunstack-starter/ with a runnable mechanical check. Run sunstack lint --root . --check, then sunstack lint --root . --all.",
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
    let request = LintRequest {
        config_path: arguments.config,
        root: arguments.root,
        paths: arguments.paths,
        scope,
        max_warnings: arguments.max_warnings,
        evidence_listing,
    };
    let report = if arguments.prepare.is_some() {
        let selections = match &arguments.selections_from {
            Some(path) => {
                match sunstack_lint::lint_reporting::selection_sources::read_selection_records(path)
                {
                    Ok(records) => records,
                    Err(error) => {
                        return output(&format!("Cannot read selection report: {error}"), 2);
                    }
                }
            }
            None => vec![],
        };
        sunstack_lint::prepare(request, cancellation, selections).await
    } else {
        let credential = crate::process_runtime::provider_credentials::resolve().await;
        sunstack_lint::run(request, cancellation, credential).await
    };
    if let Some(path) = arguments.prepare.as_ref().or(arguments.report.as_ref()) {
        let saved = serde_json::to_vec_pretty(&report)
            .map_err(|error| error.to_string())
            .and_then(|bytes| std::fs::write(path, bytes).map_err(|error| error.to_string()));
        if let Err(error) = saved {
            let _ = writeln!(io::stderr().lock(), "sunstack: cannot save report: {error}");
            return ExitCode::from(3);
        }
    }
    if let Some(listing) = &report.evidence_listing {
        return match serde_json::to_string_pretty(listing) {
            Ok(text) => output(&text, report.status.exit_code()),
            Err(_) => output("Unable to encode evidence listing.", 3),
        };
    }
    let rendered = match &arguments.prepare {
        Some(path) => sunstack_lint::render_prepared(&report, arguments.format, path),
        None => sunstack_lint::render_with_warning_limit(
            &report,
            arguments.format,
            arguments.max_warnings,
        ),
    };
    output(rendered.trim_end_matches('\n'), report.status.exit_code())
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

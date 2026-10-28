mod command_line;
mod exec_protocol;
mod lint_command;
mod operation_execution;
mod process_signals;
mod protocol_boundary;
mod provider_credentials;

use clap::Parser;
use command_line::{CommandLine, CredentialCommand, RunnerCommand};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match CommandLine::parse().command {
        RunnerCommand::Exec => exec_protocol::run().await,
        RunnerCommand::Lint(arguments) => lint_command::run(arguments).await,
        RunnerCommand::Credentials {
            command: CredentialCommand::Save,
        } => provider_credentials::save_command().await,
    }
}

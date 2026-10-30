#[path = "command_dispatch/command_line.rs"]
mod command_line;
#[path = "operation_protocol/exec_protocol.rs"]
mod exec_protocol;
#[path = "command_dispatch/lint_command.rs"]
mod lint_command;
#[path = "operation_protocol/operation_execution.rs"]
mod operation_execution;
#[path = "process_runtime/process_signals.rs"]
mod process_signals;
#[path = "operation_protocol/protocol_boundary.rs"]
mod protocol_boundary;
#[path = "process_runtime/provider_credentials.rs"]
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

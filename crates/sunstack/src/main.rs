mod command_dispatch;
mod operation_protocol;
mod process_runtime;

use clap::Parser;
use command_dispatch::command_line::{CommandLine, CredentialCommand, RunnerCommand};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match CommandLine::parse().command {
        RunnerCommand::Exec => operation_protocol::exec_protocol::run().await,
        RunnerCommand::Lint(arguments) => command_dispatch::lint_command::run(arguments).await,
        RunnerCommand::Credentials {
            command: CredentialCommand::Save,
        } => process_runtime::provider_credentials::save_command().await,
    }
}

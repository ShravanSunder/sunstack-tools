mod command_line;
mod exec_protocol;
mod lint_command;
mod operation_execution;
mod process_signals;
mod protocol_boundary;

use clap::Parser;
use command_line::{CommandLine, RunnerCommand};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match CommandLine::parse().command {
        RunnerCommand::Exec => exec_protocol::run().await,
        RunnerCommand::Lint(arguments) => lint_command::run(arguments).await,
    }
}

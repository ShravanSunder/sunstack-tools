mod command_line;
mod exec_protocol;
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
        RunnerCommand::Lint => {
            process_signals::operational_message(
                b"sunstack: lint is not available in this build\n",
            );
            ExitCode::from(2)
        }
    }
}

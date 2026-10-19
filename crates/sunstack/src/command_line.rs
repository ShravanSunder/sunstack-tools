use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sunstack",
    version,
    about = "Run Sunstack repository checks and native operations"
)]
pub struct CommandLine {
    #[command(subcommand)]
    pub command: RunnerCommand,
}

#[derive(Subcommand)]
pub enum RunnerCommand {
    /// Lint a repository using sunstack.lint.json.
    Lint,
    /// Execute one native JSON request from standard input.
    Exec,
}

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use sunstack_lint::OutputFormat;

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
    Lint(LintArguments),
    /// Execute one native JSON request from standard input.
    Exec,
    /// Manage the OpenRouter credential in the OS credential store.
    Credentials {
        #[command(subcommand)]
        command: CredentialCommand,
    },
}

#[derive(Subcommand)]
pub enum CredentialCommand {
    /// Save OPENROUTER_API_KEY from the environment to the OS credential store.
    Save,
}

#[derive(Args)]
pub struct LintArguments {
    /// Files or directories to check; named paths default to full scope.
    #[arg(conflicts_with_all = ["check", "rules", "init"])]
    pub paths: Vec<PathBuf>,
    #[arg(long, conflicts_with = "init")]
    pub config: Option<PathBuf>,
    /// Check all configured targets.
    #[arg(long, conflicts_with_all = ["since", "check", "rules", "init"])]
    pub all: bool,
    /// Check changes since a Git revision, including worktree edits.
    #[arg(long, conflicts_with_all = ["check", "rules", "init"])]
    pub since: Option<String>,
    #[arg(long, default_value = "default", value_name = "default|json|github|sarif", conflicts_with_all = ["check", "rules", "init"])]
    pub format: OutputFormat,
    /// Fail when warning findings exceed this number.
    #[arg(long, conflicts_with_all = ["check", "rules", "init"])]
    pub max_warnings: Option<u32>,
    /// Print picked evidence as JSON, optionally for one kind, without asking the model.
    #[arg(long, num_args = 0..=1, value_name = "KIND", conflicts_with_all = ["check", "rules", "init"])]
    pub evidence: Option<Option<String>>,
    /// Validate configuration, rulebook and evidence project without reading checked files.
    #[arg(long, conflicts_with_all = ["rules", "init"])]
    pub check: bool,
    /// List rules as JSON; optionally name one file.
    #[arg(long, num_args = 0..=1, value_name = "FILE", conflicts_with = "init")]
    pub rules: Option<Option<PathBuf>>,
    /// Create a config skeleton in the current directory.
    #[arg(long)]
    pub init: bool,
}

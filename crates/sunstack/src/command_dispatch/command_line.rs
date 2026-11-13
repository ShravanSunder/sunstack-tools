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
    Lint(Box<LintArguments>),
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
#[command(
    after_help = "Choose an output:\n  --check          Admit definitions; no checked files or model requests\n  --evidence       Picked pieces with text, source spans and coverage; no rule evaluation\n  --prepare FILE   Save requests/evidence without sending model requests (exit 6 when prepared)\n  --format json    Run rules and print their report\n  --report FILE    Run rules and save the full report for lint show\n\nEvidence clean means listing completion, not absence of violations.\nSelected evidence kinds may need a model credential.\n\nAuthoring: --init writes only a config skeleton, never referenced definitions.\nComplete rulebook, marker-keyed entries and matcher example:\n  https://github.com/ShravanSunder/sunstack-tools/blob/main/crates/sunstack/README.md#complete-keyless-example\n\nExit codes:\n  0  Clean or nothing selected\n  1  Findings\n  2  Invalid input\n  3  Incomplete\n  4  Cancelled\n  5  Uncertain\n  6  Prepared"
)]
pub struct LintArguments {
    #[command(subcommand)]
    pub command: Option<LintCommand>,
    /// Save complete requests, model receipts and rulebook files for later inspection.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["check", "rules", "init", "evidence", "prepare"])]
    pub report: Option<PathBuf>,
    /// Save requests and evidence without a credential or model dispatch.
    #[arg(long, global = true, value_name = "FILE")]
    pub prepare: Option<PathBuf>,
    /// Reuse matching recorded selection answers while preparing.
    #[arg(long, global = true, value_name = "REPORT", requires = "prepare")]
    pub selections_from: Option<PathBuf>,

    /// Files or directories to check; named paths default to full scope.
    #[arg(conflicts_with_all = ["check", "rules", "init"])]
    pub paths: Vec<PathBuf>,
    /// Config file to use; default: nearest sunstack.lint.json upward.
    #[arg(long, conflicts_with = "init")]
    pub config: Option<PathBuf>,
    /// Resolve checked file globs and evidence paths within this directory.
    #[arg(long, value_name = "DIR", conflicts_with = "init")]
    pub root: Option<PathBuf>,
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
    /// Print picked pieces, source spans and coverage as JSON; no rule evaluation.
    #[arg(long, num_args = 0..=1, value_name = "KIND", conflicts_with_all = ["check", "rules", "init", "prepare"])]
    pub evidence: Option<Option<String>>,
    /// Validate configuration, rulebook and evidence project without reading checked files.
    #[arg(long, conflicts_with_all = ["rules", "init", "prepare"])]
    pub check: bool,
    /// List rules as JSON; optionally name one file.
    #[arg(long, num_args = 0..=1, value_name = "FILE", conflicts_with_all = ["init", "prepare"])]
    pub rules: Option<Option<PathBuf>>,
    /// Create only sunstack.lint.json; author its referenced rulebook, entries and matchers.
    #[arg(long, conflicts_with = "prepare")]
    pub init: bool,
}

#[derive(Subcommand)]
pub enum LintCommand {
    /// Score revision-bound cases using today's rulebook through ordinary lint.
    Cases {
        file: PathBuf,
        #[arg(long)]
        set: Option<String>,
    },
    /// Explain a result ID or case ID using only a saved report.
    Show { file: PathBuf, id: String },
}

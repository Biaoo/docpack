use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "ai-doc-lint",
    version,
    about = "Diff-driven documentation drift gate for AI-assisted teams."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Check(CheckArgs),
    Explain(ExplainArgs),
    ValidateConfig(ValidateConfigArgs),
}

#[derive(Debug, Clone, Args)]
pub struct CheckArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub base: Option<String>,
    #[arg(long)]
    pub head: Option<String>,
    #[arg(long)]
    pub files: Option<String>,
    #[arg(long, default_value_t = false)]
    pub staged: bool,
    #[arg(long, default_value_t = false)]
    pub worktree: bool,
    #[arg(long = "merge-base")]
    pub merge_base: Option<String>,
    #[arg(long, value_enum, default_value_t = LintMode::Warn)]
    pub mode: LintMode,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ExplainArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct ValidateConfigArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LintMode {
    Warn,
    Enforce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Commands, LintMode, OutputFormat};

    #[test]
    fn parses_check_command() {
        let cli = Cli::try_parse_from([
            "ai-doc-lint",
            "check",
            "--base",
            "abc123",
            "--head",
            "def456",
            "--mode",
            "enforce",
            "--format",
            "json",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Check(args) => {
                assert_eq!(args.base.as_deref(), Some("abc123"));
                assert_eq!(args.head.as_deref(), Some("def456"));
                assert_eq!(args.mode, LintMode::Enforce);
                assert_eq!(args.format, OutputFormat::Json);
            }
            _ => panic!("expected check command"),
        }
    }
}

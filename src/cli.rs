use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "docpact",
    version,
    about = "Diff-driven documentation drift gate for AI-assisted teams."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Lint(LintArgs),
    Diagnostics(DiagnosticsArgs),
    Explain(ExplainArgs),
    ValidateConfig(ValidateConfigArgs),
}

#[derive(Debug, Clone, Args)]
pub struct LintArgs {
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
    #[arg(long, value_enum, default_value_t = DiagnosticDetail::Compact)]
    pub detail: DiagnosticDetail,
    #[arg(long, default_value_t = 1, value_parser = parse_positive_usize)]
    pub diagnostics_page: usize,
    #[arg(long, default_value_t = 5, value_parser = parse_positive_usize)]
    pub diagnostics_page_size: usize,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct DiagnosticsArgs {
    #[command(subcommand)]
    pub command: DiagnosticsCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DiagnosticsCommands {
    Show(DiagnosticsShowArgs),
}

#[derive(Debug, Clone, Args)]
pub struct DiagnosticsShowArgs {
    #[arg(long)]
    pub report: PathBuf,
    #[arg(long)]
    pub id: String,
    #[arg(long, value_enum, default_value_t = DiagnosticsOutputFormat::Text)]
    pub format: DiagnosticsOutputFormat,
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
    #[arg(long, default_value_t = false)]
    pub strict: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagnosticsOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagnosticDetail {
    Summary,
    Compact,
    Full,
}

impl DiagnosticDetail {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Compact => "compact",
            Self::Full => "full",
        }
    }
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(0) => Err("value must be greater than 0".into()),
        Ok(parsed) => Ok(parsed),
        Err(_) => Err(format!("invalid positive integer: {value}")),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{
        Cli, Commands, DiagnosticDetail, DiagnosticsCommands, DiagnosticsOutputFormat, LintMode,
        OutputFormat,
    };

    #[test]
    fn parses_lint_command() {
        let cli = Cli::try_parse_from([
            "docpact",
            "lint",
            "--base",
            "abc123",
            "--head",
            "def456",
            "--mode",
            "enforce",
            "--format",
            "json",
            "--detail",
            "full",
            "--diagnostics-page",
            "2",
            "--diagnostics-page-size",
            "9",
            "--output",
            ".docpact/runs/latest.json",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Lint(args) => {
                assert_eq!(args.base.as_deref(), Some("abc123"));
                assert_eq!(args.head.as_deref(), Some("def456"));
                assert_eq!(args.mode, LintMode::Enforce);
                assert_eq!(args.format, OutputFormat::Json);
                assert_eq!(args.detail, DiagnosticDetail::Full);
                assert_eq!(args.diagnostics_page, 2);
                assert_eq!(args.diagnostics_page_size, 9);
                assert_eq!(
                    args.output.as_deref(),
                    Some(std::path::Path::new(".docpact/runs/latest.json"))
                );
            }
            _ => panic!("expected lint command"),
        }
    }

    #[test]
    fn parses_diagnostics_show_command() {
        let cli = Cli::try_parse_from([
            "docpact",
            "diagnostics",
            "show",
            "--report",
            ".docpact/runs/latest.json",
            "--id",
            "d003",
            "--format",
            "json",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Diagnostics(args) => match args.command {
                DiagnosticsCommands::Show(show) => {
                    assert_eq!(
                        show.report,
                        std::path::PathBuf::from(".docpact/runs/latest.json")
                    );
                    assert_eq!(show.id, "d003");
                    assert_eq!(show.format, DiagnosticsOutputFormat::Json);
                }
            },
            _ => panic!("expected diagnostics command"),
        }
    }

    #[test]
    fn parses_validate_config_strict_flag() {
        let cli = Cli::try_parse_from(["docpact", "validate-config", "--strict"])
            .expect("cli should parse");

        match cli.command {
            Commands::ValidateConfig(args) => {
                assert!(args.strict);
            }
            _ => panic!("expected validate-config command"),
        }
    }
}

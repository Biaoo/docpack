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
    ListRules(ListRulesArgs),
    Doctor(DoctorArgs),
    Coverage(CoverageArgs),
    Freshness(FreshnessArgs),
    Diagnostics(DiagnosticsArgs),
    Review(ReviewArgs),
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
    #[arg(long, default_value_t = false)]
    pub fail_on_uncovered_change: bool,
    #[arg(long, default_value_t = false)]
    pub fail_on_stale_docs: bool,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct CoverageArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = CoverageOutputFormat::Text)]
    pub format: CoverageOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ListRulesArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = ListRulesOutputFormat::Text)]
    pub format: ListRulesOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct DoctorArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = DoctorOutputFormat::Text)]
    pub format: DoctorOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct FreshnessArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = FreshnessOutputFormat::Text)]
    pub format: FreshnessOutputFormat,
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
pub struct ReviewArgs {
    #[command(subcommand)]
    pub command: ReviewCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ReviewCommands {
    Mark(ReviewMarkArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ReviewMarkArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long = "path")]
    pub paths: Vec<PathBuf>,
    #[arg(long)]
    pub report: Option<PathBuf>,
    #[arg(long)]
    pub id: Option<String>,
    #[arg(long, value_parser = parse_iso_date)]
    pub date: Option<String>,
    #[arg(long)]
    pub commit: Option<String>,
    #[arg(long, value_enum, default_value_t = ReviewOutputFormat::Text)]
    pub format: ReviewOutputFormat,
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
pub enum ListRulesOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DoctorOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CoverageOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FreshnessOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagnosticsOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReviewOutputFormat {
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

fn parse_iso_date(value: &str) -> Result<String, String> {
    if value.len() != 10 {
        return Err(format!("invalid YYYY-MM-DD date: {value}"));
    }

    let bytes = value.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(format!("invalid YYYY-MM-DD date: {value}"));
    }

    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return Err(format!("invalid YYYY-MM-DD date: {value}"));
    }

    let month = value[5..7]
        .parse::<u8>()
        .map_err(|_| format!("invalid YYYY-MM-DD date: {value}"))?;
    let day = value[8..10]
        .parse::<u8>()
        .map_err(|_| format!("invalid YYYY-MM-DD date: {value}"))?;

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(format!("invalid YYYY-MM-DD date: {value}"));
    }

    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{
        Cli, Commands, CoverageOutputFormat, DiagnosticDetail, DiagnosticsCommands,
        DiagnosticsOutputFormat, DoctorOutputFormat, FreshnessOutputFormat, LintMode,
        ListRulesOutputFormat, OutputFormat, ReviewCommands, ReviewOutputFormat,
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
            "--fail-on-uncovered-change",
            "--fail-on-stale-docs",
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
                assert!(args.fail_on_uncovered_change);
                assert!(args.fail_on_stale_docs);
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
    fn parses_coverage_command() {
        let cli = Cli::try_parse_from(["docpact", "coverage", "--root", ".", "--format", "json"])
            .expect("cli should parse");

        match cli.command {
            Commands::Coverage(args) => {
                assert_eq!(args.root.as_deref(), Some(std::path::Path::new(".")));
                assert_eq!(args.format, CoverageOutputFormat::Json);
            }
            _ => panic!("expected coverage command"),
        }
    }

    #[test]
    fn parses_list_rules_command() {
        let cli = Cli::try_parse_from(["docpact", "list-rules", "--root", ".", "--format", "json"])
            .expect("cli should parse");

        match cli.command {
            Commands::ListRules(args) => {
                assert_eq!(args.root.as_deref(), Some(std::path::Path::new(".")));
                assert_eq!(args.format, ListRulesOutputFormat::Json);
            }
            _ => panic!("expected list-rules command"),
        }
    }

    #[test]
    fn parses_doctor_command() {
        let cli = Cli::try_parse_from(["docpact", "doctor", "--root", ".", "--format", "json"])
            .expect("cli should parse");

        match cli.command {
            Commands::Doctor(args) => {
                assert_eq!(args.root.as_deref(), Some(std::path::Path::new(".")));
                assert_eq!(args.format, DoctorOutputFormat::Json);
            }
            _ => panic!("expected doctor command"),
        }
    }

    #[test]
    fn parses_freshness_command() {
        let cli = Cli::try_parse_from(["docpact", "freshness", "--root", ".", "--format", "json"])
            .expect("cli should parse");

        match cli.command {
            Commands::Freshness(args) => {
                assert_eq!(args.root.as_deref(), Some(std::path::Path::new(".")));
                assert_eq!(args.format, FreshnessOutputFormat::Json);
            }
            _ => panic!("expected freshness command"),
        }
    }

    #[test]
    fn parses_review_mark_path_command() {
        let cli = Cli::try_parse_from([
            "docpact",
            "review",
            "mark",
            "--root",
            ".",
            "--path",
            "docs/api.md",
            "--path",
            "AGENTS.md",
            "--date",
            "2026-04-21",
            "--commit",
            "abc123",
            "--format",
            "json",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Review(args) => match args.command {
                ReviewCommands::Mark(mark) => {
                    assert_eq!(mark.paths.len(), 2);
                    assert_eq!(mark.date.as_deref(), Some("2026-04-21"));
                    assert_eq!(mark.commit.as_deref(), Some("abc123"));
                    assert_eq!(mark.format, ReviewOutputFormat::Json);
                }
            },
            _ => panic!("expected review command"),
        }
    }

    #[test]
    fn parses_review_mark_report_command() {
        let cli = Cli::try_parse_from([
            "docpact",
            "review",
            "mark",
            "--report",
            ".docpact/runs/latest.json",
            "--id",
            "d001",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Review(args) => match args.command {
                ReviewCommands::Mark(mark) => {
                    assert_eq!(
                        mark.report,
                        Some(std::path::PathBuf::from(".docpact/runs/latest.json"))
                    );
                    assert_eq!(mark.id.as_deref(), Some("d001"));
                }
            },
            _ => panic!("expected review command"),
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

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
    Baseline(BaselineArgs),
    Waiver(WaiverArgs),
    Route(RouteArgs),
    Render(RenderArgs),
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
    pub baseline: Option<PathBuf>,
    #[arg(long)]
    pub waivers: Option<PathBuf>,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct BaselineArgs {
    #[command(subcommand)]
    pub command: BaselineCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum BaselineCommands {
    Create(BaselineCreateArgs),
}

#[derive(Debug, Clone, Args)]
pub struct BaselineCreateArgs {
    #[arg(long)]
    pub report: PathBuf,
    #[arg(long)]
    pub output: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct WaiverArgs {
    #[command(subcommand)]
    pub command: WaiverCommands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum WaiverCommands {
    Add(WaiverAddArgs),
}

#[derive(Debug, Clone, Args)]
pub struct WaiverAddArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub report: PathBuf,
    #[arg(long)]
    pub id: String,
    #[arg(long)]
    pub reason: String,
    #[arg(long)]
    pub owner: String,
    #[arg(long = "expires-at", value_parser = parse_iso_date)]
    pub expires_at: String,
    #[arg(long = "scope-rule-id")]
    pub scope_rule_ids: Vec<String>,
    #[arg(long = "scope-path")]
    pub scope_paths: Vec<String>,
    #[arg(long)]
    pub waivers: PathBuf,
    #[arg(long, value_enum, default_value_t = WaiverOutputFormat::Text)]
    pub format: WaiverOutputFormat,
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
pub struct RouteArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub paths: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub module: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    pub intent: Vec<String>,
    #[arg(long, value_enum, default_value_t = RouteDetail::Compact)]
    pub detail: RouteDetail,
    #[arg(long, value_parser = parse_positive_usize)]
    pub limit: Option<usize>,
    #[arg(long, value_enum, default_value_t = RouteOutputFormat::Text)]
    pub format: RouteOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct RenderArgs {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long, value_enum)]
    pub view: RenderView,
    #[arg(long)]
    pub paths: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub module: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    pub intent: Vec<String>,
    #[arg(long, value_parser = parse_positive_usize)]
    pub limit: Option<usize>,
    #[arg(long, value_enum, default_value_t = RenderOutputFormat::Text)]
    pub format: RenderOutputFormat,
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
pub enum RouteOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RenderOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RenderView {
    CatalogSummary,
    OwnershipSummary,
    NavigationSummary,
    WorkspaceSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RouteDetail {
    Compact,
    Full,
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
pub enum WaiverOutputFormat {
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
        BaselineCommands, Cli, Commands, CoverageOutputFormat, DiagnosticDetail,
        DiagnosticsCommands, DiagnosticsOutputFormat, DoctorOutputFormat, FreshnessOutputFormat,
        LintMode, ListRulesOutputFormat, OutputFormat, ReviewCommands, ReviewOutputFormat,
        RouteDetail, RouteOutputFormat, WaiverCommands, WaiverOutputFormat,
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
            "--baseline",
            ".docpact/baseline.json",
            "--waivers",
            ".docpact/waivers.yaml",
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
                    args.baseline.as_deref(),
                    Some(std::path::Path::new(".docpact/baseline.json"))
                );
                assert_eq!(
                    args.waivers.as_deref(),
                    Some(std::path::Path::new(".docpact/waivers.yaml"))
                );
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
    fn parses_route_command() {
        let cli = Cli::try_parse_from([
            "docpact",
            "route",
            "--root",
            ".",
            "--config",
            ".docpact/config.yaml",
            "--paths",
            "src/payments/charge.ts,src/payments/refund.ts",
            "--module",
            "src/payments",
            "--intent",
            "payments,auth",
            "--detail",
            "full",
            "--limit",
            "7",
            "--format",
            "json",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Route(args) => {
                assert_eq!(args.root.as_deref(), Some(std::path::Path::new(".")));
                assert_eq!(
                    args.config.as_deref(),
                    Some(std::path::Path::new(".docpact/config.yaml"))
                );
                assert_eq!(
                    args.paths.as_deref(),
                    Some("src/payments/charge.ts,src/payments/refund.ts")
                );
                assert_eq!(args.module, vec!["src/payments"]);
                assert_eq!(args.intent, vec!["payments", "auth"]);
                assert_eq!(args.detail, RouteDetail::Full);
                assert_eq!(args.limit, Some(7));
                assert_eq!(args.format, RouteOutputFormat::Json);
            }
            _ => panic!("expected route command"),
        }
    }

    #[test]
    fn parses_render_command() {
        let cli = Cli::try_parse_from([
            "docpact",
            "render",
            "--root",
            ".",
            "--view",
            "navigation-summary",
            "--paths",
            "src/payments/charge.ts",
            "--module",
            "src/payments",
            "--intent",
            "payments",
            "--limit",
            "5",
            "--format",
            "json",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Render(args) => {
                assert_eq!(args.root.as_deref(), Some(std::path::Path::new(".")));
                assert_eq!(args.view, super::RenderView::NavigationSummary);
                assert_eq!(args.paths.as_deref(), Some("src/payments/charge.ts"));
                assert_eq!(args.module, vec!["src/payments"]);
                assert_eq!(args.intent, vec!["payments"]);
                assert_eq!(args.limit, Some(5));
                assert_eq!(args.format, super::RenderOutputFormat::Json);
            }
            _ => panic!("expected render command"),
        }
    }

    #[test]
    fn parses_baseline_create_command() {
        let cli = Cli::try_parse_from([
            "docpact",
            "baseline",
            "create",
            "--report",
            ".docpact/runs/latest.json",
            "--output",
            ".docpact/baseline.json",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Baseline(args) => match args.command {
                BaselineCommands::Create(create) => {
                    assert_eq!(
                        create.report,
                        std::path::PathBuf::from(".docpact/runs/latest.json")
                    );
                    assert_eq!(
                        create.output,
                        std::path::PathBuf::from(".docpact/baseline.json")
                    );
                }
            },
            _ => panic!("expected baseline command"),
        }
    }

    #[test]
    fn parses_waiver_add_command() {
        let cli = Cli::try_parse_from([
            "docpact",
            "waiver",
            "add",
            "--root",
            ".",
            "--report",
            ".docpact/runs/latest.json",
            "--id",
            "d001",
            "--reason",
            "legacy migration in progress",
            "--owner",
            "docs-team",
            "--expires-at",
            "2026-05-01",
            "--scope-rule-id",
            "api-docs",
            "--scope-path",
            "README.md",
            "--waivers",
            ".docpact/waivers.yaml",
            "--format",
            "json",
        ])
        .expect("cli should parse");

        match cli.command {
            Commands::Waiver(args) => match args.command {
                WaiverCommands::Add(add) => {
                    assert_eq!(add.root.as_deref(), Some(std::path::Path::new(".")));
                    assert_eq!(
                        add.report,
                        std::path::PathBuf::from(".docpact/runs/latest.json")
                    );
                    assert_eq!(add.id, "d001");
                    assert_eq!(add.reason, "legacy migration in progress");
                    assert_eq!(add.owner, "docs-team");
                    assert_eq!(add.expires_at, "2026-05-01");
                    assert_eq!(add.scope_rule_ids, vec!["api-docs"]);
                    assert_eq!(add.scope_paths, vec!["README.md"]);
                    assert_eq!(
                        add.waivers,
                        std::path::PathBuf::from(".docpact/waivers.yaml")
                    );
                    assert_eq!(add.format, WaiverOutputFormat::Json);
                }
            },
            _ => panic!("expected waiver command"),
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

pub mod baseline;
pub mod check;
pub mod cli;
pub mod config;
pub mod coverage;
pub mod diagnostics;
pub mod doctor;
pub mod explain;
pub mod freshness;
pub mod git;
pub mod list_rules;
pub mod metadata;
pub mod render;
pub mod reporters;
pub mod review;
pub mod route;
pub mod rules;
pub mod validate_config;
pub mod waiver;

use miette::Result;

use crate::cli::{Cli, Commands};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppExit {
    Success,
    LintFailure,
}

pub fn run(cli: Cli) -> Result<AppExit> {
    match cli.command {
        Commands::Lint(args) => check::run(args),
        Commands::Baseline(args) => baseline::run(args),
        Commands::Waiver(args) => waiver::run(args),
        Commands::Route(args) => route::run(args),
        Commands::Render(args) => render::run(args),
        Commands::ListRules(args) => list_rules::run(args),
        Commands::Doctor(args) => doctor::run(args),
        Commands::Coverage(args) => coverage::run(args),
        Commands::Freshness(args) => freshness::run(args),
        Commands::Diagnostics(args) => diagnostics::run(args),
        Commands::Review(args) => review::run(args),
        Commands::Explain(args) => explain::run(args),
        Commands::ValidateConfig(args) => validate_config::run(args),
    }
}

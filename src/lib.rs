pub mod check;
pub mod cli;
pub mod config;
pub mod coverage;
pub mod diagnostics;
pub mod explain;
pub mod git;
pub mod metadata;
pub mod reporters;
pub mod review;
pub mod rules;
pub mod validate_config;

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
        Commands::Coverage(args) => coverage::run(args),
        Commands::Diagnostics(args) => diagnostics::run(args),
        Commands::Review(args) => review::run(args),
        Commands::Explain(args) => explain::run(args),
        Commands::ValidateConfig(args) => validate_config::run(args),
    }
}

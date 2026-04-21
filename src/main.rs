use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    match docpact::run(docpact::cli::Cli::parse()) {
        Ok(docpact::AppExit::Success) => ExitCode::SUCCESS,
        Ok(docpact::AppExit::LintFailure) => ExitCode::from(1),
        Err(error) => {
            eprintln!("Docpact error: {error}");
            ExitCode::from(2)
        }
    }
}

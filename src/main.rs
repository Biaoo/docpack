use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    match ai_doc_lint::run(ai_doc_lint::cli::Cli::parse()) {
        Ok(ai_doc_lint::AppExit::Success) => ExitCode::SUCCESS,
        Ok(ai_doc_lint::AppExit::LintFailure) => ExitCode::from(1),
        Err(error) => {
            eprintln!("AI doc lint error: {error}");
            ExitCode::from(2)
        }
    }
}

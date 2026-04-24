use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    match docpact::run(docpact::cli::Cli::parse()) {
        Ok(docpact::AppExit::Success) => ExitCode::SUCCESS,
        Ok(docpact::AppExit::LintFailure) => ExitCode::from(1),
        Err(error) => {
            eprintln!("Docpact error:");
            eprintln!("problem: {error}");
            eprintln!("why: the command could not complete with the current inputs or config.");
            eprintln!(
                "try: rerun the command with --help, or validate config with `docpact validate-config --strict`."
            );
            eprintln!(
                "related command: `docpact render --view routing-summary --format text` discovers route intent aliases."
            );
            ExitCode::from(2)
        }
    }
}

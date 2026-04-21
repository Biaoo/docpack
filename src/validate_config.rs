use miette::Result;

use crate::AppExit;
use crate::cli::ValidateConfigArgs;
use crate::config::{load_impact_files, root_dir_from_option};

pub fn run(args: ValidateConfigArgs) -> Result<AppExit> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let rules = load_impact_files(&root_dir, args.config.as_deref())?;
    println!("AI doc lint config loaded successfully: {} rule(s).", rules.len());
    Ok(AppExit::Success)
}

use miette::Result;

use crate::AppExit;
use crate::cli::ExplainArgs;
use crate::config::{load_impact_files, normalize_path, root_dir_from_option};
use crate::rules::{collect_expected_docs, match_rules};

pub fn run(args: ExplainArgs) -> Result<AppExit> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let loaded_rules = load_impact_files(&root_dir, args.config.as_deref())?;
    let path = normalize_path(&args.path.to_string_lossy());
    let matches = match_rules(std::slice::from_ref(&path), &loaded_rules);

    if matches.is_empty() {
        println!("Docpact: no matching rules for {}.", path);
        return Ok(AppExit::Success);
    }

    println!("Docpact matches for {}:", path);
    let expected = collect_expected_docs(&matches);
    for matched in matches {
        println!("- rule {} from {}", matched.rule.id, matched.source);
    }
    for expected_doc in expected.values() {
        let modes = expected_doc
            .modes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        println!("  expects doc {} (modes: {})", expected_doc.path, modes);
    }

    Ok(AppExit::Success)
}

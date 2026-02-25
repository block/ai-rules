use crate::operations;
use crate::utils::file_utils;
use crate::utils::prompt_utils::prompt_yes_no;
use anyhow::Result;
use std::path::Path;

pub fn run_migrate(
    current_dir: &Path,
    nested_depth: usize,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    // Discover all directories that would be migrated
    let mut to_migrate = Vec::new();
    file_utils::traverse_project_directories(current_dir, nested_depth, 0, &mut |dir| {
        if operations::migrate::should_migrate(dir) {
            to_migrate.push(dir.to_path_buf());
        }
        Ok(())
    })?;

    if to_migrate.is_empty() {
        println!("No ai-rules/ directories found to migrate.");
        return Ok(());
    }

    if dry_run {
        println!("Dry run: would migrate {} project(s) to the agents.md standard:", to_migrate.len());
        for path in &to_migrate {
            println!("  {}", path.display());
        }
    } else if !force {
        println!(
            "This will migrate {} project(s) to the agents.md standard and remove ai-rules/ directories. This cannot be undone.",
            to_migrate.len()
        );
        if !prompt_yes_no("Proceed with migration?")? {
            println!("Migration cancelled.");
            return Ok(());
        }
    }

    let mut results = Vec::new();
    for dir in &to_migrate {
        let result = operations::migrate::run_migration_for_dir(dir, dry_run)?;
        results.push(result);
    }

    // Summary
    let migrated: Vec<_> = results.iter().filter(|r| !r.skipped).collect();
    if migrated.is_empty() && !dry_run {
        println!("No directories were migrated.");
    } else if dry_run {
        for r in &results {
            if !r.skipped {
                println!("  {}: would {}", r.path.display(), r.actions.join(", "));
            }
        }
    } else {
        for r in &migrated {
            println!("Migrated {}: {}", r.path.display(), r.actions.join(", "));
        }
    }

    Ok(())
}

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::constants::{AI_RULE_SOURCE_DIR, COMMANDS_DIR, GENERATED_COMMAND_SUFFIX, MD_EXTENSION};
use crate::utils::file_utils::{
    calculate_relative_path, create_relative_symlink, find_files_by_extension,
};

#[derive(Debug, Clone)]
pub struct CommandFile {
    pub name: String,
    pub relative_path: PathBuf,
    pub full_path: PathBuf,
}

/// Finds all command markdown files in ai-rules/commands/ directory
pub fn find_command_files(current_dir: &Path) -> Result<Vec<CommandFile>> {
    let commands_dir = current_dir.join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);

    if !commands_dir.exists() || !commands_dir.is_dir() {
        return Ok(Vec::new());
    }

    let command_paths = find_files_by_extension(&commands_dir, MD_EXTENSION)?;

    let mut command_files = Vec::new();
    for path in command_paths {
        if let Some(file_stem) = path.file_stem() {
            if let Some(name) = file_stem.to_str() {
                let relative_path = PathBuf::from(AI_RULE_SOURCE_DIR)
                    .join(COMMANDS_DIR)
                    .join(path.file_name().unwrap());

                command_files.push(CommandFile {
                    name: name.to_string(),
                    relative_path,
                    full_path: path,
                });
            }
        }
    }

    Ok(command_files)
}

/// Creates individual symlinks for each command file in the target directory
pub fn create_command_symlinks(current_dir: &Path, target_dir: &str) -> Result<Vec<PathBuf>> {
    let command_files = find_command_files(current_dir)?;
    if command_files.is_empty() {
        return Ok(Vec::new());
    }

    let mut created_symlinks = Vec::new();

    for command_file in command_files {
        let symlink_name = format!("{}-{}.md", command_file.name, GENERATED_COMMAND_SUFFIX);
        let from_path = PathBuf::from(target_dir).join(&symlink_name);
        let relative_source = calculate_relative_path(&from_path, &command_file.relative_path);
        let symlink_path = current_dir.join(&from_path);

        create_relative_symlink(&symlink_path, &relative_source)?;
        created_symlinks.push(symlink_path);
    }

    Ok(created_symlinks)
}

/// Removes generated command symlinks from target directory
pub fn remove_generated_command_symlinks(current_dir: &Path, target_dir: &str) -> Result<()> {
    use std::fs;

    let target_path = current_dir.join(target_dir);
    if !target_path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&target_path)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(file_name) = path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                let suffix_pattern = format!("-{}.md", GENERATED_COMMAND_SUFFIX);
                if name_str.ends_with(&suffix_pattern) && path.is_symlink() {
                    fs::remove_file(&path)?;
                }
            }
        }
    }

    Ok(())
}

/// Checks if generated command symlinks are in sync
pub fn check_command_symlinks_in_sync(current_dir: &Path, target_dir: &str) -> Result<bool> {
    use std::fs;

    let command_files = find_command_files(current_dir)?;
    let target_path = current_dir.join(target_dir);

    if command_files.is_empty() {
        if !target_path.exists() {
            return Ok(true);
        }

        for entry in fs::read_dir(&target_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    let suffix_pattern = format!("-{}.md", GENERATED_COMMAND_SUFFIX);
                    if name_str.ends_with(&suffix_pattern) && path.is_symlink() {
                        return Ok(false);
                    }
                }
            }
        }
        return Ok(true);
    }

    for command_file in command_files {
        let symlink_name = format!("{}-{}.md", command_file.name, GENERATED_COMMAND_SUFFIX);
        let symlink_path = target_path.join(&symlink_name);

        if !symlink_path.is_symlink() {
            return Ok(false);
        }

        let actual_target = fs::read_link(&symlink_path)?;
        let resolved_target = if actual_target.is_absolute() {
            actual_target
        } else {
            let symlink_parent = symlink_path.parent().unwrap_or(current_dir);
            symlink_parent.join(&actual_target)
        };

        let resolved_canonical = resolved_target.canonicalize().unwrap_or(resolved_target);
        let expected_canonical = command_file
            .full_path
            .canonicalize()
            .unwrap_or(command_file.full_path.clone());

        if resolved_canonical != expected_canonical {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Returns gitignore patterns for generated command symlinks
pub fn get_command_gitignore_patterns(target_dir: &str) -> Vec<String> {
    vec![format!("{}/*-{}.md", target_dir, GENERATED_COMMAND_SUFFIX)]
}

// === Subfolder-based command symlinks (for Claude) ===

/// Creates symlinks for commands in a subdirectory (e.g., .claude/commands/ai-rules/)
pub fn create_command_symlinks_in_subdir(
    current_dir: &Path,
    target_dir: &str,
    subdir: &str,
) -> Result<Vec<PathBuf>> {
    let command_files = find_command_files(current_dir)?;
    if command_files.is_empty() {
        return Ok(Vec::new());
    }

    let mut created_symlinks = Vec::new();

    for command_file in command_files {
        // Use original name in subfolder (e.g., ai-rules/commit.md)
        let symlink_name = format!("{}.md", command_file.name);
        let from_path = PathBuf::from(target_dir).join(subdir).join(&symlink_name);
        let relative_source = calculate_relative_path(&from_path, &command_file.relative_path);
        let symlink_path = current_dir.join(&from_path);

        create_relative_symlink(&symlink_path, &relative_source)?;
        created_symlinks.push(symlink_path);
    }

    Ok(created_symlinks)
}

/// Removes command symlinks from a subdirectory
pub fn remove_command_symlinks_in_subdir(
    current_dir: &Path,
    target_dir: &str,
    subdir: &str,
) -> Result<()> {
    use std::fs;

    let subdir_path = current_dir.join(target_dir).join(subdir);
    if subdir_path.exists() {
        fs::remove_dir_all(&subdir_path)?;
    }
    Ok(())
}

/// Checks if command symlinks in subdirectory are in sync
pub fn check_command_symlinks_in_subdir_in_sync(
    current_dir: &Path,
    target_dir: &str,
    subdir: &str,
) -> Result<bool> {
    use std::fs;

    let command_files = find_command_files(current_dir)?;
    let subdir_path = current_dir.join(target_dir).join(subdir);

    if command_files.is_empty() {
        // No commands - subfolder should not exist
        return Ok(!subdir_path.exists());
    }

    if !subdir_path.exists() {
        return Ok(false);
    }

    // Check all expected symlinks exist and point to correct targets
    for command_file in &command_files {
        let symlink_name = format!("{}.md", command_file.name);
        let symlink_path = subdir_path.join(&symlink_name);

        if !symlink_path.is_symlink() {
            return Ok(false);
        }

        let actual_target = fs::read_link(&symlink_path)?;
        let resolved_target = if actual_target.is_absolute() {
            actual_target
        } else {
            let symlink_parent = symlink_path.parent().unwrap_or(current_dir);
            symlink_parent.join(&actual_target)
        };

        let resolved_canonical = resolved_target.canonicalize().unwrap_or(resolved_target);
        let expected_canonical = command_file
            .full_path
            .canonicalize()
            .unwrap_or(command_file.full_path.clone());

        if resolved_canonical != expected_canonical {
            return Ok(false);
        }
    }

    // Check no extra files exist in subfolder
    let mut expected_count = 0;
    for entry in fs::read_dir(&subdir_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_symlink() || path.is_file() {
            expected_count += 1;
        }
    }

    Ok(expected_count == command_files.len())
}

/// Returns gitignore patterns for subfolder-based command symlinks
pub fn get_command_gitignore_patterns_subdir(target_dir: &str, subdir: &str) -> Vec<String> {
    vec![format!("{}/{}/", target_dir, subdir)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_command_files_empty_when_no_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_command_files(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_find_command_files_with_commands() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&commands_dir).unwrap();

        fs::write(commands_dir.join("commit.md"), "Commit command content").unwrap();
        fs::write(commands_dir.join("review.md"), "Review command content").unwrap();
        fs::write(commands_dir.join("readme.txt"), "Not a markdown file").unwrap();

        let result = find_command_files(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 2);

        let names: Vec<String> = result.iter().map(|c| c.name.clone()).collect();
        assert!(names.contains(&"commit".to_string()));
        assert!(names.contains(&"review".to_string()));
    }

    #[test]
    fn test_create_command_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&commands_dir).unwrap();
        fs::write(commands_dir.join("commit.md"), "Commit command").unwrap();
        fs::write(commands_dir.join("review.md"), "Review command").unwrap();

        let symlinks = create_command_symlinks(temp_dir.path(), ".claude/commands").unwrap();
        assert_eq!(symlinks.len(), 2);

        let commit_symlink = temp_dir
            .path()
            .join(".claude/commands")
            .join(format!("commit-{}.md", GENERATED_COMMAND_SUFFIX));
        assert!(commit_symlink.is_symlink());

        let content = fs::read_to_string(&commit_symlink).unwrap();
        assert_eq!(content, "Commit command");
    }

    #[test]
    fn test_remove_generated_command_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&commands_dir).unwrap();
        fs::write(commands_dir.join("test.md"), "Test").unwrap();

        create_command_symlinks(temp_dir.path(), ".claude/commands").unwrap();

        let commands_path = temp_dir.path().join(".claude/commands");
        fs::write(commands_path.join("custom.md"), "User's custom command").unwrap();

        remove_generated_command_symlinks(temp_dir.path(), ".claude/commands").unwrap();

        let generated = commands_path.join(format!("test-{}.md", GENERATED_COMMAND_SUFFIX));
        assert!(!generated.exists());

        assert!(commands_path.join("custom.md").exists());
    }
}

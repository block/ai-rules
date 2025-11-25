use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::constants::{AI_RULE_SOURCE_DIR, COMMANDS_DIR, GENERATED_FILE_PREFIX, MD_EXTENSION};
use crate::utils::file_utils::{
    calculate_relative_path, create_relative_symlink, find_files_by_extension,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CommandFile {
    pub name: String,
    pub relative_path: PathBuf,
    pub full_path: PathBuf,
}

/// Finds all command markdown files in ai-rules/commands/ directory
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn create_command_symlinks(current_dir: &Path, target_dir: &str) -> Result<Vec<PathBuf>> {
    let command_files = find_command_files(current_dir)?;
    if command_files.is_empty() {
        return Ok(Vec::new());
    }

    let mut created_symlinks = Vec::new();

    for command_file in command_files {
        let symlink_name = format!("{}{}.md", GENERATED_FILE_PREFIX, command_file.name);
        let from_path = PathBuf::from(target_dir).join(&symlink_name);
        let relative_source = calculate_relative_path(&from_path, &command_file.relative_path);
        let symlink_path = current_dir.join(&from_path);

        create_relative_symlink(&symlink_path, &relative_source)?;
        created_symlinks.push(symlink_path);
    }

    Ok(created_symlinks)
}

/// Removes generated command symlinks from target directory
#[allow(dead_code)]
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
                if name_str.starts_with(GENERATED_FILE_PREFIX) && path.is_symlink() {
                    fs::remove_file(&path)?;
                }
            }
        }
    }

    Ok(())
}

/// Checks if generated command symlinks are in sync
#[allow(dead_code)]
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
                    if name_str.starts_with(GENERATED_FILE_PREFIX) && path.is_symlink() {
                        return Ok(false);
                    }
                }
            }
        }
        return Ok(true);
    }

    for command_file in command_files {
        let symlink_name = format!("{}{}.md", GENERATED_FILE_PREFIX, command_file.name);
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
#[allow(dead_code)]
pub fn get_command_gitignore_patterns(target_dir: &str) -> Vec<String> {
    vec![format!("{}/{}*.md", target_dir, GENERATED_FILE_PREFIX)]
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
            .join(format!("{}commit.md", GENERATED_FILE_PREFIX));
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

        let generated = commands_path.join(format!("{}test.md", GENERATED_FILE_PREFIX));
        assert!(!generated.exists());

        assert!(commands_path.join("custom.md").exists());
    }
}

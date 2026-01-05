use crate::agents::command_generator::CommandGeneratorTrait;
use crate::constants::{CURSOR_COMMANDS_DIR, GENERATED_COMMANDS_SUBDIR};
use crate::operations::{find_command_files, get_command_body_content};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CursorCommandGenerator;

impl CommandGeneratorTrait for CursorCommandGenerator {
    fn generate_commands(&self, current_dir: &Path) -> HashMap<PathBuf, String> {
        let mut files = HashMap::new();

        let command_files = match find_command_files(current_dir) {
            Ok(files) => files,
            Err(_) => return files,
        };

        if command_files.is_empty() {
            return files;
        }

        let commands_dir = current_dir
            .join(CURSOR_COMMANDS_DIR)
            .join(GENERATED_COMMANDS_SUBDIR);

        for command in command_files {
            let output_name = format!("{}.md", command.name);
            let output_path = commands_dir.join(&output_name);

            // Strip frontmatter for Cursor
            let content = get_command_body_content(&command);
            files.insert(output_path, content);
        }

        files
    }

    fn clean_commands(&self, current_dir: &Path) -> Result<()> {
        let commands_subdir = current_dir
            .join(CURSOR_COMMANDS_DIR)
            .join(GENERATED_COMMANDS_SUBDIR);
        if commands_subdir.exists() {
            fs::remove_dir_all(&commands_subdir)?;
        }
        Ok(())
    }

    fn check_commands(&self, current_dir: &Path) -> Result<bool> {
        let command_files = find_command_files(current_dir)?;
        let commands_subdir = current_dir
            .join(CURSOR_COMMANDS_DIR)
            .join(GENERATED_COMMANDS_SUBDIR);

        if command_files.is_empty() {
            // No commands - subfolder should not exist
            return Ok(!commands_subdir.exists());
        }

        // Check all expected files exist with correct content
        let expected_files = self.generate_commands(current_dir);
        for (path, expected_content) in &expected_files {
            if !path.exists() {
                return Ok(false);
            }
            let actual_content = fs::read_to_string(path)?;
            if actual_content != *expected_content {
                return Ok(false);
            }
        }

        // Check no extra files exist in subfolder
        if commands_subdir.exists() {
            for entry in fs::read_dir(&commands_subdir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && !expected_files.contains_key(&path) {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    fn command_gitignore_patterns(&self) -> Vec<String> {
        vec![format!(
            "{}/{}/",
            CURSOR_COMMANDS_DIR, GENERATED_COMMANDS_SUBDIR
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{AI_RULE_SOURCE_DIR, COMMANDS_DIR};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_generate_commands_empty_when_no_commands() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CursorCommandGenerator;

        let files = generator.generate_commands(temp_dir.path());
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_generate_commands_strips_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&commands_dir).unwrap();

        let command_content =
            "---\nallowed-tools: Bash(git:*)\ndescription: Test command\n---\n\nCommand body";
        fs::write(commands_dir.join("test.md"), command_content).unwrap();

        let generator = CursorCommandGenerator;
        let files = generator.generate_commands(temp_dir.path());

        assert_eq!(files.len(), 1);
        let output_path = temp_dir
            .path()
            .join(CURSOR_COMMANDS_DIR)
            .join("ai-rules")
            .join("test.md");
        assert!(files.contains_key(&output_path));

        // Verify frontmatter is stripped
        let content = files.get(&output_path).unwrap();
        assert!(!content.contains("---"));
        assert!(!content.contains("allowed-tools: Bash(git:*)"));
        assert!(!content.contains("description: Test command"));
        assert!(content.contains("Command body"));
        assert_eq!(content.trim(), "Command body");
    }

    #[test]
    fn test_clean_commands_removes_generated_files() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(CURSOR_COMMANDS_DIR);
        let ai_rules_subdir = commands_dir.join("ai-rules");
        fs::create_dir_all(&ai_rules_subdir).unwrap();

        fs::write(ai_rules_subdir.join("test.md"), "generated").unwrap();
        fs::write(commands_dir.join("custom.md"), "user file").unwrap();

        let generator = CursorCommandGenerator;
        generator.clean_commands(temp_dir.path()).unwrap();

        assert!(!ai_rules_subdir.exists());
        assert!(commands_dir.join("custom.md").exists());
    }

    #[test]
    fn test_clean_commands_removes_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let ai_rules_subdir = temp_dir.path().join(CURSOR_COMMANDS_DIR).join("ai-rules");
        fs::create_dir_all(&ai_rules_subdir).unwrap();

        fs::write(ai_rules_subdir.join("test.md"), "generated").unwrap();

        let generator = CursorCommandGenerator;
        generator.clean_commands(temp_dir.path()).unwrap();

        assert!(!ai_rules_subdir.exists());
    }

    #[test]
    fn test_check_commands_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let source_commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&source_commands_dir).unwrap();

        fs::write(source_commands_dir.join("test.md"), "Test command").unwrap();

        let generator = CursorCommandGenerator;

        // Not in sync initially
        assert!(!generator.check_commands(temp_dir.path()).unwrap());

        // Generate files
        let files = generator.generate_commands(temp_dir.path());
        for (path, content) in files {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, &content).unwrap();
        }

        // Now in sync
        assert!(generator.check_commands(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_check_commands_detects_extra_files() {
        let temp_dir = TempDir::new().unwrap();
        let source_commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        let target_commands_subdir = temp_dir.path().join(CURSOR_COMMANDS_DIR).join("ai-rules");
        fs::create_dir_all(&source_commands_dir).unwrap();
        fs::create_dir_all(&target_commands_subdir).unwrap();

        fs::write(source_commands_dir.join("test.md"), "Test").unwrap();

        let generator = CursorCommandGenerator;
        let files = generator.generate_commands(temp_dir.path());
        for (path, content) in files {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, &content).unwrap();
        }

        // Add extra generated file
        fs::write(target_commands_subdir.join("extra.md"), "extra").unwrap();

        // Should detect out of sync
        assert!(!generator.check_commands(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_command_gitignore_patterns() {
        let generator = CursorCommandGenerator;
        let patterns = generator.command_gitignore_patterns();

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0], ".cursor/commands/ai-rules/");
    }
}

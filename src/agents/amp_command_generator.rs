use crate::agents::command_generator::CommandGeneratorTrait;
use crate::constants::{AMP_COMMANDS_DIR, GENERATED_FILE_PREFIX};
use crate::operations::{find_command_files, get_command_body_content};
use crate::utils::file_utils::check_directory_files_match;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct AmpCommandGenerator;

impl CommandGeneratorTrait for AmpCommandGenerator {
    fn generate_commands(&self, current_dir: &Path) -> HashMap<PathBuf, String> {
        let mut files = HashMap::new();

        let command_files = match find_command_files(current_dir) {
            Ok(files) => files,
            Err(_) => return files,
        };

        if command_files.is_empty() {
            return files;
        }

        let commands_dir = current_dir.join(AMP_COMMANDS_DIR);

        for command in command_files {
            let output_name = format!("{}{}.md", GENERATED_FILE_PREFIX, command.name);
            let output_path = commands_dir.join(&output_name);

            // Strip frontmatter for AMP
            let content = get_command_body_content(&command);
            files.insert(output_path, content);
        }

        files
    }

    fn clean_commands(&self, current_dir: &Path) -> Result<()> {
        let commands_dir = current_dir.join(AMP_COMMANDS_DIR);
        if !commands_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&commands_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    if name_str.starts_with(GENERATED_FILE_PREFIX) {
                        fs::remove_file(&path)?;
                    }
                }
            }
        }

        // Remove empty directory
        if commands_dir.exists() && fs::read_dir(&commands_dir)?.next().is_none() {
            fs::remove_dir(&commands_dir)?;
        }

        // Remove empty parent directory (.agents) if it exists and is empty
        let parent_dir = current_dir.join(".agents");
        if parent_dir.exists() && fs::read_dir(&parent_dir)?.next().is_none() {
            fs::remove_dir(&parent_dir)?;
        }

        Ok(())
    }

    fn check_commands(&self, current_dir: &Path) -> Result<bool> {
        let command_files = find_command_files(current_dir)?;
        let commands_dir = current_dir.join(AMP_COMMANDS_DIR);

        if command_files.is_empty() {
            // No commands - directory should not exist or be empty of generated files
            if !commands_dir.exists() {
                return Ok(true);
            }
            for entry in fs::read_dir(&commands_dir)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(GENERATED_FILE_PREFIX) {
                        return Ok(false);
                    }
                }
            }
            return Ok(true);
        }

        let expected_files = self.generate_commands(current_dir);
        check_directory_files_match(&commands_dir, &expected_files, GENERATED_FILE_PREFIX)
    }

    fn command_gitignore_patterns(&self) -> Vec<String> {
        vec![format!("{}/{}*.md", AMP_COMMANDS_DIR, GENERATED_FILE_PREFIX)]
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
        let generator = AmpCommandGenerator;

        let files = generator.generate_commands(temp_dir.path());
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_generate_commands_strips_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&commands_dir).unwrap();

        let command_content = "---\nallowed-tools: Bash(git:*)\ndescription: Test command\n---\n\nCommand body";
        fs::write(commands_dir.join("test.md"), command_content).unwrap();

        let generator = AmpCommandGenerator;
        let files = generator.generate_commands(temp_dir.path());

        assert_eq!(files.len(), 1);
        let output_path = temp_dir.path().join(AMP_COMMANDS_DIR).join("ai-rules-generated-test.md");
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
        let commands_dir = temp_dir.path().join(AMP_COMMANDS_DIR);
        fs::create_dir_all(&commands_dir).unwrap();

        fs::write(commands_dir.join("ai-rules-generated-test.md"), "generated").unwrap();
        fs::write(commands_dir.join("custom.md"), "user file").unwrap();

        let generator = AmpCommandGenerator;
        generator.clean_commands(temp_dir.path()).unwrap();

        assert!(!commands_dir.join("ai-rules-generated-test.md").exists());
        assert!(commands_dir.join("custom.md").exists());
    }

    #[test]
    fn test_clean_commands_removes_empty_directories() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(AMP_COMMANDS_DIR);
        fs::create_dir_all(&commands_dir).unwrap();

        fs::write(commands_dir.join("ai-rules-generated-test.md"), "generated").unwrap();

        let generator = AmpCommandGenerator;
        generator.clean_commands(temp_dir.path()).unwrap();

        // Both .agents/commands and .agents should be removed
        assert!(!commands_dir.exists());
        assert!(!temp_dir.path().join(".agents").exists());
    }

    #[test]
    fn test_check_commands_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let source_commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&source_commands_dir).unwrap();

        fs::write(source_commands_dir.join("test.md"), "Test command").unwrap();

        let generator = AmpCommandGenerator;

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
        let target_commands_dir = temp_dir.path().join(AMP_COMMANDS_DIR);
        fs::create_dir_all(&source_commands_dir).unwrap();
        fs::create_dir_all(&target_commands_dir).unwrap();

        fs::write(source_commands_dir.join("test.md"), "Test").unwrap();

        let generator = AmpCommandGenerator;
        let files = generator.generate_commands(temp_dir.path());
        for (path, content) in files {
            fs::write(&path, &content).unwrap();
        }

        // Add extra generated file
        fs::write(target_commands_dir.join("ai-rules-generated-extra.md"), "extra").unwrap();

        // Should detect out of sync
        assert!(!generator.check_commands(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_command_gitignore_patterns() {
        let generator = AmpCommandGenerator;
        let patterns = generator.command_gitignore_patterns();

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0], ".agents/commands/ai-rules-generated-*.md");
    }
}

use crate::agents::command_generator::CommandGeneratorTrait;
use crate::operations::command_reader::{
    check_command_symlinks_in_subdir_in_sync, check_command_symlinks_in_sync,
    create_command_symlinks, create_command_symlinks_in_subdir, get_command_gitignore_patterns,
    get_command_gitignore_patterns_subdir, remove_command_symlinks_in_subdir,
    remove_generated_command_symlinks,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct ExternalCommandsGenerator {
    target_dir: String,
    /// Optional subdirectory for symlinks (e.g., "ai-rules" for .claude/commands/ai-rules/)
    /// When None, uses flat structure with -ai-rules.md suffix
    subdir: Option<String>,
}

impl ExternalCommandsGenerator {
    /// Create a generator with flat structure (name-ai-rules.md)
    pub fn new(target_dir: &str) -> Self {
        Self {
            target_dir: target_dir.to_string(),
            subdir: None,
        }
    }

    /// Create a generator with subfolder structure (subdir/name.md)
    pub fn with_subdir(target_dir: &str, subdir: &str) -> Self {
        Self {
            target_dir: target_dir.to_string(),
            subdir: Some(subdir.to_string()),
        }
    }
}

impl CommandGeneratorTrait for ExternalCommandsGenerator {
    fn generate_command_symlinks(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        match &self.subdir {
            Some(subdir) => {
                create_command_symlinks_in_subdir(current_dir, &self.target_dir, subdir)
            }
            None => create_command_symlinks(current_dir, &self.target_dir),
        }
    }

    fn clean_commands(&self, current_dir: &Path) -> Result<()> {
        match &self.subdir {
            Some(subdir) => {
                remove_command_symlinks_in_subdir(current_dir, &self.target_dir, subdir)
            }
            None => remove_generated_command_symlinks(current_dir, &self.target_dir),
        }
    }

    fn check_commands(&self, current_dir: &Path) -> Result<bool> {
        match &self.subdir {
            Some(subdir) => {
                check_command_symlinks_in_subdir_in_sync(current_dir, &self.target_dir, subdir)
            }
            None => check_command_symlinks_in_sync(current_dir, &self.target_dir),
        }
    }

    fn command_gitignore_patterns(&self) -> Vec<String> {
        match &self.subdir {
            Some(subdir) => get_command_gitignore_patterns_subdir(&self.target_dir, subdir),
            None => get_command_gitignore_patterns(&self.target_dir),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{AI_RULE_SOURCE_DIR, COMMANDS_DIR, GENERATED_COMMAND_SUFFIX};
    use std::fs;
    use tempfile::TempDir;

    fn create_command_file(temp_dir: &Path, command_name: &str, content: &str) -> PathBuf {
        let command_dir = temp_dir.join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&command_dir).unwrap();
        let file_path = command_dir.join(format!("{}.md", command_name));
        fs::write(&file_path, content).unwrap();
        file_path
    }

    // === Flat structure tests (AMP, Cursor) ===

    #[test]
    fn test_flat_generator_target_dir() {
        let generator = ExternalCommandsGenerator::new(".agents/commands");
        assert_eq!(generator.target_dir, ".agents/commands");
        assert!(generator.subdir.is_none());
    }

    #[test]
    fn test_flat_generator_generate_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalCommandsGenerator::new(".agents/commands");

        create_command_file(temp_dir.path(), "my-command", "command content");

        let result = generator.generate_command_symlinks(temp_dir.path());
        assert!(result.is_ok());

        let symlinks = result.unwrap();
        assert_eq!(symlinks.len(), 1);

        let symlink_path = temp_dir
            .path()
            .join(".agents/commands")
            .join(format!("my-command-{}.md", GENERATED_COMMAND_SUFFIX));
        assert!(symlink_path.is_symlink());
    }

    #[test]
    fn test_flat_generator_clean() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalCommandsGenerator::new(".agents/commands");

        create_command_file(temp_dir.path(), "my-command", "command content");
        generator
            .generate_command_symlinks(temp_dir.path())
            .unwrap();

        // Create user command (real file, not symlink)
        let user_command = temp_dir.path().join(".agents/commands/custom.md");
        fs::write(&user_command, "user content").unwrap();

        generator.clean_commands(temp_dir.path()).unwrap();

        // Generated symlink should be gone
        let generated = temp_dir
            .path()
            .join(".agents/commands")
            .join(format!("my-command-{}.md", GENERATED_COMMAND_SUFFIX));
        assert!(!generated.exists());

        // User command should remain
        assert!(user_command.exists());
    }

    #[test]
    fn test_flat_generator_check_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalCommandsGenerator::new(".agents/commands");

        create_command_file(temp_dir.path(), "my-command", "command content");

        // Not in sync before generating
        assert!(!generator.check_commands(temp_dir.path()).unwrap());

        generator
            .generate_command_symlinks(temp_dir.path())
            .unwrap();

        // Now in sync
        assert!(generator.check_commands(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_flat_generator_gitignore_patterns() {
        let generator = ExternalCommandsGenerator::new(".agents/commands");
        let patterns = generator.command_gitignore_patterns();
        assert_eq!(
            patterns,
            vec![format!(
                ".agents/commands/*-{}.md",
                GENERATED_COMMAND_SUFFIX
            )]
        );
    }

    // === Subfolder structure tests (Claude) ===

    #[test]
    fn test_subdir_generator_target_dir() {
        let generator = ExternalCommandsGenerator::with_subdir(".claude/commands", "ai-rules");
        assert_eq!(generator.target_dir, ".claude/commands");
        assert_eq!(generator.subdir, Some("ai-rules".to_string()));
    }

    #[test]
    fn test_subdir_generator_generate_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalCommandsGenerator::with_subdir(".claude/commands", "ai-rules");

        create_command_file(temp_dir.path(), "my-command", "command content");

        let result = generator.generate_command_symlinks(temp_dir.path());
        assert!(result.is_ok());

        let symlinks = result.unwrap();
        assert_eq!(symlinks.len(), 1);

        // Subfolder uses original name without suffix
        let symlink_path = temp_dir
            .path()
            .join(".claude/commands/ai-rules/my-command.md");
        assert!(symlink_path.is_symlink());

        // Verify content is accessible through symlink
        let content = fs::read_to_string(&symlink_path).unwrap();
        assert_eq!(content, "command content");
    }

    #[test]
    fn test_subdir_generator_clean() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalCommandsGenerator::with_subdir(".claude/commands", "ai-rules");

        create_command_file(temp_dir.path(), "my-command", "command content");
        generator
            .generate_command_symlinks(temp_dir.path())
            .unwrap();

        // Create user command in parent directory (should be preserved)
        let user_command = temp_dir.path().join(".claude/commands/custom.md");
        fs::write(&user_command, "user content").unwrap();

        generator.clean_commands(temp_dir.path()).unwrap();

        // Subfolder should be removed entirely
        let subdir_path = temp_dir.path().join(".claude/commands/ai-rules");
        assert!(!subdir_path.exists());

        // User command in parent should remain
        assert!(user_command.exists());
    }

    #[test]
    fn test_subdir_generator_check_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalCommandsGenerator::with_subdir(".claude/commands", "ai-rules");

        create_command_file(temp_dir.path(), "my-command", "command content");

        // Not in sync before generating
        assert!(!generator.check_commands(temp_dir.path()).unwrap());

        generator
            .generate_command_symlinks(temp_dir.path())
            .unwrap();

        // Now in sync
        assert!(generator.check_commands(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_subdir_generator_check_no_commands() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalCommandsGenerator::with_subdir(".claude/commands", "ai-rules");

        // No commands - should be in sync
        assert!(generator.check_commands(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_subdir_generator_gitignore_patterns() {
        let generator = ExternalCommandsGenerator::with_subdir(".claude/commands", "ai-rules");
        let patterns = generator.command_gitignore_patterns();
        assert_eq!(patterns, vec![".claude/commands/ai-rules/"]);
    }

    #[test]
    fn test_different_generators_for_different_agents() {
        // Claude uses subfolder
        let claude_gen = ExternalCommandsGenerator::with_subdir(".claude/commands", "ai-rules");
        assert_eq!(
            claude_gen.command_gitignore_patterns(),
            vec![".claude/commands/ai-rules/"]
        );

        // Cursor uses subfolder
        let cursor_gen = ExternalCommandsGenerator::with_subdir(".cursor/commands", "ai-rules");
        assert_eq!(
            cursor_gen.command_gitignore_patterns(),
            vec![".cursor/commands/ai-rules/"]
        );

        // AMP uses flat
        let amp_gen = ExternalCommandsGenerator::new(".agents/commands");
        assert_eq!(
            amp_gen.command_gitignore_patterns(),
            vec![format!(
                ".agents/commands/*-{}.md",
                GENERATED_COMMAND_SUFFIX
            )]
        );
    }
}

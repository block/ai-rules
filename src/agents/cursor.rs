use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::external_commands_generator::ExternalCommandsGenerator;
use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::mcp_generator::{ExternalMcpGenerator, McpGeneratorTrait};
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::SingleFileBasedGenerator;
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{
    AGENTS_MD_FILENAME, CURSOR_COMMANDS_DIR, CURSOR_COMMANDS_SUBDIR, CURSOR_SKILLS_DIR, MCP_JSON,
};
use crate::models::SourceFile;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CursorGenerator {
    inner: SingleFileBasedGenerator,
}

impl CursorGenerator {
    pub fn new() -> Self {
        Self {
            inner: SingleFileBasedGenerator::new("cursor", AGENTS_MD_FILENAME),
        }
    }
}

impl Default for CursorGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn get_cursor_rules_dir(current_dir: &Path) -> PathBuf {
    current_dir.join(".cursor").join("rules")
}

fn clean_cursor_rules_dir(current_dir: &Path) -> Result<()> {
    let cursor_rules_dir = get_cursor_rules_dir(current_dir);
    if cursor_rules_dir.exists() {
        fs::remove_dir_all(cursor_rules_dir)?;
    }
    Ok(())
}

impl AgentRuleGenerator for CursorGenerator {
    fn name(&self) -> &str {
        "cursor"
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        clean_cursor_rules_dir(current_dir)?;
        self.inner.clean(current_dir)
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        self.inner
            .generate_agent_contents(source_files, current_dir)
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        Ok(self.inner.check_agent_contents(source_files, current_dir)?
            && !get_cursor_rules_dir(current_dir).exists())
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        Ok(self.inner.check_symlink(current_dir)? && !get_cursor_rules_dir(current_dir).exists())
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        self.inner.gitignore_patterns()
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        clean_cursor_rules_dir(current_dir)?;
        self.inner.generate_symlink(current_dir)
    }

    fn uses_inlined_symlink(&self) -> bool {
        true
    }

    fn generate_inlined_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        clean_cursor_rules_dir(current_dir)?;
        self.inner.generate_inlined_symlink(current_dir)
    }

    fn check_inlined_symlink(&self, current_dir: &Path) -> Result<bool> {
        Ok(self.inner.check_inlined_symlink(current_dir)?
            && !get_cursor_rules_dir(current_dir).exists())
    }

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        Some(Box::new(ExternalMcpGenerator::new(
            PathBuf::from(".cursor").join(MCP_JSON),
        )))
    }

    fn command_generator(&self) -> Option<Box<dyn CommandGeneratorTrait>> {
        Some(Box::new(ExternalCommandsGenerator::with_subdir(
            CURSOR_COMMANDS_DIR,
            CURSOR_COMMANDS_SUBDIR,
        )))
    }

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        Some(Box::new(ExternalSkillsGenerator::new(CURSOR_SKILLS_DIR)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    fn create_standard_test_source_file() -> SourceFile {
        create_test_source_file(
            "test",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "test body",
        )
    }

    #[test]
    fn test_cursor_generator_name() {
        let generator = CursorGenerator::default();

        assert_eq!(generator.name(), "cursor");
    }

    #[test]
    fn test_cursor_gitignore_patterns_use_agents_md() {
        let generator = CursorGenerator::default();
        let patterns = generator.gitignore_patterns();

        assert_eq!(patterns, vec![AGENTS_MD_FILENAME.to_string()]);
    }

    #[test]
    fn test_generate_agent_contents_uses_agents_md() {
        let generator = CursorGenerator::default();
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_standard_test_source_file()];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        let expected_path = temp_dir.path().join(AGENTS_MD_FILENAME);
        let content = result.get(&expected_path).unwrap();

        assert_eq!(
            content,
            "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n"
        );
    }

    #[test]
    fn test_clean_non_existing_directory() {
        let generator = CursorGenerator::default();
        let temp_dir = TempDir::new().unwrap();

        let result = generator.clean(temp_dir.path());

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".cursor/rules");
    }

    #[test]
    fn test_clean_removes_existing_directory() {
        let generator = CursorGenerator::default();
        let temp_dir = TempDir::new().unwrap();
        create_file(
            temp_dir.path(),
            ".cursor/settings.json",
            "existing settings content",
        );
        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-test.mdc",
            "test content",
        );
        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-other.mdc",
            "other content",
        );
        assert_file_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");

        let result = generator.clean(temp_dir.path());

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".cursor/rules");
        assert_file_exists(temp_dir.path(), ".cursor/settings.json");
    }

    #[test]
    fn test_clean_removes_agents_md_symlink() {
        let generator = CursorGenerator::default();
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let result = generator.generate_symlink(temp_dir.path());
        assert!(result.is_ok());

        let agents_md_path = temp_dir.path().join(AGENTS_MD_FILENAME);
        assert!(agents_md_path.is_symlink());

        let result = generator.clean(temp_dir.path());
        assert!(result.is_ok());

        assert!(!agents_md_path.exists());

        assert_file_exists(temp_dir.path(), "ai-rules/AGENTS.md");
    }

    #[test]
    fn test_clean_removes_stale_cursor_rules_dir_and_agents_md() {
        let generator = CursorGenerator::default();
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), AGENTS_MD_FILENAME, "shared agents content");
        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-stale.mdc",
            "stale content",
        );

        generator.clean(temp_dir.path()).unwrap();

        assert_file_not_exists(temp_dir.path(), AGENTS_MD_FILENAME);
        assert_file_not_exists(temp_dir.path(), ".cursor/rules");
    }

    #[test]
    fn test_check_empty_source_files_with_stale_cursor_rules_dir() {
        let generator = CursorGenerator::default();
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-stale.mdc",
            "stale content",
        );

        let result = generator
            .check_agent_contents(&[], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_symlink_with_correct_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CursorGenerator::default();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let result = generator.generate_symlink(temp_dir.path());
        assert!(result.is_ok());

        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_symlink_rejects_stale_cursor_rules_dir() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CursorGenerator::default();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");
        generator.generate_symlink(temp_dir.path()).unwrap();
        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-stale.mdc",
            "stale content",
        );

        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_agent_contents_rejects_stale_cursor_rules_dir() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CursorGenerator::default();
        let source_file = create_standard_test_source_file();

        create_file(
            temp_dir.path(),
            AGENTS_MD_FILENAME,
            "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n",
        );
        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-stale.mdc",
            "stale content",
        );

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_inlined_symlink_rejects_stale_cursor_rules_dir() {
        let generator = CursorGenerator::default();
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-AGENTS.md",
            "# Inlined content\n",
        );
        generator.generate_inlined_symlink(temp_dir.path()).unwrap();
        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-stale.mdc",
            "stale content",
        );

        let result = generator.check_inlined_symlink(temp_dir.path()).unwrap();
        assert!(!result);
    }
}

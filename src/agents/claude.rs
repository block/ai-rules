use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::external_commands_generator::ExternalCommandsGenerator;
use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::mcp_generator::{ExternalMcpGenerator, McpGeneratorTrait};
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{
    CLAUDE_COMMANDS_DIR, CLAUDE_COMMANDS_SUBDIR, CLAUDE_MCP_JSON, CLAUDE_SKILLS_DIR,
};
use crate::models::source_file::SourceFile;
use crate::operations::remove_generated_skill_symlinks;
use crate::utils::file_utils::{
    check_agents_md_symlink, check_inlined_file_symlink, create_symlink_to_agents_md,
    create_symlink_to_inlined_file,
};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ClaudeGenerator {
    name: String,
    output_filename: String,
}

impl ClaudeGenerator {
    pub fn new(name: &str, output_filename: &str) -> Self {
        Self {
            name: name.to_string(),
            output_filename: output_filename.to_string(),
        }
    }
}

impl AgentRuleGenerator for ClaudeGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        let output_file = current_dir.join(&self.output_filename);
        if output_file.exists() || output_file.is_symlink() {
            fs::remove_file(&output_file)?;
        }
        remove_generated_skill_symlinks(current_dir, CLAUDE_SKILLS_DIR)?;

        Ok(())
    }

    fn generate_agent_contents(
        &self,
        _source_files: &[SourceFile],
        _current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        HashMap::new()
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        if source_files.is_empty() {
            let file_path = current_dir.join(&self.output_filename);
            if file_path.exists() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(&self.output_filename);
        check_agents_md_symlink(current_dir, &output_file)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        vec![self.output_filename.clone()]
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_agents_md(current_dir, Path::new(&self.output_filename))?;
        if success {
            Ok(vec![current_dir.join(&self.output_filename)])
        } else {
            Ok(vec![])
        }
    }

    fn uses_inlined_symlink(&self) -> bool {
        true
    }

    fn generate_inlined_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success =
            create_symlink_to_inlined_file(current_dir, Path::new(&self.output_filename))?;
        if success {
            Ok(vec![current_dir.join(&self.output_filename)])
        } else {
            Ok(vec![])
        }
    }

    fn check_inlined_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(&self.output_filename);
        check_inlined_file_symlink(current_dir, &output_file)
    }

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        Some(Box::new(ExternalMcpGenerator::new(PathBuf::from(
            CLAUDE_MCP_JSON,
        ))))
    }

    fn command_generator(&self) -> Option<Box<dyn CommandGeneratorTrait>> {
        Some(Box::new(ExternalCommandsGenerator::with_subdir(
            CLAUDE_COMMANDS_DIR,
            CLAUDE_COMMANDS_SUBDIR,
        )))
    }

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        Some(Box::new(ExternalSkillsGenerator::new(CLAUDE_SKILLS_DIR)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_clean_removes_both_file_and_skills() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md");

        create_file(temp_dir.path(), "CLAUDE.md", "content");

        let generated_skills_dir = temp_dir
            .path()
            .join(".claude/skills/ai-rules-generated-test");
        std::fs::create_dir_all(&generated_skills_dir).unwrap();
        std::fs::write(generated_skills_dir.join("SKILL.md"), "generated skill").unwrap();

        let user_skills_dir = temp_dir.path().join(".claude/skills/my-custom-skill");
        std::fs::create_dir_all(&user_skills_dir).unwrap();
        std::fs::write(user_skills_dir.join("SKILL.md"), "user skill").unwrap();

        generator.clean(temp_dir.path()).unwrap();

        assert!(!temp_dir.path().join("CLAUDE.md").exists());
        assert!(!generated_skills_dir.exists());
        assert!(user_skills_dir.exists());
    }

    #[test]
    fn test_gitignore_patterns() {
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md");
        let patterns = generator.gitignore_patterns();

        assert_eq!(patterns.len(), 1);
        assert!(patterns.contains(&"CLAUDE.md".to_string()));
    }

    #[test]
    fn test_generate_agent_contents_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md");
        let source_files = vec![
            create_test_source_file(
                "always1",
                "Always",
                true,
                vec!["**/*.ts".to_string()],
                "Always content",
            ),
            create_test_source_file(
                "optional1",
                "Optional",
                false,
                vec!["**/*.js".to_string()],
                "Optional content",
            ),
        ];

        let files = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(files.len(), 0);
    }
}

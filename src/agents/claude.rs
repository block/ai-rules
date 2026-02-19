use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::external_commands_generator::ExternalCommandsGenerator;
use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::mcp_generator::{ExternalMcpGenerator, McpGeneratorTrait};
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{
    CLAUDE_COMMANDS_DIR, CLAUDE_COMMANDS_SUBDIR, CLAUDE_MCP_JSON, CLAUDE_SKILLS_DIR,
    GENERATED_FILE_PREFIX,
};
use crate::models::source_file::SourceFile;
use crate::operations::{claude_skills, generate_inlined_required_content};
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
    skills_mode: bool,
}

impl ClaudeGenerator {
    pub fn new(name: &str, output_filename: &str, skills_mode: bool) -> Self {
        Self {
            name: name.to_string(),
            output_filename: output_filename.to_string(),
            skills_mode,
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
        claude_skills::remove_generated_skills(current_dir)?;

        Ok(())
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        let mut all_files = HashMap::new();

        if !source_files.is_empty() && self.skills_mode {
            // In skills mode: inline required content (not @ refs), skills handle optional
            let content = generate_inlined_required_content(source_files);
            all_files.insert(current_dir.join(&self.output_filename), content);

            if let Ok(skill_files) =
                claude_skills::generate_skills_for_optional_rules(source_files, current_dir)
            {
                all_files.extend(skill_files);
            }
            // Non-skills mode is handled by generate_inlined_symlink
        }

        all_files
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        let file_path = current_dir.join(&self.output_filename);

        if source_files.is_empty() {
            if file_path.exists() {
                return Ok(false);
            }
        } else {
            if !file_path.exists() {
                return Ok(false);
            }
            // In skills mode: check inlined required content
            let expected_content = generate_inlined_required_content(source_files);
            let actual_content = fs::read_to_string(&file_path)?;
            if actual_content != expected_content {
                return Ok(false);
            }
        }

        if self.skills_mode {
            claude_skills::check_skills_in_sync(source_files, current_dir)
        } else {
            Ok(true)
        }
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(&self.output_filename);
        check_agents_md_symlink(current_dir, &output_file)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        let mut patterns = vec![self.output_filename.clone()];
        if self.skills_mode {
            patterns.push(format!("{}/{}*", CLAUDE_SKILLS_DIR, GENERATED_FILE_PREFIX));
        }
        patterns
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
        !self.skills_mode
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
        // Return a skills generator for user-defined skills in ai-rules/skills/
        // This is separate from the existing optional-rules-based skills
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
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", true);

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
    fn test_gitignore_patterns_includes_skills() {
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", true);
        let patterns = generator.gitignore_patterns();

        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"CLAUDE.md".to_string()));
        assert!(patterns.contains(&".claude/skills/ai-rules-generated-*".to_string()));
    }

    #[test]
    fn test_gitignore_patterns_no_skills_mode() {
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", false);
        let patterns = generator.gitignore_patterns();

        assert_eq!(patterns.len(), 1);
        assert!(patterns.contains(&"CLAUDE.md".to_string()));
    }

    #[test]
    fn test_generate_agent_contents_creates_both() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", true);
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

        assert_eq!(files.len(), 2);

        let claude_md_path = temp_dir.path().join("CLAUDE.md");
        let claude_content = files.get(&claude_md_path).expect("CLAUDE.md should exist");
        // In skills mode, CLAUDE.md should contain inlined required content with description header
        assert_eq!(claude_content, "# Always\n\nAlways content\n");

        let skill_path = temp_dir
            .path()
            .join(".claude/skills/ai-rules-generated-optional1/SKILL.md");
        let skill_content = files.get(&skill_path).expect("Skill file should exist");
        assert!(skill_content.contains("name: optional"));
        assert!(skill_content.contains("description: Optional"));
        assert!(
            skill_content.contains("@ai-rules/.generated-ai-rules/ai-rules-generated-optional1.md")
        );
    }

    #[test]
    fn test_generate_agent_contents_non_skills_mode() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", false);
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

        // In non-skills mode, generate_agent_contents returns empty
        // (symlinks handle output via generate_inlined_symlink)
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_check_agent_contents_validates_both() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", true);
        let source_files = vec![
            create_test_source_file("always1", "Always", true, vec![], "Always content"),
            create_test_source_file("optional1", "Optional", false, vec![], "Optional content"),
        ];

        // Initially not in sync (no files)
        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(!result);

        // Create CLAUDE.md with inlined required content
        let claude_content = generate_inlined_required_content(&source_files);
        create_file(temp_dir.path(), "CLAUDE.md", &claude_content);

        // Still not in sync (missing skill)
        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(!result);

        // Create skill file
        let skill_dir = temp_dir
            .path()
            .join(".claude/skills/ai-rules-generated-optional1");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: optional\ndescription: Optional\n---\n\n@ai-rules/.generated-ai-rules/ai-rules-generated-optional1.md",
        )
        .unwrap();

        // Now in sync
        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(result);
    }
}

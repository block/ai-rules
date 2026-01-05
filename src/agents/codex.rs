use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::{
    check_in_sync, clean_generated_files, generate_agent_file_contents,
};
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{AGENTS_MD_FILENAME, CODEX_SKILLS_DIR};
use crate::models::SourceFile;
use crate::utils::file_utils::{check_agents_md_symlink, create_symlink_to_agents_md};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct CodexGenerator {
    name: String,
    output_filename: String,
}

impl CodexGenerator {
    pub fn new() -> Self {
        Self {
            name: "codex".to_string(),
            output_filename: AGENTS_MD_FILENAME.to_string(),
        }
    }
}

impl Default for CodexGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRuleGenerator for CodexGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        clean_generated_files(current_dir, &self.output_filename)
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        generate_agent_file_contents(source_files, current_dir, &self.output_filename)
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        check_in_sync(source_files, current_dir, &self.output_filename)
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

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        Some(Box::new(ExternalSkillsGenerator::new(CODEX_SKILLS_DIR)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_codex_generator_name() {
        let generator = CodexGenerator::new();
        assert_eq!(generator.name(), "codex");
    }

    #[test]
    fn test_codex_generator_gitignore_patterns() {
        let generator = CodexGenerator::new();
        let patterns = generator.gitignore_patterns();
        assert_eq!(patterns, vec!["AGENTS.md".to_string()]);
    }

    #[test]
    fn test_codex_generator_clean() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        // Create an AGENTS.md file
        create_file(temp_dir.path(), "AGENTS.md", "existing content");
        assert_file_exists(temp_dir.path(), "AGENTS.md");

        // Clean should remove it
        let result = generator.clean(temp_dir.path());
        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), "AGENTS.md");
    }

    #[test]
    fn test_codex_generator_generate_agent_contents() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(result.len(), 1);
        let expected_path = temp_dir.path().join("AGENTS.md");
        let content = result.get(&expected_path).unwrap();
        assert!(content.contains("@ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md"));
    }

    #[test]
    fn test_codex_generator_check_agent_contents_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        // Write correct content
        let expected_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md\n";
        create_file(temp_dir.path(), "AGENTS.md", expected_content);

        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_codex_generator_check_agent_contents_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        // Write wrong content
        create_file(temp_dir.path(), "AGENTS.md", "wrong content");

        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_codex_generator_generate_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        // Create the source file for symlinking
        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let result = generator.generate_symlink(temp_dir.path());
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("AGENTS.md"));
    }

    #[test]
    fn test_codex_generator_check_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        // Create source and symlink
        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");
        generator.generate_symlink(temp_dir.path()).unwrap();

        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(result);
    }
}

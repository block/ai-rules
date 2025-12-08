use crate::agents::amp_command_generator::AmpCommandGenerator;
use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::{
    check_in_sync, clean_generated_files, generate_agent_file_contents,
};
use crate::constants::AGENTS_MD_FILENAME;
use crate::models::SourceFile;
use crate::utils::file_utils::{check_agents_md_symlink, create_symlink_to_agents_md};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct AmpGenerator;

impl AgentRuleGenerator for AmpGenerator {
    fn name(&self) -> &str {
        "amp"
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        clean_generated_files(current_dir, AGENTS_MD_FILENAME)
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        generate_agent_file_contents(source_files, current_dir, AGENTS_MD_FILENAME)
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        check_in_sync(source_files, current_dir, AGENTS_MD_FILENAME)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(AGENTS_MD_FILENAME);
        check_agents_md_symlink(current_dir, &output_file)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        vec![AGENTS_MD_FILENAME.to_string()]
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_agents_md(current_dir, Path::new(AGENTS_MD_FILENAME))?;
        if success {
            Ok(vec![current_dir.join(AGENTS_MD_FILENAME)])
        } else {
            Ok(vec![])
        }
    }

    fn command_generator(&self) -> Option<Box<dyn CommandGeneratorTrait>> {
        Some(Box::new(AmpCommandGenerator))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{AI_RULE_SOURCE_DIR, COMMANDS_DIR};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_amp_generator_name() {
        let generator = AmpGenerator;
        assert_eq!(generator.name(), "amp");
    }

    #[test]
    fn test_amp_generator_has_command_generator() {
        let generator = AmpGenerator;
        assert!(generator.command_generator().is_some());
    }

    #[test]
    fn test_amp_generator_gitignore_patterns() {
        let generator = AmpGenerator;
        let patterns = generator.gitignore_patterns();
        assert!(patterns.contains(&"AGENTS.md".to_string()));
    }

    #[test]
    fn test_amp_command_generator_integration() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(COMMANDS_DIR);
        fs::create_dir_all(&commands_dir).unwrap();

        let command_content = "---\ndescription: Test\n---\n\nCommand content";
        fs::write(commands_dir.join("test.md"), command_content).unwrap();

        let generator = AmpGenerator;
        let cmd_gen = generator.command_generator().unwrap();
        let files = cmd_gen.generate_commands(temp_dir.path());

        assert_eq!(files.len(), 1);

        // Verify frontmatter is stripped
        let (_, content) = files.iter().next().unwrap();
        assert!(!content.contains("---"));
        assert!(content.contains("Command content"));
    }
}

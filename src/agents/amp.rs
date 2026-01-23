use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::external_commands_generator::ExternalCommandsGenerator;
use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::{
    check_in_sync, clean_generated_files, generate_agent_file_contents,
};
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{
    AGENTS_MD_AGENTS, AGENTS_MD_FILENAME, AGENTS_MD_GROUP_NAME, AMP_COMMANDS_DIR, AMP_SKILLS_DIR,
};
use crate::models::source_file::filter_source_files_for_agent_group;
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
        let filtered_source_files =
            filter_source_files_for_agent_group(source_files, &AGENTS_MD_AGENTS);
        generate_agent_file_contents(
            &filtered_source_files,
            current_dir,
            AGENTS_MD_FILENAME,
            AGENTS_MD_GROUP_NAME,
        )
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        let filtered_source_files =
            filter_source_files_for_agent_group(source_files, &AGENTS_MD_AGENTS);
        check_in_sync(
            &filtered_source_files,
            current_dir,
            AGENTS_MD_FILENAME,
            AGENTS_MD_GROUP_NAME,
        )
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
        Some(Box::new(ExternalCommandsGenerator::new(AMP_COMMANDS_DIR)))
    }

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        Some(Box::new(ExternalSkillsGenerator::new(AMP_SKILLS_DIR)))
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

        // generate_command_symlinks creates symlinks (flat structure with -ai-rules.md suffix)
        let symlinks = cmd_gen.generate_command_symlinks(temp_dir.path()).unwrap();
        assert_eq!(symlinks.len(), 1);

        // Verify symlink was created with correct naming
        let symlink_path = temp_dir.path().join(".agents/commands/test-ai-rules.md");
        assert!(symlink_path.is_symlink());

        // Verify symlink points to source with frontmatter preserved
        let content = fs::read_to_string(&symlink_path).unwrap();
        assert!(content.contains("---"));
        assert!(content.contains("Command content"));
    }
}

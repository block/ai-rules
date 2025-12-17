use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::operations::skills_reader::{
    check_skill_symlinks_in_sync, create_skill_symlinks, get_skill_gitignore_patterns,
    remove_generated_skill_symlinks,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub struct ExternalSkillsGenerator {
    target_dir: String,
}

impl ExternalSkillsGenerator {
    #[allow(dead_code)]
    pub fn new(target_dir: &str) -> Self {
        Self {
            target_dir: target_dir.to_string(),
        }
    }
}

impl SkillsGeneratorTrait for ExternalSkillsGenerator {
    fn skills_target_dir(&self) -> &str {
        &self.target_dir
    }

    fn generate_skills(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        create_skill_symlinks(current_dir, &self.target_dir)
    }

    fn clean_skills(&self, current_dir: &Path) -> Result<()> {
        remove_generated_skill_symlinks(current_dir, &self.target_dir)
    }

    fn check_skills(&self, current_dir: &Path) -> Result<bool> {
        check_skill_symlinks_in_sync(current_dir, &self.target_dir)
    }

    fn skills_gitignore_patterns(&self) -> Vec<String> {
        get_skill_gitignore_patterns(&self.target_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{AI_RULE_SOURCE_DIR, GENERATED_FILE_PREFIX, SKILLS_DIR, SKILL_FILENAME};
    use std::fs;
    use tempfile::TempDir;

    fn create_skill_folder(temp_dir: &Path, skill_name: &str, content: &str) -> PathBuf {
        let skill_dir = temp_dir
            .join(AI_RULE_SOURCE_DIR)
            .join(SKILLS_DIR)
            .join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join(SKILL_FILENAME), content).unwrap();
        skill_dir
    }

    #[test]
    fn test_external_skills_generator_target_dir() {
        let generator = ExternalSkillsGenerator::new(".claude/skills");
        assert_eq!(generator.skills_target_dir(), ".claude/skills");
    }

    #[test]
    fn test_external_skills_generator_generate() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalSkillsGenerator::new(".claude/skills");

        create_skill_folder(temp_dir.path(), "my-skill", "skill content");

        let result = generator.generate_skills(temp_dir.path());
        assert!(result.is_ok());

        let symlinks = result.unwrap();
        assert_eq!(symlinks.len(), 1);

        let symlink_path = temp_dir
            .path()
            .join(".claude/skills")
            .join(format!("{}my-skill", GENERATED_FILE_PREFIX));
        assert!(symlink_path.is_symlink());
    }

    #[test]
    fn test_external_skills_generator_clean() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalSkillsGenerator::new(".claude/skills");

        // Create skill and generate symlink
        create_skill_folder(temp_dir.path(), "my-skill", "skill content");
        generator.generate_skills(temp_dir.path()).unwrap();

        // Create user skill (real folder, not symlink)
        let user_skill = temp_dir.path().join(".claude/skills/user-skill");
        fs::create_dir_all(&user_skill).unwrap();
        fs::write(user_skill.join(SKILL_FILENAME), "user content").unwrap();

        // Clean
        generator.clean_skills(temp_dir.path()).unwrap();

        // Generated symlink should be gone
        let generated = temp_dir
            .path()
            .join(".claude/skills")
            .join(format!("{}my-skill", GENERATED_FILE_PREFIX));
        assert!(!generated.exists());

        // User skill should remain
        assert!(user_skill.exists());
    }

    #[test]
    fn test_external_skills_generator_check_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalSkillsGenerator::new(".claude/skills");

        // Create skill
        create_skill_folder(temp_dir.path(), "my-skill", "skill content");

        // Not in sync before generating
        let result = generator.check_skills(temp_dir.path()).unwrap();
        assert!(!result);

        // Generate symlinks
        generator.generate_skills(temp_dir.path()).unwrap();

        // Now in sync
        let result = generator.check_skills(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_external_skills_generator_check_no_skills() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalSkillsGenerator::new(".claude/skills");

        // No skills directory - should be in sync (nothing to do)
        let result = generator.check_skills(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_external_skills_generator_gitignore_patterns() {
        let generator = ExternalSkillsGenerator::new(".claude/skills");
        let patterns = generator.skills_gitignore_patterns();
        assert_eq!(patterns, vec![".claude/skills/ai-rules-generated-*/"]);
    }

    #[test]
    fn test_external_skills_generator_different_target_dirs() {
        // Test Claude target
        let claude_gen = ExternalSkillsGenerator::new(".claude/skills");
        assert_eq!(
            claude_gen.skills_gitignore_patterns(),
            vec![".claude/skills/ai-rules-generated-*/"]
        );

        // Test Codex target
        let codex_gen = ExternalSkillsGenerator::new(".codex/skills");
        assert_eq!(
            codex_gen.skills_gitignore_patterns(),
            vec![".codex/skills/ai-rules-generated-*/"]
        );

        // Test AMP target
        let amp_gen = ExternalSkillsGenerator::new(".agents/skills");
        assert_eq!(
            amp_gen.skills_gitignore_patterns(),
            vec![".agents/skills/ai-rules-generated-*/"]
        );
    }
}

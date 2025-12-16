use anyhow::Result;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub trait SkillsGeneratorTrait {
    fn skills_target_dir(&self) -> &str;
    fn generate_skills(&self, current_dir: &Path) -> Result<Vec<PathBuf>>;
    fn clean_skills(&self, current_dir: &Path) -> Result<()>;
    fn check_skills(&self, current_dir: &Path) -> Result<bool>;
    fn skills_gitignore_patterns(&self) -> Vec<String>;
}

use anyhow::Result;
use std::path::{Path, PathBuf};

pub trait SkillsGeneratorTrait {
    fn generate_skills(&self, current_dir: &Path) -> Result<Vec<PathBuf>>;
    fn clean_skills(&self, current_dir: &Path) -> Result<()>;
    fn check_skills(&self, current_dir: &Path) -> Result<bool>;
    fn skills_gitignore_patterns(&self) -> Vec<String>;
}

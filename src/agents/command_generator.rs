use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub trait CommandGeneratorTrait {
    /// Generate command files for this agent
    /// Returns HashMap of output path -> content
    fn generate_commands(&self, current_dir: &Path) -> HashMap<PathBuf, String>;

    /// Clean generated command files
    fn clean_commands(&self, current_dir: &Path) -> Result<()>;

    /// Check if command files are in sync
    #[allow(dead_code)]
    fn check_commands(&self, current_dir: &Path) -> Result<bool>;

    /// Get gitignore patterns for generated commands
    #[allow(dead_code)]
    fn command_gitignore_patterns(&self) -> Vec<String>;
}

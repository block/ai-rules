use anyhow::Result;
use std::path::{Path, PathBuf};

pub trait CommandGeneratorTrait {
    /// Generate command symlinks for this agent
    /// Returns Vec of created symlink paths
    fn generate_command_symlinks(&self, current_dir: &Path) -> Result<Vec<PathBuf>>;

    /// Clean generated command files/symlinks
    fn clean_commands(&self, current_dir: &Path) -> Result<()>;

    /// Check if command files/symlinks are in sync
    fn check_commands(&self, current_dir: &Path) -> Result<bool>;

    /// Get gitignore patterns for generated commands
    fn command_gitignore_patterns(&self) -> Vec<String>;
}

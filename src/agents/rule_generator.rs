use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::mcp_generator::McpGeneratorTrait;
use crate::models::SourceFile;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub trait AgentRuleGenerator {
    fn name(&self) -> &str;

    fn clean(&self, current_dir: &Path) -> Result<()>;

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String>;

    fn check_agent_contents(&self, source_files: &[SourceFile], current_dir: &Path)
        -> Result<bool>;

    fn check_symlink(&self, current_dir: &Path) -> Result<bool>;

    fn gitignore_patterns(&self) -> Vec<String>;

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>>;

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        None
    }

    fn command_generator(&self) -> Option<Box<dyn CommandGeneratorTrait>> {
        None
    }
}

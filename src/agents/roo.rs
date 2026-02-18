use crate::agents::mcp_generator::{ExternalMcpGenerator, McpGeneratorTrait};
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::SingleFileBasedGenerator;
use crate::constants::{AGENTS_MD_FILENAME, MCP_JSON};
use crate::models::SourceFile;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[allow(dead_code)] // Used in Phase 3 when integrated into registry
const ROO_DIR: &str = ".roo";

/// Roo generator that uses AGENTS.md (via SingleFileBasedGenerator) with MCP support
#[allow(dead_code)] // Used in Phase 3 when integrated into registry
pub struct RooGenerator {
    inner: SingleFileBasedGenerator,
}

#[allow(dead_code)] // Used in Phase 3 when integrated into registry
impl RooGenerator {
    pub fn new() -> Self {
        Self {
            inner: SingleFileBasedGenerator::new("roo", AGENTS_MD_FILENAME),
        }
    }
}

impl Default for RooGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRuleGenerator for RooGenerator {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        self.inner.clean(current_dir)
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        self.inner
            .generate_agent_contents(source_files, current_dir)
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        self.inner.check_agent_contents(source_files, current_dir)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        self.inner.check_symlink(current_dir)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        self.inner.gitignore_patterns()
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        self.inner.generate_symlink(current_dir)
    }

    fn uses_inlined_symlink(&self) -> bool {
        self.inner.uses_inlined_symlink()
    }

    fn generate_inlined_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        self.inner.generate_inlined_symlink(current_dir)
    }

    fn check_inlined_symlink(&self, current_dir: &Path) -> Result<bool> {
        self.inner.check_inlined_symlink(current_dir)
    }

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        Some(Box::new(ExternalMcpGenerator::new(
            PathBuf::from(ROO_DIR).join(MCP_JSON),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    const TEST_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-test"]
    }
  }
}"#;

    #[test]
    fn test_roo_generator_name() {
        let generator = RooGenerator::new();
        assert_eq!(generator.name(), "roo");
    }

    #[test]
    fn test_roo_generator_has_mcp_generator() {
        let generator = RooGenerator::new();
        assert!(generator.mcp_generator().is_some());
    }

    #[test]
    fn test_roo_generator_mcp_generates_to_roo_dir() {
        let temp_dir = TempDir::new().unwrap();
        let generator = RooGenerator::new();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let mcp_gen = generator.mcp_generator().unwrap();
        let files = mcp_gen.generate_mcp(temp_dir.path());

        assert_eq!(files.len(), 1);
        let expected_path = temp_dir.path().join(".roo/mcp.json");
        assert!(files.contains_key(&expected_path));
    }

    #[test]
    fn test_roo_generator_gitignore_patterns() {
        let generator = RooGenerator::new();
        let patterns = generator.gitignore_patterns();

        // Should return AGENTS.md pattern from SingleFileBasedGenerator
        assert_eq!(patterns, vec!["AGENTS.md"]);
    }
}

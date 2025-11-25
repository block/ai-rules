use crate::operations::mcp_reader::read_mcp_config;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub trait McpGeneratorTrait {
    fn generate_mcp(&self, current_dir: &Path) -> HashMap<PathBuf, String>;

    fn clean_mcp(&self, current_dir: &Path) -> Result<()>;

    fn check_mcp(&self, current_dir: &Path) -> Result<bool>;

    fn mcp_gitignore_patterns(&self) -> Vec<String>;
}

pub struct ExternalMcpGenerator {
    output_path: PathBuf,
}

impl ExternalMcpGenerator {
    pub fn new(output_path: PathBuf) -> Self {
        Self { output_path }
    }
}

impl McpGeneratorTrait for ExternalMcpGenerator {
    fn generate_mcp(&self, current_dir: &Path) -> HashMap<PathBuf, String> {
        let mut files = HashMap::new();

        if let Ok(Some(mcp_content)) = read_mcp_config(current_dir) {
            files.insert(current_dir.join(&self.output_path), mcp_content);
        }

        files
    }

    fn clean_mcp(&self, current_dir: &Path) -> Result<()> {
        let mcp_file = current_dir.join(&self.output_path);
        if mcp_file.exists() {
            fs::remove_file(mcp_file)?;
        }
        Ok(())
    }

    fn check_mcp(&self, current_dir: &Path) -> Result<bool> {
        let mcp_file = current_dir.join(&self.output_path);

        match read_mcp_config(current_dir)? {
            Some(expected) => {
                if !mcp_file.exists() {
                    return Ok(false);
                }
                let actual = fs::read_to_string(mcp_file)?;
                Ok(actual == expected)
            }
            None => Ok(!mcp_file.exists()),
        }
    }

    fn mcp_gitignore_patterns(&self) -> Vec<String> {
        vec![self.output_path.display().to_string()]
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
    fn test_external_mcp_generator_generate_with_source() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalMcpGenerator::new(PathBuf::from(".mcp.json"));

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let files = generator.generate_mcp(temp_dir.path());

        assert_eq!(files.len(), 1);
        let expected_path = temp_dir.path().join(".mcp.json");
        assert!(files.contains_key(&expected_path));
    }

    #[test]
    fn test_external_mcp_generator_generate_without_source() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalMcpGenerator::new(PathBuf::from(".mcp.json"));

        let files = generator.generate_mcp(temp_dir.path());

        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_external_mcp_generator_clean() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalMcpGenerator::new(PathBuf::from(".mcp.json"));

        create_file(temp_dir.path(), ".mcp.json", "test content");
        assert_file_exists(temp_dir.path(), ".mcp.json");

        let result = generator.clean_mcp(temp_dir.path());
        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".mcp.json");
    }

    #[test]
    fn test_external_mcp_generator_check_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalMcpGenerator::new(PathBuf::from(".mcp.json"));

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        let expected = read_mcp_config(temp_dir.path()).unwrap().unwrap();
        create_file(temp_dir.path(), ".mcp.json", &expected);

        let result = generator.check_mcp(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_external_mcp_generator_check_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ExternalMcpGenerator::new(PathBuf::from(".mcp.json"));

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        create_file(temp_dir.path(), ".mcp.json", "wrong content");

        let result = generator.check_mcp(temp_dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_external_mcp_generator_gitignore_patterns() {
        let generator = ExternalMcpGenerator::new(PathBuf::from(".cursor/mcp.json"));
        let patterns = generator.mcp_gitignore_patterns();

        assert_eq!(patterns, vec![".cursor/mcp.json"]);
    }
}

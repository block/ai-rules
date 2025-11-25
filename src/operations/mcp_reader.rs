use crate::constants::{AI_RULE_SOURCE_DIR, MCP_JSON, MCP_SERVERS_FIELD};
use crate::utils::file_utils::ensure_trailing_newline;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

fn read_mcp_source_file_content(current_dir: &Path) -> Result<Option<String>> {
    let mcp_source_path = current_dir.join(AI_RULE_SOURCE_DIR).join(MCP_JSON);

    if !mcp_source_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&mcp_source_path)
        .with_context(|| format!("Failed to read {}", mcp_source_path.display()))?;

    let _config: McpConfig = serde_json::from_str(&content)
        .with_context(|| format!("Invalid MCP configuration in {}", mcp_source_path.display()))?;

    Ok(Some(content))
}

pub fn read_mcp_config(current_dir: &Path) -> Result<Option<String>> {
    match read_mcp_source_file_content(current_dir)? {
        Some(content) => Ok(Some(ensure_trailing_newline(content))),
        None => Ok(None),
    }
}

pub fn extract_mcp_servers_for_firebender(current_dir: &Path) -> Result<Option<Value>> {
    match read_mcp_source_file_content(current_dir)? {
        Some(content) => {
            let json: Value = serde_json::from_str(&content)?;
            Ok(json.get(MCP_SERVERS_FIELD).cloned())
        }
        None => Ok(None),
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
    fn test_read_mcp_config_valid() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let result = read_mcp_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("mcpServers"));
        assert!(content.ends_with('\n'));
    }

    #[test]
    fn test_read_mcp_config_not_exists() {
        let temp_dir = TempDir::new().unwrap();

        let result = read_mcp_config(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_read_mcp_config_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/mcp.json", "{ invalid json");

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_mcp_servers_for_firebender() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let result = extract_mcp_servers_for_firebender(temp_dir.path()).unwrap();
        assert!(result.is_some());

        let servers = result.unwrap();
        assert!(servers.is_object());
        assert!(servers.get("test-server").is_some());
    }

    #[test]
    fn test_extract_mcp_servers_not_exists() {
        let temp_dir = TempDir::new().unwrap();

        let result = extract_mcp_servers_for_firebender(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_mcp_config_missing_mcpservers() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_config = r#"{"servers": {}}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", invalid_config);

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mcp_config_mcpservers_not_object() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_config = r#"{"mcpServers": "not an object"}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", invalid_config);

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mcp_config_missing_command() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_config = r#"{
  "mcpServers": {
    "test-server": {
      "args": ["-y", "test"]
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", invalid_config);

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mcp_config_command_not_string() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_config = r#"{
  "mcpServers": {
    "test-server": {
      "command": 123
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", invalid_config);

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mcp_config_args_not_array() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_config = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "args": "not an array"
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", invalid_config);

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mcp_config_env_not_object() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_config = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "env": "not an object"
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", invalid_config);

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mcp_config_valid_with_all_fields() {
        let temp_dir = TempDir::new().unwrap();
        let valid_config = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-test"],
      "env": {
        "API_KEY": "${API_KEY}",
        "NODE_ENV": "production",
        "PORT": "3000"
      }
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", valid_config);

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }
}

use crate::agents::mcp_generator::McpGeneratorTrait;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::{
    check_in_sync, clean_generated_files, generate_agent_file_contents,
};
use crate::models::SourceFile;
use crate::operations::mcp_reader::read_mcp_config;
use crate::utils::file_utils::{check_agents_md_symlink, create_symlink_to_agents_md};
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const GEMINI_SETTINGS_JSON: &str = ".gemini/settings.json";
const GEMINI_AGENT_FILE: &str = "GEMINI.md";

pub struct GeminiGenerator;

impl AgentRuleGenerator for GeminiGenerator {
    fn name(&self) -> &str {
        "gemini"
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        clean_generated_files(current_dir, GEMINI_AGENT_FILE)?;
        if let Some(mcp) = self.mcp_generator() {
            mcp.clean_mcp(current_dir)?;
        }
        Ok(())
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        generate_agent_file_contents(source_files, current_dir, GEMINI_AGENT_FILE)
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        check_in_sync(source_files, current_dir, GEMINI_AGENT_FILE)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(GEMINI_AGENT_FILE);
        check_agents_md_symlink(current_dir, &output_file)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        let mut patterns = vec![GEMINI_AGENT_FILE.to_string()];
        if let Some(mcp) = self.mcp_generator() {
            patterns.extend(mcp.mcp_gitignore_patterns());
        }
        patterns
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_agents_md(current_dir, Path::new(GEMINI_AGENT_FILE))?;
        if success {
            Ok(vec![current_dir.join(GEMINI_AGENT_FILE)])
        } else {
            Ok(vec![])
        }
    }

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        Some(Box::new(GeminiMcpGenerator))
    }
}

struct GeminiMcpGenerator;

impl McpGeneratorTrait for GeminiMcpGenerator {
    fn generate_mcp(&self, current_dir: &Path) -> HashMap<PathBuf, String> {
        let mut files = HashMap::new();

        // 1. Read source MCP config (ai-rules/mcp.json)
        let source_mcp_content = match read_mcp_config(current_dir) {
            Ok(Some(c)) => c,
            _ => return files, // No source config, nothing to generate
        };
        let source_json: Value = serde_json::from_str(&source_mcp_content).unwrap_or(json!({}));
        let mut source_servers = source_json.get("mcpServers").unwrap_or(&json!({})).clone();

        // Apply Gemini-specific transformations
        self.transform_mcp_servers(&mut source_servers);

        // 2. Read existing target config (.gemini/settings.json)
        let target_path = current_dir.join(GEMINI_SETTINGS_JSON);
        let mut target_json = if target_path.exists() {
            let content = fs::read_to_string(&target_path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };

        // 3. Update mcpServers
        // We overwrite mcpServers with the transformed source configuration
        target_json["mcpServers"] = source_servers;

        // 4. Format and return
        // Use pretty print
        if let Ok(content) = serde_json::to_string_pretty(&target_json) {
            files.insert(target_path, content);
        }

        files
    }

    fn clean_mcp(&self, current_dir: &Path) -> Result<()> {
        let target_path = current_dir.join(GEMINI_SETTINGS_JSON);
        if target_path.exists() {
            let content = fs::read_to_string(&target_path)?;
            let mut json: Value = serde_json::from_str(&content)?;

            if let Some(obj) = json.as_object_mut() {
                obj.remove("mcpServers");

                // For safety, write it back without mcpServers
                let new_content = serde_json::to_string_pretty(&json)?;
                fs::write(&target_path, new_content)?;
            }
        }
        Ok(())
    }

    fn check_mcp(&self, current_dir: &Path) -> Result<bool> {
        let target_path = current_dir.join(GEMINI_SETTINGS_JSON);

        let source_mcp_content = match read_mcp_config(current_dir)? {
            Some(c) => c,
            None => {
                // If no source, assume sync
                return Ok(true);
            }
        };

        if !target_path.exists() {
            return Ok(false);
        }

        let source_json: Value = serde_json::from_str(&source_mcp_content)?;
        let empty_obj = json!({});
        let mut source_servers = source_json.get("mcpServers").unwrap_or(&empty_obj).clone();
        // Transform source before comparison
        self.transform_mcp_servers(&mut source_servers);

        let target_content = fs::read_to_string(&target_path)?;
        let target_json: Value = serde_json::from_str(&target_content)?;
        let target_servers = target_json.get("mcpServers").unwrap_or(&empty_obj);

        Ok(&source_servers == target_servers)
    }

    fn mcp_gitignore_patterns(&self) -> Vec<String> {
        vec![GEMINI_SETTINGS_JSON.to_string()]
    }

    fn box_clone(&self) -> Box<dyn McpGeneratorTrait> {
        Box::new(Self)
    }
}

impl GeminiMcpGenerator {
    fn transform_mcp_servers(&self, servers: &mut Value) {
        if let Some(servers_obj) = servers.as_object_mut() {
            for (_, server_config) in servers_obj.iter_mut() {
                if let Some(server_obj) = server_config.as_object_mut() {
                    // Always remove "type" for Gemini
                    server_obj.remove("type");

                    // If httpUrl is present, remove command and url
                    if server_obj.contains_key("httpUrl") {
                        server_obj.remove("command");
                        server_obj.remove("url");
                    } else if let Some(url_value) = server_obj.get("url") {
                        if let Some(url_str) = url_value.as_str() {
                            if url_str.ends_with("/mcp") {
                                server_obj.insert("httpUrl".to_string(), url_value.clone());
                                server_obj.remove("url");
                                server_obj.remove("command");
                            } else {
                                // If url is present and doesn't end with "/mcp", remove command
                                server_obj.remove("command");
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_gemini_mcp_transformation() {
        let mut servers = json!({
            "jira": {
                "command": "",
                "type": "http",
                "url": "https://mcp.atlassian.com/v1/sse"
            },
            "notion-block": {
                "command": "",
                "type": "http",
                "url": "https://mcp.notion.com/mcp"
            },
            "notion-square": {
                "command": "",
                "type": "http",
                "httpUrl": "https://mcp.notion.com/mcp"
            },
            "stdio-server": {
                "command": "npx",
                "type": "stdio",
                "args": ["-y", "server"]
            }
        });

        let generator = GeminiMcpGenerator;
        generator.transform_mcp_servers(&mut servers);

        let servers_obj = servers.as_object().unwrap();

        // Jira: url present (not ending in /mcp) -> remove command, type, keep url
        let jira = servers_obj.get("jira").unwrap();
        assert!(jira.get("url").is_some());
        assert!(jira.get("command").is_none());
        assert!(jira.get("type").is_none());

        // Notion-block: url present (ending in /mcp) -> convert to httpUrl, remove command, type
        let notion_block = servers_obj.get("notion-block").unwrap();
        assert!(notion_block.get("httpUrl").is_some());
        assert!(notion_block.get("url").is_none());
        assert!(notion_block.get("command").is_none());
        assert!(notion_block.get("type").is_none());

        // Notion-square: httpUrl already present -> remove command, url, type
        let notion_square = servers_obj.get("notion-square").unwrap();
        assert!(notion_square.get("httpUrl").is_some());
        assert!(notion_square.get("url").is_none());
        assert!(notion_square.get("command").is_none());
        assert!(notion_square.get("type").is_none());

        // Stdio: no url/httpUrl -> keep command, remove type
        let stdio = servers_obj.get("stdio-server").unwrap();
        assert!(stdio.get("command").is_some());
        assert!(stdio.get("args").is_some());
        assert!(stdio.get("type").is_none());
    }

    #[test]
    fn test_gemini_check_mcp_in_sync_after_transform() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // Create source config that needs transformation
        let source_config = r#"{
  "mcpServers": {
    "jira": {
      "command": "",
      "type": "http",
      "url": "https://mcp.atlassian.com/v1/sse"
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", source_config);

        // Create target config that is already transformed
        let target_config = r#"{
  "mcpServers": {
    "jira": {
      "url": "https://mcp.atlassian.com/v1/sse"
    }
  }
}"#;
        create_file(temp_dir.path(), ".gemini/settings.json", target_config);

        // Check should pass because source is transformed before comparison
        let result = generator.check_mcp(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_gemini_generator_gitignore_patterns() {
        let generator = GeminiGenerator;
        let patterns = generator.gitignore_patterns();

        assert!(patterns.contains(&"GEMINI.md".to_string()));
        assert!(patterns.contains(&".gemini/settings.json".to_string()));
    }

    #[test]
    fn test_gemini_generator_name() {
        let generator = GeminiGenerator;
        assert_eq!(generator.name(), "gemini");
    }
}

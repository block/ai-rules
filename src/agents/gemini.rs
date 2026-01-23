use crate::agents::mcp_generator::McpGeneratorTrait;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::{
    check_in_sync, clean_generated_files, generate_agent_file_contents,
};
use crate::constants::GENERATED_FILE_PREFIX;
use crate::models::source_file::filter_source_files_for_agent;
use crate::models::SourceFile;
use crate::operations::mcp_reader::read_mcp_config;
use crate::utils::file_utils::{check_agents_md_symlink, create_symlink_to_agents_md};
use anyhow::Result;
use serde_json::{json, Map, Value};
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
        let filtered_source_files = filter_source_files_for_agent(source_files, self.name());
        generate_agent_file_contents(
            &filtered_source_files,
            current_dir,
            GEMINI_AGENT_FILE,
            self.name(),
        )
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        let filtered_source_files = filter_source_files_for_agent(source_files, self.name());
        check_in_sync(
            &filtered_source_files,
            current_dir,
            GEMINI_AGENT_FILE,
            self.name(),
        )
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

        // Prefix source server names with GENERATED_FILE_PREFIX
        let prefixed_servers = self.prefix_server_names(&source_servers);

        // 2. Read existing target config (.gemini/settings.json)
        let target_path = current_dir.join(GEMINI_SETTINGS_JSON);
        let mut target_json = if target_path.exists() {
            let content = fs::read_to_string(&target_path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };

        // 3. Merge mcpServers: preserve user servers, replace generated ones
        let existing_servers = target_json
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // Keep only user-configured servers (those without the prefix)
        let mut merged_servers: Map<String, Value> = existing_servers
            .into_iter()
            .filter(|(name, _)| !name.starts_with(GENERATED_FILE_PREFIX))
            .collect();

        // Add new prefixed servers
        if let Some(prefixed_obj) = prefixed_servers.as_object() {
            for (name, config) in prefixed_obj {
                merged_servers.insert(name.clone(), config.clone());
            }
        }

        target_json["mcpServers"] = Value::Object(merged_servers);

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
                // Only remove servers with the generated prefix, preserve user servers
                if let Some(mcp_servers) = obj.get_mut("mcpServers") {
                    if let Some(servers_obj) = mcp_servers.as_object_mut() {
                        servers_obj.retain(|name, _| !name.starts_with(GENERATED_FILE_PREFIX));
                    }
                }

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
                // If no source, check target doesn't have any generated (prefixed) servers
                if !target_path.exists() {
                    return Ok(true);
                }
                let target_content = fs::read_to_string(&target_path)?;
                let target_json: Value = serde_json::from_str(&target_content)?;
                let mcp_servers = target_json.get("mcpServers");
                let has_no_generated = match mcp_servers {
                    None => true,
                    Some(val) => val
                        .as_object()
                        .is_none_or(|o| !o.keys().any(|k| k.starts_with(GENERATED_FILE_PREFIX))),
                };
                return Ok(has_no_generated);
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
        // Prefix source server names for comparison
        let expected_servers = self.prefix_server_names(&source_servers);

        let target_content = fs::read_to_string(&target_path)?;
        let target_json: Value = serde_json::from_str(&target_content)?;

        // Extract only generated (prefixed) servers from target for comparison
        let target_generated_servers: Map<String, Value> = target_json
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter(|(name, _)| name.starts_with(GENERATED_FILE_PREFIX))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let expected_obj = expected_servers.as_object().cloned().unwrap_or_default();
        Ok(target_generated_servers == expected_obj)
    }

    fn mcp_gitignore_patterns(&self) -> Vec<String> {
        vec![GEMINI_SETTINGS_JSON.to_string()]
    }

    fn box_clone(&self) -> Box<dyn McpGeneratorTrait> {
        Box::new(Self)
    }
}

impl GeminiMcpGenerator {
    /// Prefixes all server names with GENERATED_FILE_PREFIX
    fn prefix_server_names(&self, servers: &Value) -> Value {
        if let Some(servers_obj) = servers.as_object() {
            let prefixed: Map<String, Value> = servers_obj
                .iter()
                .map(|(name, config)| {
                    let prefixed_name = format!("{}{}", GENERATED_FILE_PREFIX, name);
                    (prefixed_name, config.clone())
                })
                .collect();
            Value::Object(prefixed)
        } else {
            json!({})
        }
    }

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

        // Create target config that is already transformed AND prefixed
        let target_config = r#"{
  "mcpServers": {
    "ai-rules-generated-jira": {
      "url": "https://mcp.atlassian.com/v1/sse"
    }
  }
}"#;
        create_file(temp_dir.path(), ".gemini/settings.json", target_config);

        // Check should pass because source is transformed and prefixed before comparison
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

    #[test]
    fn test_generate_mcp_preserves_user_servers_and_replaces_generated() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // Create source MCP config (command field is required)
        let source_config =
            r#"{"mcpServers": {"new-server": {"command": "npx", "args": ["-y", "test"]}}}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", source_config);

        // Create existing target with user servers, old generated servers, and other settings
        let existing_target = r#"{
  "mcpServers": {
    "user-server": {"url": "http://user.example.com"},
    "ai-rules-generated-old-server": {"url": "http://old.example.com"}
  },
  "otherSetting": "preserved",
  "nestedSetting": {
    "key": "value"
  }
}"#;
        create_file(temp_dir.path(), ".gemini/settings.json", existing_target);

        // Generate MCP
        let files = generator.generate_mcp(temp_dir.path());

        // Verify the result - get the first (and only) file
        assert_eq!(files.len(), 1);
        let content = files.values().next().unwrap();
        let json: Value = serde_json::from_str(content).unwrap();

        // Other settings should be preserved
        assert_eq!(json.get("otherSetting").unwrap(), "preserved");
        assert_eq!(
            json.get("nestedSetting").unwrap().get("key").unwrap(),
            "value"
        );

        // mcpServers should exist
        let mcp_servers = json.get("mcpServers").unwrap().as_object().unwrap();

        // User server should be preserved
        assert!(mcp_servers.contains_key("user-server"));

        // Old generated server should be removed
        assert!(!mcp_servers.contains_key("ai-rules-generated-old-server"));

        // New server should be added with prefix
        assert!(mcp_servers.contains_key("ai-rules-generated-new-server"));
    }

    #[test]
    fn test_generate_mcp_invalid_source_json() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // Create invalid JSON source
        create_file(temp_dir.path(), "ai-rules/mcp.json", "{ invalid json }");

        // Should return empty map - read_mcp_config returns Err for invalid JSON,
        // which is caught by the wildcard pattern and returns empty files
        let files = generator.generate_mcp(temp_dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn test_clean_mcp_only_removes_prefixed_servers() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // Create target with both user and generated servers, plus other settings
        let target_config = r#"{
  "mcpServers": {
    "user-server": {"url": "http://user.example.com"},
    "ai-rules-generated-test": {"url": "http://generated.example.com"}
  },
  "otherSetting": "preserved",
  "anotherSetting": 42
}"#;
        create_file(temp_dir.path(), ".gemini/settings.json", target_config);

        // Clean MCP
        generator.clean_mcp(temp_dir.path()).unwrap();

        // Verify the result
        let target_path = temp_dir.path().join(".gemini/settings.json");
        let content = fs::read_to_string(&target_path).unwrap();
        let json: Value = serde_json::from_str(&content).unwrap();

        // mcpServers should still exist with user servers
        let mcp_servers = json.get("mcpServers").unwrap().as_object().unwrap();
        assert!(mcp_servers.contains_key("user-server"));
        assert!(!mcp_servers.contains_key("ai-rules-generated-test"));
        // Other settings should be preserved
        assert_eq!(json.get("otherSetting").unwrap(), "preserved");
        assert_eq!(json.get("anotherSetting").unwrap(), 42);
    }

    #[test]
    fn test_clean_mcp_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // Should not error when file doesn't exist
        let result = generator.clean_mcp(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_clean_mcp_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // Create invalid JSON target
        create_file(temp_dir.path(), ".gemini/settings.json", "{ invalid json }");

        // Should return error for invalid JSON
        let result = generator.clean_mcp(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_check_mcp_no_source_with_user_servers_is_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // No source config (ai-rules/mcp.json doesn't exist)

        // Target with user-configured mcpServers (no prefix)
        let target_config = r#"{
  "mcpServers": {
    "user-server": {"url": "http://example.com"}
  }
}"#;
        create_file(temp_dir.path(), ".gemini/settings.json", target_config);

        // Should report in sync (true) because user servers are allowed without source
        let result = generator.check_mcp(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_mcp_no_source_with_generated_servers_is_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // No source config (ai-rules/mcp.json doesn't exist)

        // Target with generated (prefixed) mcpServers
        let target_config = r#"{
  "mcpServers": {
    "ai-rules-generated-test": {"url": "http://example.com"}
  }
}"#;
        create_file(temp_dir.path(), ".gemini/settings.json", target_config);

        // Should report out of sync (false) because target has generated servers but no source
        let result = generator.check_mcp(temp_dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_mcp_no_source_no_target() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // No source config, no target file
        // Should report in sync (true)
        let result = generator.check_mcp(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_mcp_no_source_target_without_mcpservers() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // No source config

        // Target exists but has no mcpServers
        let target_config = r#"{"otherSetting": "value"}"#;
        create_file(temp_dir.path(), ".gemini/settings.json", target_config);

        // Should report in sync (true) because target has no mcpServers
        let result = generator.check_mcp(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_mcp_no_source_target_with_empty_mcpservers() {
        let temp_dir = TempDir::new().unwrap();
        let generator = GeminiMcpGenerator;

        // No source config

        // Target exists with empty mcpServers
        let target_config = r#"{"mcpServers": {}}"#;
        create_file(temp_dir.path(), ".gemini/settings.json", target_config);

        // Should report in sync (true) because mcpServers is empty
        let result = generator.check_mcp(temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_prefix_server_names() {
        let generator = GeminiMcpGenerator;
        let servers = json!({
            "server1": {"command": "npx"},
            "server2": {"url": "http://example.com"}
        });

        let prefixed = generator.prefix_server_names(&servers);
        let prefixed_obj = prefixed.as_object().unwrap();

        // Original names should not exist
        assert!(!prefixed_obj.contains_key("server1"));
        assert!(!prefixed_obj.contains_key("server2"));

        // Prefixed names should exist
        assert!(prefixed_obj.contains_key("ai-rules-generated-server1"));
        assert!(prefixed_obj.contains_key("ai-rules-generated-server2"));

        // Values should be preserved
        assert_eq!(
            prefixed_obj
                .get("ai-rules-generated-server1")
                .unwrap()
                .get("command")
                .unwrap(),
            "npx"
        );
    }
}

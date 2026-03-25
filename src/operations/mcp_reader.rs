use crate::constants::{AI_RULE_SOURCE_DIR, ENV_AGENTS_KEY, MCP_ENV_FILENAME, MCP_JSON, MCP_SERVERS_FIELD};
use crate::utils::file_utils::ensure_trailing_newline;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum McpServerType {
    Http,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum McpServerConfig {
    Command {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
    },
    Http {
        #[serde(rename = "type")]
        server_type: McpServerType,
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

fn load_dot_env(current_dir: &Path) -> HashMap<String, String> {
    let env_path = current_dir
        .join(AI_RULE_SOURCE_DIR)
        .join(MCP_ENV_FILENAME);
    let mut vars = HashMap::new();

    let content = match fs::read_to_string(&env_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return vars,
        Err(e) => {
            eprintln!("Warning: failed to read {}: {e}", env_path.display());
            return vars;
        }
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            let value = strip_surrounding_quotes(value);
            if !key.is_empty() {
                vars.insert(key.to_string(), value.to_string());
            }
        }
    }
    vars
}

pub fn read_default_agents_from_env(current_dir: &Path) -> Option<Vec<String>> {
    let dot_env = load_dot_env(current_dir);
    dot_env.get(ENV_AGENTS_KEY).map(|value| {
        value
            .split(',')
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty())
            .collect()
    })
}

fn strip_surrounding_quotes(s: &str) -> &str {
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"'))
            || (s.starts_with('\'') && s.ends_with('\'')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn expand_env_vars(s: &str, dot_env: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let mut var_name = String::new();
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == '}' {
                    closed = true;
                    break;
                }
                var_name.push(inner);
            }
            if closed {
                if let Ok(val) = env::var(&var_name) {
                    result.push_str(&val);
                } else if let Some(val) = dot_env.get(&var_name) {
                    result.push_str(val);
                } else {
                    eprintln!("Warning: environment variable ${{{var_name}}} is not set");
                    result.push('$');
                    result.push('{');
                    result.push_str(&var_name);
                    result.push('}');
                }
            } else {
                result.push('$');
                result.push('{');
                result.push_str(&var_name);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn substitute_env_vars(value: &mut Value, dot_env: &HashMap<String, String>) {
    match value {
        Value::String(s) => *s = expand_env_vars(s, dot_env),
        Value::Array(arr) => {
            for item in arr {
                substitute_env_vars(item, dot_env);
            }
        }
        Value::Object(map) => {
            for val in map.values_mut() {
                substitute_env_vars(val, dot_env);
            }
        }
        _ => {}
    }
}

fn read_mcp_source_file_content(current_dir: &Path) -> Result<Option<String>> {
    let mcp_source_path = current_dir.join(AI_RULE_SOURCE_DIR).join(MCP_JSON);

    if !mcp_source_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&mcp_source_path)
        .with_context(|| format!("Failed to read {}", mcp_source_path.display()))?;

    let mut json: Value = serde_json::from_str(&content)
        .with_context(|| format!("Invalid MCP configuration in {}", mcp_source_path.display()))?;

    let _: McpConfig = serde_json::from_value(json.clone())
        .with_context(|| format!("Invalid MCP configuration in {}", mcp_source_path.display()))?;

    let dot_env = load_dot_env(current_dir);
    substitute_env_vars(&mut json, &dot_env);

    Ok(Some(serde_json::to_string_pretty(&json)?))
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
    use std::env;
    use tempfile::TempDir;

    const TEST_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-test"]
    }
  }
}"#;

    const TEST_HTTP_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    "figma": {
      "type": "http",
      "url": "https://mcp.figma.com/mcp"
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

    #[test]
    fn test_read_mcp_config_http_server() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_HTTP_MCP_CONFIG);

        let result = read_mcp_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("mcpServers"));
        assert!(content.contains("figma"));
        assert!(content.ends_with('\n'));
    }

    #[test]
    fn test_extract_mcp_servers_for_firebender_http() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_HTTP_MCP_CONFIG);

        let result = extract_mcp_servers_for_firebender(temp_dir.path()).unwrap();
        assert!(result.is_some());

        let servers = result.unwrap();
        assert!(servers.is_object());
        assert!(servers.get("figma").is_some());

        let figma_server = servers.get("figma").unwrap();
        assert_eq!(
            figma_server.get("url").unwrap().as_str().unwrap(),
            "https://mcp.figma.com/mcp"
        );
    }

    #[test]
    fn test_read_mcp_config_http_with_headers() {
        let temp_dir = TempDir::new().unwrap();
        let config_with_headers = r#"{
  "mcpServers": {
    "api-server": {
      "type": "http",
      "url": "https://api.example.com/mcp",
      "headers": {
        "Authorization": "Bearer token123",
        "X-Custom-Header": "custom-value"
      }
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", config_with_headers);

        let result = read_mcp_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("Authorization"));
        assert!(content.contains("Bearer token123"));
    }

    #[test]
    fn test_read_mcp_config_mixed_command_and_http() {
        let temp_dir = TempDir::new().unwrap();
        let mixed_config = r#"{
  "mcpServers": {
    "local-server": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-test"]
    },
    "remote-server": {
      "type": "http",
      "url": "https://remote.example.com/mcp"
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", mixed_config);

        let result = read_mcp_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let content = result.unwrap();
        assert!(content.contains("local-server"));
        assert!(content.contains("remote-server"));
        assert!(content.contains("npx"));
        assert!(content.contains("https://remote.example.com/mcp"));
    }

    #[test]
    fn test_expand_env_vars_substitutes_set_variable() {
        unsafe { env::set_var("AI_RULES_TEST_EXPAND_1", "secret123") };
        let result = expand_env_vars("Bearer ${AI_RULES_TEST_EXPAND_1}", &HashMap::new());
        unsafe { env::remove_var("AI_RULES_TEST_EXPAND_1") };
        assert_eq!(result, "Bearer secret123");
    }

    #[test]
    fn test_expand_env_vars_leaves_unset_variable_as_placeholder() {
        unsafe { env::remove_var("AI_RULES_TEST_UNSET_XYZ_999") };
        let result = expand_env_vars("${AI_RULES_TEST_UNSET_XYZ_999}", &HashMap::new());
        assert_eq!(result, "${AI_RULES_TEST_UNSET_XYZ_999}");
    }

    #[test]
    fn test_expand_env_vars_passes_through_plain_strings() {
        let result = expand_env_vars("no substitution needed", &HashMap::new());
        assert_eq!(result, "no substitution needed");
    }

    #[test]
    fn test_expand_env_vars_uses_dot_env_when_shell_var_unset() {
        unsafe { env::remove_var("AI_RULES_TEST_DOT_ENV_VAR") };
        let mut dot_env = HashMap::new();
        dot_env.insert("AI_RULES_TEST_DOT_ENV_VAR".to_string(), "from-dot-env".to_string());
        let result = expand_env_vars("${AI_RULES_TEST_DOT_ENV_VAR}", &dot_env);
        assert_eq!(result, "from-dot-env");
    }

    #[test]
    fn test_expand_env_vars_shell_var_takes_priority_over_dot_env() {
        unsafe { env::set_var("AI_RULES_TEST_PRIORITY_VAR", "from-shell") };
        let mut dot_env = HashMap::new();
        dot_env.insert("AI_RULES_TEST_PRIORITY_VAR".to_string(), "from-dot-env".to_string());
        let result = expand_env_vars("${AI_RULES_TEST_PRIORITY_VAR}", &dot_env);
        unsafe { env::remove_var("AI_RULES_TEST_PRIORITY_VAR") };
        assert_eq!(result, "from-shell");
    }

    #[test]
    fn test_load_dot_env_parses_key_value_pairs() {
        let temp_dir = TempDir::new().unwrap();
        create_file(
            temp_dir.path(),
            "ai-rules/.env",
            "API_KEY=secret\nOTHER_KEY=value\n",
        );
        let vars = load_dot_env(temp_dir.path());
        assert_eq!(vars.get("API_KEY").unwrap(), "secret");
        assert_eq!(vars.get("OTHER_KEY").unwrap(), "value");
    }

    #[test]
    fn test_load_dot_env_skips_comments_and_blank_lines() {
        let temp_dir = TempDir::new().unwrap();
        create_file(
            temp_dir.path(),
            "ai-rules/.env",
            "# comment\n\nAPI_KEY=secret\n",
        );
        let vars = load_dot_env(temp_dir.path());
        assert_eq!(vars.len(), 1);
        assert_eq!(vars.get("API_KEY").unwrap(), "secret");
    }

    #[test]
    fn test_load_dot_env_strips_surrounding_quotes() {
        let temp_dir = TempDir::new().unwrap();
        create_file(
            temp_dir.path(),
            "ai-rules/.env",
            "DOUBLE=\"quoted value\"\nSINGLE='also quoted'\n",
        );
        let vars = load_dot_env(temp_dir.path());
        assert_eq!(vars.get("DOUBLE").unwrap(), "quoted value");
        assert_eq!(vars.get("SINGLE").unwrap(), "also quoted");
    }

    #[test]
    fn test_load_dot_env_handles_value_containing_equals_sign() {
        let temp_dir = TempDir::new().unwrap();
        create_file(
            temp_dir.path(),
            "ai-rules/.env",
            "API_URL=https://example.com/path?foo=bar\n",
        );
        let vars = load_dot_env(temp_dir.path());
        assert_eq!(vars.get("API_URL").unwrap(), "https://example.com/path?foo=bar");
    }

    #[test]
    fn test_load_dot_env_returns_empty_when_file_missing() {
        let temp_dir = TempDir::new().unwrap();
        let vars = load_dot_env(temp_dir.path());
        assert!(vars.is_empty());
    }

    #[test]
    fn test_read_mcp_config_substitutes_vars_from_dot_env_file() {
        let temp_dir = TempDir::new().unwrap();
        unsafe { env::remove_var("AI_RULES_TEST_DOT_ENV_KEY") };
        create_file(temp_dir.path(), "ai-rules/.env", "AI_RULES_TEST_DOT_ENV_KEY=from-dot-env\n");
        let config = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "env": {
        "API_KEY": "${AI_RULES_TEST_DOT_ENV_KEY}"
      }
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", config);

        let result = read_mcp_config(temp_dir.path()).unwrap().unwrap();
        assert!(result.contains("from-dot-env"));
        assert!(!result.contains("${AI_RULES_TEST_DOT_ENV_KEY}"));
    }

    #[test]
    fn test_read_mcp_config_substitutes_env_vars_in_output() {
        let temp_dir = TempDir::new().unwrap();
        unsafe { env::set_var("AI_RULES_TEST_API_KEY", "my-api-key") };
        let config = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "env": {
        "API_KEY": "${AI_RULES_TEST_API_KEY}"
      }
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", config);

        let result = read_mcp_config(temp_dir.path());
        unsafe { env::remove_var("AI_RULES_TEST_API_KEY") };
        let content = result.unwrap().unwrap();

        assert!(content.contains("my-api-key"));
        assert!(!content.contains("${AI_RULES_TEST_API_KEY}"));
    }

    #[test]
    fn test_read_mcp_config_leaves_unset_env_vars_as_placeholder() {
        let temp_dir = TempDir::new().unwrap();
        unsafe { env::remove_var("AI_RULES_TEST_MISSING_KEY_999") };
        let config = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "env": {
        "API_KEY": "${AI_RULES_TEST_MISSING_KEY_999}"
      }
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", config);

        let result = read_mcp_config(temp_dir.path()).unwrap().unwrap();
        assert!(result.contains("${AI_RULES_TEST_MISSING_KEY_999}"));
    }

    #[test]
    fn test_read_mcp_config_invalid_type() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_config = r#"{
  "mcpServers": {
    "bad-server": {
      "type": "invalid-type",
      "url": "https://example.com/mcp"
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/mcp.json", invalid_config);

        let result = read_mcp_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_default_agents_from_env_returns_agents() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/.env", "AI_RULES_AGENTS=claude,cursor\n");

        let result = read_default_agents_from_env(temp_dir.path());
        assert_eq!(result, Some(vec!["claude".to_string(), "cursor".to_string()]));
    }

    #[test]
    fn test_read_default_agents_from_env_trims_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/.env", "AI_RULES_AGENTS= claude , cursor \n");

        let result = read_default_agents_from_env(temp_dir.path());
        assert_eq!(result, Some(vec!["claude".to_string(), "cursor".to_string()]));
    }

    #[test]
    fn test_read_default_agents_from_env_returns_none_when_key_absent() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/.env", "OTHER_KEY=value\n");

        let result = read_default_agents_from_env(temp_dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_read_default_agents_from_env_returns_none_when_no_env_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = read_default_agents_from_env(temp_dir.path());
        assert!(result.is_none());
    }
}

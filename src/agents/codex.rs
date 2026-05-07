use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::mcp_generator::McpGeneratorTrait;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::{
    check_in_sync, clean_generated_files, generate_agent_file_contents,
};
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR, CODEX_SKILLS_DIR};
use crate::models::SourceFile;
use crate::operations::mcp_reader::{read_mcp_config, McpConfig, McpServerConfig};
use crate::utils::file_utils::{
    check_agents_md_symlink, check_inlined_file_symlink, create_symlink_to_agents_md,
    create_symlink_to_inlined_file, ensure_trailing_newline,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::{Table, Value};

const CODEX_CONFIG_TOML: &str = ".codex/config.toml";
const CODEX_CONFIG_OVERLAY_TOML: &str = "codex-config.toml";

pub struct CodexGenerator {
    name: String,
    output_filename: String,
}

impl CodexGenerator {
    pub fn new() -> Self {
        Self {
            name: "codex".to_string(),
            output_filename: AGENTS_MD_FILENAME.to_string(),
        }
    }
}

impl Default for CodexGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRuleGenerator for CodexGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        clean_generated_files(current_dir, &self.output_filename)
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        generate_agent_file_contents(source_files, current_dir, &self.output_filename)
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        check_in_sync(source_files, current_dir, &self.output_filename)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(&self.output_filename);
        check_agents_md_symlink(current_dir, &output_file)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        vec![self.output_filename.clone()]
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_agents_md(current_dir, Path::new(&self.output_filename))?;
        if success {
            Ok(vec![current_dir.join(&self.output_filename)])
        } else {
            Ok(vec![])
        }
    }

    fn uses_inlined_symlink(&self) -> bool {
        true
    }

    fn generate_inlined_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success =
            create_symlink_to_inlined_file(current_dir, Path::new(&self.output_filename))?;
        if success {
            Ok(vec![current_dir.join(&self.output_filename)])
        } else {
            Ok(vec![])
        }
    }

    fn check_inlined_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(&self.output_filename);
        check_inlined_file_symlink(current_dir, &output_file)
    }

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        Some(Box::new(CodexMcpGenerator))
    }

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        Some(Box::new(ExternalSkillsGenerator::new(CODEX_SKILLS_DIR)))
    }
}

struct CodexMcpGenerator;

impl McpGeneratorTrait for CodexMcpGenerator {
    fn generate_mcp(&self, current_dir: &Path) -> Result<HashMap<PathBuf, String>> {
        let mut files = HashMap::new();

        if let Some(config) = generate_codex_config(current_dir)? {
            files.insert(current_dir.join(CODEX_CONFIG_TOML), config);
        }

        Ok(files)
    }

    fn clean_mcp(&self, current_dir: &Path) -> Result<()> {
        let target_path = current_dir.join(CODEX_CONFIG_TOML);
        if target_path.exists() {
            fs::remove_file(&target_path)
                .with_context(|| format!("Failed to remove {}", target_path.display()))?;
        }
        Ok(())
    }

    fn check_mcp(&self, current_dir: &Path) -> Result<bool> {
        let target_path = current_dir.join(CODEX_CONFIG_TOML);

        match generate_codex_config(current_dir)? {
            Some(expected_content) => file_matches_expected(&target_path, &expected_content),
            None => Ok(!target_path.exists()),
        }
    }

    fn mcp_gitignore_patterns(&self) -> Vec<String> {
        vec![CODEX_CONFIG_TOML.to_string()]
    }

    fn box_clone(&self) -> Box<dyn McpGeneratorTrait> {
        Box::new(Self)
    }
}

fn generate_codex_config(current_dir: &Path) -> Result<Option<String>> {
    let mut codex_config = Value::Table(Table::new());
    let mut has_content = false;

    if let Some(source_mcp_content) = read_mcp_config(current_dir)? {
        let source_config: McpConfig = serde_json::from_str(&source_mcp_content)?;
        if let Some(config_table) = codex_config.as_table_mut() {
            config_table.insert(
                "mcp_servers".to_string(),
                Value::Table(generate_codex_mcp_servers_table(&source_config)),
            );
        }
        has_content = true;
    }

    let overlay_path = current_dir
        .join(AI_RULE_SOURCE_DIR)
        .join(CODEX_CONFIG_OVERLAY_TOML);
    if overlay_path.exists() {
        let overlay_content = fs::read_to_string(&overlay_path)
            .with_context(|| format!("Failed to read overlay file: {}", overlay_path.display()))?;

        let overlay_config: Value = overlay_content
            .parse()
            .with_context(|| format!("Invalid TOML in overlay file: {}", overlay_path.display()))?;

        merge_toml_values(&mut codex_config, &overlay_config);
        has_content = true;
    }

    if !has_content {
        return Ok(None);
    }

    let toml_string = toml::to_string_pretty(&codex_config)
        .with_context(|| "Failed to serialize Codex configuration to TOML")?;
    Ok(Some(ensure_trailing_newline(toml_string)))
}

fn generate_codex_mcp_servers_table(config: &McpConfig) -> Table {
    let mut mcp_servers = Table::new();

    let mut server_names: Vec<_> = config.mcp_servers.keys().collect();
    server_names.sort();

    for server_name in server_names {
        let server_config = config
            .mcp_servers
            .get(server_name)
            .expect("server name came from mcp_servers keys");

        let mut server_table = Table::new();

        match server_config {
            McpServerConfig::Command { command, args, env } => {
                server_table.insert("command".to_string(), Value::String(command.clone()));
                if let Some(args) = args {
                    server_table.insert(
                        "args".to_string(),
                        Value::Array(args.iter().cloned().map(Value::String).collect()),
                    );
                }
                if let Some(env) = env {
                    server_table.insert("env".to_string(), Value::Table(string_map_to_table(env)));
                }
            }
            McpServerConfig::Http { url, headers, .. } => {
                server_table.insert("url".to_string(), Value::String(url.clone()));
                if let Some(headers) = headers {
                    server_table.insert(
                        "http_headers".to_string(),
                        Value::Table(string_map_to_table(headers)),
                    );
                }
            }
        }

        mcp_servers.insert(server_name.clone(), Value::Table(server_table));
    }

    mcp_servers
}

fn string_map_to_table(values: &HashMap<String, String>) -> Table {
    let mut table = Table::new();
    let mut keys: Vec<_> = values.keys().collect();
    keys.sort();

    for key in keys {
        let value = values.get(key).expect("value key came from values keys");
        table.insert(key.clone(), Value::String(value.clone()));
    }

    table
}

fn merge_toml_values(base: &mut Value, overlay: &Value) {
    if let (Some(base_table), Some(overlay_table)) = (base.as_table_mut(), overlay.as_table()) {
        for (key, value) in overlay_table {
            match base_table.get_mut(key) {
                Some(base_value) if base_value.is_table() && value.is_table() => {
                    merge_toml_values(base_value, value);
                }
                _ => {
                    base_table.insert(key.clone(), value.clone());
                }
            }
        }
    }
}

fn file_matches_expected(file_path: &Path, expected_content: &str) -> Result<bool> {
    if !file_path.exists() {
        return Ok(false);
    }

    let actual_content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    Ok(actual_content == expected_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_codex_generator_name() {
        let generator = CodexGenerator::new();
        assert_eq!(generator.name(), "codex");
    }

    #[test]
    fn test_codex_generator_gitignore_patterns() {
        let generator = CodexGenerator::new();
        let patterns = generator.gitignore_patterns();
        assert_eq!(patterns, vec!["AGENTS.md".to_string()]);
    }

    #[test]
    fn test_codex_generator_clean() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        // Create an AGENTS.md file
        create_file(temp_dir.path(), "AGENTS.md", "existing content");
        assert_file_exists(temp_dir.path(), "AGENTS.md");

        // Clean should remove it
        let result = generator.clean(temp_dir.path());
        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), "AGENTS.md");
    }

    #[test]
    fn test_codex_generator_generate_agent_contents() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(result.len(), 1);
        let expected_path = temp_dir.path().join("AGENTS.md");
        let content = result.get(&expected_path).unwrap();
        assert!(content.contains("@ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md"));
    }

    #[test]
    fn test_codex_generator_check_agent_contents_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        // Write correct content
        let expected_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md\n";
        create_file(temp_dir.path(), "AGENTS.md", expected_content);

        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_codex_generator_check_agent_contents_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        // Write wrong content
        create_file(temp_dir.path(), "AGENTS.md", "wrong content");

        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_codex_generator_generate_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        // Create the source file for symlinking
        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let result = generator.generate_symlink(temp_dir.path());
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("AGENTS.md"));
    }

    #[test]
    fn test_codex_generator_check_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        // Create source and symlink
        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");
        generator.generate_symlink(temp_dir.path()).unwrap();

        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(result);
    }

    const TEST_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-test"],
      "env": {
        "API_KEY": "${API_KEY}"
      }
    }
  }
}"#;

    const TEST_HTTP_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    "figma": {
      "type": "http",
      "url": "https://mcp.figma.com/mcp",
      "headers": {
        "X-Figma-Region": "us-east-1"
      }
    }
  }
}"#;

    #[test]
    fn test_codex_generator_has_mcp_generator() {
        let generator = CodexGenerator::new();

        assert!(generator.mcp_generator().is_some());
    }

    #[test]
    fn test_codex_mcp_generator_writes_config_toml() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let mcp_gen = generator.mcp_generator().unwrap();
        let files = mcp_gen.generate_mcp(temp_dir.path()).unwrap();

        let expected_path = temp_dir.path().join(".codex/config.toml");
        let content = files.get(&expected_path).unwrap();
        let parsed: Value = content.parse().unwrap();
        let test_server = &parsed["mcp_servers"]["test-server"];

        assert_eq!(test_server["command"].as_str(), Some("npx"));
        assert_eq!(
            test_server["args"].as_array().unwrap()[0].as_str(),
            Some("-y")
        );
        assert_eq!(
            test_server["args"].as_array().unwrap()[1].as_str(),
            Some("@modelcontextprotocol/server-test")
        );
        assert_eq!(test_server["env"]["API_KEY"].as_str(), Some("${API_KEY}"));
    }

    #[test]
    fn test_codex_mcp_generator_converts_http_headers() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_HTTP_MCP_CONFIG);

        let mcp_gen = generator.mcp_generator().unwrap();
        let files = mcp_gen.generate_mcp(temp_dir.path()).unwrap();

        let expected_path = temp_dir.path().join(".codex/config.toml");
        let content = files.get(&expected_path).unwrap();
        let parsed: Value = content.parse().unwrap();
        let figma = &parsed["mcp_servers"]["figma"];

        assert_eq!(figma["url"].as_str(), Some("https://mcp.figma.com/mcp"));
        assert_eq!(
            figma["http_headers"]["X-Figma-Region"].as_str(),
            Some("us-east-1")
        );
        assert!(figma.get("type").is_none());
        assert!(figma.get("headers").is_none());
    }

    #[test]
    fn test_codex_mcp_generator_merges_overlay_with_mcp() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        create_file(
            temp_dir.path(),
            "ai-rules/codex-config.toml",
            r#"model = "gpt-5.2"

[mcp_servers."test-server"]
command = "python"

[mcp_servers.user]
command = "custom"
"#,
        );

        let mcp_gen = generator.mcp_generator().unwrap();
        let files = mcp_gen.generate_mcp(temp_dir.path()).unwrap();
        let content = files
            .get(&temp_dir.path().join(".codex/config.toml"))
            .unwrap();
        let parsed: Value = content.parse().unwrap();

        assert_eq!(parsed["model"].as_str(), Some("gpt-5.2"));
        assert_eq!(
            parsed["mcp_servers"]["test-server"]["command"].as_str(),
            Some("python")
        );
        assert_eq!(
            parsed["mcp_servers"]["test-server"]["args"]
                .as_array()
                .unwrap()[0]
                .as_str(),
            Some("-y")
        );
        assert_eq!(
            parsed["mcp_servers"]["user"]["command"].as_str(),
            Some("custom")
        );
    }

    #[test]
    fn test_codex_mcp_generator_merges_nested_overlay_tables() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        create_file(
            temp_dir.path(),
            "ai-rules/codex-config.toml",
            r#"[mcp_servers."test-server".env]
API_KEY = "override"
NODE_ENV = "test"
"#,
        );

        let mcp_gen = generator.mcp_generator().unwrap();
        let files = mcp_gen.generate_mcp(temp_dir.path()).unwrap();
        let content = files
            .get(&temp_dir.path().join(".codex/config.toml"))
            .unwrap();
        let parsed: Value = content.parse().unwrap();
        let test_server = &parsed["mcp_servers"]["test-server"];

        assert_eq!(test_server["command"].as_str(), Some("npx"));
        assert_eq!(
            test_server["args"].as_array().unwrap()[0].as_str(),
            Some("-y")
        );
        assert_eq!(test_server["env"]["API_KEY"].as_str(), Some("override"));
        assert_eq!(test_server["env"]["NODE_ENV"].as_str(), Some("test"));
    }

    #[test]
    fn test_codex_mcp_generator_overlay_only() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        create_file(
            temp_dir.path(),
            "ai-rules/codex-config.toml",
            r#"model = "gpt-5.2"

[mcp_servers.user]
command = "custom"
"#,
        );

        let mcp_gen = generator.mcp_generator().unwrap();
        let files = mcp_gen.generate_mcp(temp_dir.path()).unwrap();
        let content = files
            .get(&temp_dir.path().join(".codex/config.toml"))
            .unwrap();
        let parsed: Value = content.parse().unwrap();

        assert_eq!(parsed["model"].as_str(), Some("gpt-5.2"));
        assert_eq!(
            parsed["mcp_servers"]["user"]["command"].as_str(),
            Some("custom")
        );
    }

    #[test]
    fn test_generate_codex_config_invalid_overlay_errors() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/codex-config.toml", "model =\n");

        let result = generate_codex_config(temp_dir.path());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid TOML in overlay file"));
    }

    #[test]
    fn test_codex_mcp_generator_clean_removes_config() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(
            temp_dir.path(),
            ".codex/config.toml",
            "model = \"gpt-5.2\"\n\n[mcp_servers.user]\ncommand = \"custom\"\n",
        );

        mcp_gen.clean_mcp(temp_dir.path()).unwrap();

        assert!(!temp_dir.path().join(".codex/config.toml").exists());
    }

    #[test]
    fn test_codex_mcp_generator_check_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        let files = mcp_gen.generate_mcp(temp_dir.path()).unwrap();
        let content = files
            .get(&temp_dir.path().join(".codex/config.toml"))
            .unwrap();
        create_file(temp_dir.path(), ".codex/config.toml", content);

        assert!(mcp_gen.check_mcp(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_codex_mcp_generator_check_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        create_file(
            temp_dir.path(),
            ".codex/config.toml",
            "model = \"gpt-5.2\"\n",
        );

        assert!(!mcp_gen.check_mcp(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_codex_mcp_generator_check_no_source_with_generated_block() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(
            temp_dir.path(),
            ".codex/config.toml",
            "[mcp_servers.\"test-server\"]\ncommand = \"npx\"\n",
        );

        assert!(!mcp_gen.check_mcp(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_codex_mcp_generator_overlay_collision_writes_valid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        create_file(
            temp_dir.path(),
            "ai-rules/codex-config.toml",
            r#"[mcp_servers."test-server"]
command = "custom"
"#,
        );

        let files = mcp_gen.generate_mcp(temp_dir.path()).unwrap();
        let content = files
            .get(&temp_dir.path().join(".codex/config.toml"))
            .unwrap();
        let parsed: Value = content.parse().unwrap();

        assert_eq!(
            parsed["mcp_servers"]["test-server"]["command"].as_str(),
            Some("custom")
        );
        assert_eq!(
            parsed["mcp_servers"]["test-server"]["args"]
                .as_array()
                .unwrap()[0]
                .as_str(),
            Some("-y")
        );
    }
}

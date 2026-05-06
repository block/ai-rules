use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::mcp_generator::McpGeneratorTrait;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::{
    check_in_sync, clean_generated_files, generate_agent_file_contents,
};
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{AGENTS_MD_FILENAME, CODEX_SKILLS_DIR};
use crate::models::SourceFile;
use crate::operations::mcp_reader::{read_mcp_config, McpConfig, McpServerConfig};
use crate::utils::file_utils::{
    check_agents_md_symlink, check_inlined_file_symlink, create_symlink_to_agents_md,
    create_symlink_to_inlined_file, ensure_trailing_newline,
};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const CODEX_CONFIG_TOML: &str = ".codex/config.toml";
const MCP_GENERATED_START: &str = "# AI Rules MCP - Generated Servers";
const MCP_GENERATED_END: &str = "# End AI Rules MCP";

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
    fn generate_mcp(&self, current_dir: &Path) -> HashMap<PathBuf, String> {
        let mut files = HashMap::new();

        let source_mcp_content = match read_mcp_config(current_dir) {
            Ok(Some(content)) => content,
            _ => return files,
        };

        let source_config: McpConfig = match serde_json::from_str(&source_mcp_content) {
            Ok(config) => config,
            Err(_) => return files,
        };

        let target_path = current_dir.join(CODEX_CONFIG_TOML);
        let existing_content = fs::read_to_string(&target_path).unwrap_or_default();
        let base_content = remove_generated_mcp_block(&existing_content);

        let generated_block = generate_codex_mcp_block(&source_config);
        if generated_block.is_empty() && base_content.trim().is_empty() {
            return files;
        }

        let content = merge_generated_mcp_block(&base_content, &generated_block);
        files.insert(target_path, content);
        files
    }

    fn clean_mcp(&self, current_dir: &Path) -> Result<()> {
        let target_path = current_dir.join(CODEX_CONFIG_TOML);
        if !target_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&target_path)?;
        let cleaned = remove_generated_mcp_block(&content);
        if cleaned.trim().is_empty() {
            fs::remove_file(target_path)?;
        } else if cleaned != content {
            fs::write(target_path, ensure_trailing_newline(cleaned))?;
        }

        Ok(())
    }

    fn check_mcp(&self, current_dir: &Path) -> Result<bool> {
        let target_path = current_dir.join(CODEX_CONFIG_TOML);

        let source_mcp_content = match read_mcp_config(current_dir)? {
            Some(content) => content,
            None => {
                if !target_path.exists() {
                    return Ok(true);
                }

                let content = fs::read_to_string(&target_path)?;
                return Ok(!content.contains(MCP_GENERATED_START));
            }
        };

        if !target_path.exists() {
            return Ok(false);
        }

        let source_config: McpConfig = serde_json::from_str(&source_mcp_content)?;
        let actual = fs::read_to_string(&target_path)?;
        let base_content = remove_generated_mcp_block(&actual);
        let expected =
            merge_generated_mcp_block(&base_content, &generate_codex_mcp_block(&source_config));

        Ok(actual == expected)
    }

    fn mcp_gitignore_patterns(&self) -> Vec<String> {
        vec![CODEX_CONFIG_TOML.to_string()]
    }

    fn box_clone(&self) -> Box<dyn McpGeneratorTrait> {
        Box::new(Self)
    }
}

fn remove_generated_mcp_block(content: &str) -> String {
    let Some(start) = content.find(MCP_GENERATED_START) else {
        return content.to_string();
    };
    let Some(relative_end) = content[start..].find(MCP_GENERATED_END) else {
        return content.to_string();
    };

    let end = start + relative_end + MCP_GENERATED_END.len();
    let mut result = content.to_string();

    let range_start = start.saturating_sub(if start > 0 && result.as_bytes()[start - 1] == b'\n' {
        1
    } else {
        0
    });
    let range_end = if result.as_bytes().get(end) == Some(&b'\n') {
        end + 1
    } else {
        end
    };

    result.replace_range(range_start..range_end, "");
    result.trim_end().to_string()
}

fn merge_generated_mcp_block(base_content: &str, generated_block: &str) -> String {
    if generated_block.is_empty() {
        return ensure_trailing_newline(base_content.trim_end().to_string());
    }

    let mut content = base_content.trim_end().to_string();
    if !content.is_empty() {
        content.push_str("\n\n");
    }
    content.push_str(generated_block);
    ensure_trailing_newline(content)
}

fn generate_codex_mcp_block(config: &McpConfig) -> String {
    if config.mcp_servers.is_empty() {
        return String::new();
    }

    let mut server_names: Vec<_> = config.mcp_servers.keys().collect();
    server_names.sort();

    let mut block = String::from(MCP_GENERATED_START);
    block.push('\n');

    for server_name in server_names {
        let server_config = config
            .mcp_servers
            .get(server_name)
            .expect("server name came from mcp_servers keys");

        block.push('\n');
        block.push_str(&format!("[mcp_servers.{}]\n", toml_quoted_key(server_name)));

        match server_config {
            McpServerConfig::Command { command, args, env } => {
                block.push_str(&format!("command = {}\n", toml_string(command)));
                if let Some(args) = args {
                    block.push_str(&format!("args = {}\n", toml_string_array(args)));
                }
                if let Some(env) = env {
                    append_toml_string_table(&mut block, server_name, "env", env);
                }
            }
            McpServerConfig::Http { url, headers, .. } => {
                block.push_str(&format!("url = {}\n", toml_string(url)));
                if let Some(headers) = headers {
                    append_toml_string_table(&mut block, server_name, "http_headers", headers);
                }
            }
        }
    }

    block.push('\n');
    block.push_str(MCP_GENERATED_END);
    block
}

fn append_toml_string_table(
    block: &mut String,
    server_name: &str,
    table_name: &str,
    values: &HashMap<String, String>,
) {
    if values.is_empty() {
        return;
    }

    block.push('\n');
    block.push_str(&format!(
        "[mcp_servers.{}.{}]\n",
        toml_quoted_key(server_name),
        table_name
    ));

    let mut keys: Vec<_> = values.keys().collect();
    keys.sort();
    for key in keys {
        let value = values.get(key).expect("value key came from values keys");
        block.push_str(&format!(
            "{} = {}\n",
            toml_quoted_key(key),
            toml_string(value)
        ));
    }
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string should not fail")
}

fn toml_string_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| toml_string(value))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
}

fn toml_quoted_key(value: &str) -> String {
    toml_string(value)
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
        let files = mcp_gen.generate_mcp(temp_dir.path());

        let expected_path = temp_dir.path().join(".codex/config.toml");
        let content = files.get(&expected_path).unwrap();

        assert!(content.contains(MCP_GENERATED_START));
        assert!(content.contains("[mcp_servers.\"test-server\"]"));
        assert!(content.contains("command = \"npx\""));
        assert!(content.contains("args = [\"-y\", \"@modelcontextprotocol/server-test\"]"));
        assert!(content.contains("[mcp_servers.\"test-server\".env]"));
        assert!(content.contains("\"API_KEY\" = \"${API_KEY}\""));
    }

    #[test]
    fn test_codex_mcp_generator_converts_http_headers() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_HTTP_MCP_CONFIG);

        let mcp_gen = generator.mcp_generator().unwrap();
        let files = mcp_gen.generate_mcp(temp_dir.path());

        let expected_path = temp_dir.path().join(".codex/config.toml");
        let content = files.get(&expected_path).unwrap();

        assert!(content.contains("[mcp_servers.\"figma\"]"));
        assert!(content.contains("url = \"https://mcp.figma.com/mcp\""));
        assert!(content.contains("[mcp_servers.\"figma\".http_headers]"));
        assert!(content.contains("\"X-Figma-Region\" = \"us-east-1\""));
        assert!(!content.contains("type = \"http\""));
        assert!(!content.contains("headers ="));
    }

    #[test]
    fn test_codex_mcp_generator_preserves_existing_config() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        create_file(
            temp_dir.path(),
            ".codex/config.toml",
            "model = \"gpt-5.2\"\n\n[mcp_servers.user]\ncommand = \"custom\"\n",
        );

        let mcp_gen = generator.mcp_generator().unwrap();
        let files = mcp_gen.generate_mcp(temp_dir.path());
        let content = files
            .get(&temp_dir.path().join(".codex/config.toml"))
            .unwrap();

        assert!(content.contains("model = \"gpt-5.2\""));
        assert!(content.contains("[mcp_servers.user]"));
        assert!(content.contains("command = \"custom\""));
        assert!(content.contains("[mcp_servers.\"test-server\"]"));
    }

    #[test]
    fn test_codex_mcp_generator_clean_removes_only_generated_block() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        let files = mcp_gen.generate_mcp(temp_dir.path());
        let generated = files
            .get(&temp_dir.path().join(".codex/config.toml"))
            .unwrap();
        create_file(
            temp_dir.path(),
            ".codex/config.toml",
            &format!("model = \"gpt-5.2\"\n\n{generated}"),
        );

        mcp_gen.clean_mcp(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(temp_dir.path().join(".codex/config.toml")).unwrap();
        assert_eq!(content, "model = \"gpt-5.2\"\n");
    }

    #[test]
    fn test_codex_mcp_generator_check_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = CodexGenerator::new();
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        let files = mcp_gen.generate_mcp(temp_dir.path());
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
            "# AI Rules MCP - Generated Servers\n\n[mcp_servers.\"test-server\"]\ncommand = \"npx\"\n# End AI Rules MCP\n",
        );

        assert!(!mcp_gen.check_mcp(temp_dir.path()).unwrap());
    }
}

//! Firebender agent implementation using AGENTS.md plus supplemental firebender.json config.

use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::external_commands_generator::ExternalCommandsGenerator;
use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::mcp_generator::McpGeneratorTrait;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::single_file_based::{
    check_in_sync, clean_generated_files, generate_agent_file_contents,
};
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{
    AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR, AMP_SKILLS_DIR, FIREBENDER_COMMANDS_DIR,
    FIREBENDER_JSON, FIREBENDER_OVERLAY_JSON, MCP_SERVERS_FIELD,
};
use crate::models::SourceFile;
use crate::operations::mcp_reader::extract_mcp_servers_for_firebender;
use crate::utils::file_utils::{
    check_agents_md_symlink, check_inlined_file_symlink, create_symlink_to_agents_md,
    create_symlink_to_inlined_file, ensure_trailing_newline,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FirebenderGenerator;

#[derive(Clone)]
struct FirebenderConfigGenerator;

impl AgentRuleGenerator for FirebenderGenerator {
    fn name(&self) -> &str {
        "firebender"
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        clean_generated_files(current_dir, AGENTS_MD_FILENAME)
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        generate_agent_file_contents(source_files, current_dir, AGENTS_MD_FILENAME)
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        check_in_sync(source_files, current_dir, AGENTS_MD_FILENAME)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(AGENTS_MD_FILENAME);
        check_agents_md_symlink(current_dir, &output_file)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        vec![AGENTS_MD_FILENAME.to_string()]
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_agents_md(current_dir, Path::new(AGENTS_MD_FILENAME))?;
        if success {
            Ok(vec![current_dir.join(AGENTS_MD_FILENAME)])
        } else {
            Ok(vec![])
        }
    }

    fn uses_inlined_symlink(&self) -> bool {
        true
    }

    fn generate_inlined_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_inlined_file(current_dir, Path::new(AGENTS_MD_FILENAME))?;
        if success {
            Ok(vec![current_dir.join(AGENTS_MD_FILENAME)])
        } else {
            Ok(vec![])
        }
    }

    fn check_inlined_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(AGENTS_MD_FILENAME);
        check_inlined_file_symlink(current_dir, &output_file)
    }

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        Some(Box::new(FirebenderConfigGenerator))
    }

    fn command_generator(&self) -> Option<Box<dyn CommandGeneratorTrait>> {
        Some(Box::new(ExternalCommandsGenerator::with_extension(
            FIREBENDER_COMMANDS_DIR,
            "mdc",
        )))
    }

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        Some(Box::new(ExternalSkillsGenerator::new(AMP_SKILLS_DIR)))
    }
}

impl McpGeneratorTrait for FirebenderConfigGenerator {
    fn generate_mcp(&self, current_dir: &Path) -> HashMap<PathBuf, String> {
        let mut files = HashMap::new();

        if let Ok(Some(config)) = generate_firebender_config(current_dir) {
            files.insert(current_dir.join(FIREBENDER_JSON), config);
        }

        files
    }

    fn clean_mcp(&self, current_dir: &Path) -> Result<()> {
        let firebender_file = current_dir.join(FIREBENDER_JSON);
        if firebender_file.exists() {
            fs::remove_file(&firebender_file)
                .with_context(|| format!("Failed to remove {}", firebender_file.display()))?;
        }
        Ok(())
    }

    fn check_mcp(&self, current_dir: &Path) -> Result<bool> {
        let firebender_file = current_dir.join(FIREBENDER_JSON);

        match generate_firebender_config(current_dir)? {
            Some(expected_content) => file_matches_expected(&firebender_file, &expected_content),
            None => Ok(!firebender_file.exists()),
        }
    }

    fn mcp_gitignore_patterns(&self) -> Vec<String> {
        vec![FIREBENDER_JSON.to_string()]
    }

    fn box_clone(&self) -> Box<dyn McpGeneratorTrait> {
        Box::new(self.clone())
    }
}

fn generate_firebender_config(current_dir: &Path) -> Result<Option<String>> {
    let mut firebender_config = json!({});
    let mut has_content = false;

    if let Some(mcp_servers) = extract_mcp_servers_for_firebender(current_dir)? {
        firebender_config[MCP_SERVERS_FIELD] = mcp_servers;
        has_content = true;
    }

    let overlay_path = current_dir
        .join(AI_RULE_SOURCE_DIR)
        .join(FIREBENDER_OVERLAY_JSON);
    if overlay_path.exists() {
        let overlay_content = fs::read_to_string(&overlay_path)
            .with_context(|| format!("Failed to read overlay file: {}", overlay_path.display()))?;

        let overlay_json: Value = serde_json::from_str(&overlay_content)
            .with_context(|| format!("Invalid JSON in overlay file: {}", overlay_path.display()))?;

        merge_json_objects(&mut firebender_config, &overlay_json);
        has_content = true;
    }

    if !has_content {
        return Ok(None);
    }

    let json_string = serde_json::to_string_pretty(&firebender_config)
        .with_context(|| "Failed to serialize firebender configuration to JSON")?;

    Ok(Some(ensure_trailing_newline(json_string)))
}

/// Recursively merges JSON objects, giving precedence to values in `overlay`.
fn merge_json_objects(base: &mut Value, overlay: &Value) {
    if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
        for (key, value) in overlay_obj {
            match base_obj.get_mut(key) {
                Some(base_value) if base_value.is_object() && value.is_object() => {
                    merge_json_objects(base_value, value);
                }
                _ => {
                    base_obj.insert(key.clone(), value.clone());
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
    use crate::constants::{
        AI_RULE_SOURCE_DIR, FIREBENDER_OVERLAY_JSON, GENERATED_FILE_PREFIX, SKILLS_DIR,
        SKILL_FILENAME,
    };
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    fn create_standard_test_source_file() -> SourceFile {
        create_test_source_file(
            "test",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "test body",
        )
    }

    const TEST_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-test"]
    }
  }
}"#;

    #[test]
    fn test_generate_agent_contents_uses_agents_md() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_standard_test_source_file()];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(result.len(), 1);
        let expected_path = temp_dir.path().join(AGENTS_MD_FILENAME);
        let content = result.get(&expected_path).unwrap();
        assert_eq!(
            content,
            "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n"
        );
    }

    #[test]
    fn test_generate_symlink_creates_agents_md_symlink() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Shared rules");

        let result = generator.generate_symlink(temp_dir.path()).unwrap();

        assert_eq!(result, vec![temp_dir.path().join(AGENTS_MD_FILENAME)]);
        assert!(temp_dir.path().join(AGENTS_MD_FILENAME).is_symlink());
    }

    #[test]
    fn test_clean_removes_agents_md() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), AGENTS_MD_FILENAME, "generated content");
        generator.clean(temp_dir.path()).unwrap();

        assert_file_not_exists(temp_dir.path(), AGENTS_MD_FILENAME);
    }

    #[test]
    fn test_generate_firebender_config_without_sources_returns_none() {
        let temp_dir = TempDir::new().unwrap();

        let result = generate_firebender_config(temp_dir.path()).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_generate_firebender_config_with_mcp() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let result = generate_firebender_config(temp_dir.path())
            .unwrap()
            .unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed["mcpServers"]["test-server"].is_object());
        assert_eq!(parsed["mcpServers"]["test-server"]["command"], "npx");
    }

    #[test]
    fn test_generate_firebender_config_with_overlay_only() {
        let temp_dir = TempDir::new().unwrap();
        create_file(
            temp_dir.path(),
            &format!("{AI_RULE_SOURCE_DIR}/{FIREBENDER_OVERLAY_JSON}"),
            r#"{
  "backgroundAgent": {
    "copyFiles": ["local.properties"]
  }
}"#,
        );

        let result = generate_firebender_config(temp_dir.path())
            .unwrap()
            .unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(
            parsed["backgroundAgent"]["copyFiles"].as_array().unwrap()[0],
            "local.properties"
        );
        assert!(parsed["mcpServers"].is_null());
    }

    #[test]
    fn test_generate_firebender_config_merges_overlay_with_mcp() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        create_file(
            temp_dir.path(),
            &format!("{AI_RULE_SOURCE_DIR}/{FIREBENDER_OVERLAY_JSON}"),
            r#"{
  "mcpServers": {
    "overlay-server": {
      "command": "python",
      "args": ["-m", "overlay_mcp"]
    }
  },
  "backgroundAgent": {
    "copyFiles": ["settings.gradle"]
  }
}"#,
        );

        let result = generate_firebender_config(temp_dir.path())
            .unwrap()
            .unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["mcpServers"]["test-server"]["command"], "npx");
        assert_eq!(parsed["mcpServers"]["overlay-server"]["command"], "python");
        assert_eq!(
            parsed["backgroundAgent"]["copyFiles"].as_array().unwrap()[0],
            "settings.gradle"
        );
    }

    #[test]
    fn test_generate_firebender_config_invalid_overlay_errors() {
        let temp_dir = TempDir::new().unwrap();
        create_file(
            temp_dir.path(),
            &format!("{AI_RULE_SOURCE_DIR}/{FIREBENDER_OVERLAY_JSON}"),
            "{ invalid json",
        );

        let result = generate_firebender_config(temp_dir.path());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid JSON in overlay file"));
    }

    #[test]
    fn test_firebender_mcp_generator_generates_firebender_json() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let files = mcp_gen.generate_mcp(temp_dir.path());

        assert_eq!(files.len(), 1);
        let expected_path = temp_dir.path().join(FIREBENDER_JSON);
        assert!(files.contains_key(&expected_path));
    }

    #[test]
    fn test_firebender_mcp_generator_omits_firebender_json_without_mcp_or_overlay() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;
        let mcp_gen = generator.mcp_generator().unwrap();

        let files = mcp_gen.generate_mcp(temp_dir.path());

        assert!(files.is_empty());
    }

    #[test]
    fn test_firebender_mcp_generator_check_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;
        let mcp_gen = generator.mcp_generator().unwrap();

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        let expected = generate_firebender_config(temp_dir.path())
            .unwrap()
            .unwrap();
        create_file(temp_dir.path(), FIREBENDER_JSON, &expected);

        assert!(mcp_gen.check_mcp(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_firebender_has_command_generator() {
        let generator = FirebenderGenerator;
        let cmd_gen = generator.command_generator().unwrap();

        assert_eq!(
            cmd_gen.command_gitignore_patterns(),
            vec![".firebender/commands/*-ai-rules.mdc"]
        );
    }

    #[test]
    fn test_firebender_command_generator_creates_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;
        let cmd_gen = generator.command_generator().unwrap();

        create_file(
            temp_dir.path(),
            "ai-rules/commands/commit.md",
            "# Commit command",
        );

        let symlinks = cmd_gen.generate_command_symlinks(temp_dir.path()).unwrap();

        assert_eq!(symlinks.len(), 1);
        assert!(temp_dir
            .path()
            .join(".firebender/commands/commit-ai-rules.mdc")
            .is_symlink());
    }

    #[test]
    fn test_firebender_has_skills_generator() {
        let generator = FirebenderGenerator;
        assert!(generator.skills_generator().is_some());
    }

    #[test]
    fn test_firebender_skills_gitignore_patterns() {
        let generator = FirebenderGenerator;
        let skills_gen = generator.skills_generator().unwrap();
        assert_eq!(
            skills_gen.skills_gitignore_patterns(),
            vec![".agents/skills/ai-rules-generated-*"]
        );
    }

    #[test]
    fn test_firebender_skills_generator_creates_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;
        let skills_gen = generator.skills_generator().unwrap();

        let skill_dir = temp_dir
            .path()
            .join(AI_RULE_SOURCE_DIR)
            .join(SKILLS_DIR)
            .join("debugging");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join(SKILL_FILENAME), "# Debugging").unwrap();

        let symlinks = skills_gen.generate_skills(temp_dir.path()).unwrap();

        assert_eq!(symlinks.len(), 1);
        assert!(temp_dir
            .path()
            .join(".agents/skills")
            .join(format!("{GENERATED_FILE_PREFIX}debugging"))
            .is_symlink());
    }

    #[test]
    fn test_merge_json_objects_merges_nested_objects() {
        let mut base = json!({
            "mcpServers": {
                "server-a": {
                    "command": "npx"
                }
            },
            "backgroundAgent": {
                "copyFiles": ["a"]
            }
        });

        let overlay = json!({
            "mcpServers": {
                "server-b": {
                    "command": "python"
                }
            },
            "backgroundAgent": {
                "otherSetting": true
            }
        });

        merge_json_objects(&mut base, &overlay);

        assert_eq!(base["mcpServers"]["server-a"]["command"], "npx");
        assert_eq!(base["mcpServers"]["server-b"]["command"], "python");
        assert_eq!(
            base["backgroundAgent"]["copyFiles"].as_array().unwrap()[0],
            "a"
        );
        assert_eq!(base["backgroundAgent"]["otherSetting"], true);
    }
}

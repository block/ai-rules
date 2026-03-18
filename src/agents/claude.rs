use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::external_commands_generator::ExternalCommandsGenerator;
use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::mcp_generator::{ExternalMcpGenerator, McpGeneratorTrait};
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{
    CLAUDE_COMMANDS_DIR, CLAUDE_COMMANDS_SUBDIR, CLAUDE_MCP_JSON, CLAUDE_SETTINGS_JSON,
    CLAUDE_SKILLS_DIR, GENERATED_FILE_PREFIX, GENERATED_MCP_SERVER_PREFIX,
};
use crate::models::source_file::SourceFile;
use crate::operations::{claude_skills, generate_inlined_required_content};
use crate::operations::mcp_reader::read_mcp_config;
use crate::utils::file_utils::{
    check_agents_md_symlink, check_inlined_file_symlink, create_symlink_to_agents_md,
    create_symlink_to_inlined_file,
};
use anyhow::Result;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};


pub struct ClaudeGenerator {
    name: String,
    output_filename: String,
    skills_mode: bool,
    global_mcp: bool,
}

impl ClaudeGenerator {
    pub fn new(name: &str, output_filename: &str, skills_mode: bool, global_mcp: bool) -> Self {
        Self {
            name: name.to_string(),
            output_filename: output_filename.to_string(),
            skills_mode,
            global_mcp,
        }
    }
}

impl AgentRuleGenerator for ClaudeGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        let output_file = current_dir.join(&self.output_filename);
        if output_file.exists() || output_file.is_symlink() {
            fs::remove_file(&output_file)?;
        }
        claude_skills::remove_generated_skills(current_dir)?;

        Ok(())
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        let mut all_files = HashMap::new();

        if !source_files.is_empty() && self.skills_mode {
            // In skills mode: inline required content (not @ refs), skills handle optional
            let content = generate_inlined_required_content(source_files);
            all_files.insert(current_dir.join(&self.output_filename), content);

            if let Ok(skill_files) =
                claude_skills::generate_skills_for_optional_rules(source_files, current_dir)
            {
                all_files.extend(skill_files);
            }
            // Non-skills mode is handled by generate_inlined_symlink
        }

        all_files
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        let file_path = current_dir.join(&self.output_filename);

        if source_files.is_empty() {
            if file_path.exists() {
                return Ok(false);
            }
        } else {
            if !file_path.exists() {
                return Ok(false);
            }
            // In skills mode: check inlined required content
            let expected_content = generate_inlined_required_content(source_files);
            let actual_content = fs::read_to_string(&file_path)?;
            if actual_content != expected_content {
                return Ok(false);
            }
        }

        if self.skills_mode {
            claude_skills::check_skills_in_sync(source_files, current_dir)
        } else {
            Ok(true)
        }
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(&self.output_filename);
        check_agents_md_symlink(current_dir, &output_file)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        let mut patterns = vec![self.output_filename.clone()];
        if self.skills_mode {
            patterns.push(format!("{}/{}*", CLAUDE_SKILLS_DIR, GENERATED_FILE_PREFIX));
        }
        patterns
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
        !self.skills_mode
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
        if self.global_mcp {
            Some(Box::new(ClaudeGlobalMcpGenerator))
        } else {
            Some(Box::new(ExternalMcpGenerator::new(PathBuf::from(
                CLAUDE_MCP_JSON,
            ))))
        }
    }

    fn command_generator(&self) -> Option<Box<dyn CommandGeneratorTrait>> {
        Some(Box::new(ExternalCommandsGenerator::with_subdir(
            CLAUDE_COMMANDS_DIR,
            CLAUDE_COMMANDS_SUBDIR,
        )))
    }

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        // Return a skills generator for user-defined skills in ai-rules/skills/
        // This is separate from the existing optional-rules-based skills
        Some(Box::new(ExternalSkillsGenerator::new(CLAUDE_SKILLS_DIR)))
    }
}

struct ClaudeGlobalMcpGenerator;

impl McpGeneratorTrait for ClaudeGlobalMcpGenerator {
    fn generate_mcp(&self, current_dir: &Path) -> HashMap<PathBuf, String> {
        let mut files = HashMap::new();

        let source_mcp_content = match read_mcp_config(current_dir) {
            Ok(Some(c)) => c,
            _ => return files,
        };
        let source_json: Value = serde_json::from_str(&source_mcp_content).unwrap_or(json!({}));
        let source_servers = source_json.get("mcpServers").unwrap_or(&json!({})).clone();
        let prefixed_servers = self.prefix_server_names(&source_servers);

        let target_path = current_dir.join(CLAUDE_SETTINGS_JSON);
        let mut target_json = if target_path.exists() {
            let content = fs::read_to_string(&target_path).unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };

        let existing_servers = target_json
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let mut merged_servers: Map<String, Value> = existing_servers
            .into_iter()
            .filter(|(name, _)| !name.starts_with(GENERATED_MCP_SERVER_PREFIX))
            .collect();

        if let Some(obj) = prefixed_servers.as_object() {
            for (name, config) in obj {
                merged_servers.insert(name.clone(), config.clone());
            }
        }

        target_json["mcpServers"] = Value::Object(merged_servers);

        if let Ok(content) = serde_json::to_string_pretty(&target_json) {
            files.insert(target_path, content);
        }

        files
    }

    fn clean_mcp(&self, current_dir: &Path) -> Result<()> {
        let target_path = current_dir.join(CLAUDE_SETTINGS_JSON);
        if target_path.exists() {
            let content = fs::read_to_string(&target_path)?;
            let mut json: Value = serde_json::from_str(&content)?;

            if let Some(obj) = json.as_object_mut() {
                if let Some(mcp_servers) = obj.get_mut("mcpServers") {
                    if let Some(servers_obj) = mcp_servers.as_object_mut() {
                        servers_obj.retain(|name, _| !name.starts_with(GENERATED_MCP_SERVER_PREFIX));
                    }
                }
                fs::write(&target_path, serde_json::to_string_pretty(&json)?)?;
            }
        }
        Ok(())
    }

    fn check_mcp(&self, current_dir: &Path) -> Result<bool> {
        let target_path = current_dir.join(CLAUDE_SETTINGS_JSON);

        let source_mcp_content = match read_mcp_config(current_dir)? {
            Some(c) => c,
            None => {
                if !target_path.exists() {
                    return Ok(true);
                }
                let target_json: Value =
                    serde_json::from_str(&fs::read_to_string(&target_path)?)?;
                let has_no_generated = match target_json.get("mcpServers") {
                    None => true,
                    Some(val) => val
                        .as_object()
                        .is_none_or(|o| !o.keys().any(|k| k.starts_with(GENERATED_MCP_SERVER_PREFIX))),
                };
                return Ok(has_no_generated);
            }
        };

        if !target_path.exists() {
            return Ok(false);
        }

        let source_json: Value = serde_json::from_str(&source_mcp_content)?;
        let expected_servers =
            self.prefix_server_names(source_json.get("mcpServers").unwrap_or(&json!({})));

        let target_json: Value = serde_json::from_str(&fs::read_to_string(&target_path)?)?;
        let target_generated: Map<String, Value> = target_json
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter(|(name, _)| name.starts_with(GENERATED_MCP_SERVER_PREFIX))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(target_generated == expected_servers.as_object().cloned().unwrap_or_default())
    }

    fn mcp_gitignore_patterns(&self) -> Vec<String> {
        vec![]
    }

    fn box_clone(&self) -> Box<dyn McpGeneratorTrait> {
        Box::new(Self)
    }
}

impl ClaudeGlobalMcpGenerator {
    fn prefix_server_names(&self, servers: &Value) -> Value {
        if let Some(obj) = servers.as_object() {
            Value::Object(
                obj.iter()
                    .map(|(name, config)| {
                        (format!("{}{}", GENERATED_MCP_SERVER_PREFIX, name), config.clone())
                    })
                    .collect(),
            )
        } else {
            json!({})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_clean_removes_both_file_and_skills() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", true, false);

        create_file(temp_dir.path(), "CLAUDE.md", "content");

        let generated_skills_dir = temp_dir
            .path()
            .join(".claude/skills/ai-rules-generated-test");
        std::fs::create_dir_all(&generated_skills_dir).unwrap();
        std::fs::write(generated_skills_dir.join("SKILL.md"), "generated skill").unwrap();

        let user_skills_dir = temp_dir.path().join(".claude/skills/my-custom-skill");
        std::fs::create_dir_all(&user_skills_dir).unwrap();
        std::fs::write(user_skills_dir.join("SKILL.md"), "user skill").unwrap();

        generator.clean(temp_dir.path()).unwrap();

        assert!(!temp_dir.path().join("CLAUDE.md").exists());
        assert!(!generated_skills_dir.exists());
        assert!(user_skills_dir.exists());
    }

    #[test]
    fn test_mcp_generator_project_mode_uses_dot_mcp_json() {
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", false, false);
        let mcp_gen = generator.mcp_generator().unwrap();
        assert!(mcp_gen.mcp_gitignore_patterns().contains(&".mcp.json".to_string()));
    }

    #[test]
    fn test_mcp_generator_global_mode_returns_global_generator() {
        let generator = ClaudeGenerator::new("claude", ".claude/CLAUDE.md", false, true);
        let mcp_gen = generator.mcp_generator().unwrap();
        assert!(mcp_gen.mcp_gitignore_patterns().is_empty());
    }

    #[test]
    fn test_global_mcp_generates_nothing_when_no_source() {
        let temp_dir = TempDir::new().unwrap();
        let gen = ClaudeGlobalMcpGenerator;

        let files = gen.generate_mcp(temp_dir.path());

        assert!(files.is_empty());
    }

    #[test]
    fn test_global_mcp_generates_claude_json_with_prefixed_servers() {
        let temp_dir = TempDir::new().unwrap();
        let gen = ClaudeGlobalMcpGenerator;

        create_file(
            temp_dir.path(),
            "ai-rules/mcp.json",
            r#"{"mcpServers":{"my-server":{"command":"npx","args":["-y","@test/server"]}}}"#,
        );

        let files = gen.generate_mcp(temp_dir.path());

        assert_eq!(files.len(), 1);
        let content = files.values().next().unwrap();
        let json: Value = serde_json::from_str(content).unwrap();
        let servers = json["mcpServers"].as_object().unwrap();
        assert!(servers.contains_key("air-my-server"));
        assert!(!servers.contains_key("my-server"));
    }

    #[test]
    fn test_global_mcp_preserves_user_servers() {
        let temp_dir = TempDir::new().unwrap();
        let gen = ClaudeGlobalMcpGenerator;

        create_file(
            temp_dir.path(),
            ".claude.json",
            r#"{"mcpServers":{"user-server":{"command":"my-tool"}},"someOtherSetting":true}"#,
        );
        create_file(
            temp_dir.path(),
            "ai-rules/mcp.json",
            r#"{"mcpServers":{"gen-server":{"command":"npx"}}}"#,
        );

        let files = gen.generate_mcp(temp_dir.path());
        let content = files.values().next().unwrap();
        let json: Value = serde_json::from_str(content).unwrap();
        let servers = json["mcpServers"].as_object().unwrap();

        assert!(servers.contains_key("user-server"), "user server should be preserved");
        assert!(servers.contains_key("air-gen-server"), "generated server should be added");
        assert_eq!(json["someOtherSetting"], true, "other settings should be preserved");
    }

    #[test]
    fn test_global_mcp_clean_removes_generated_preserves_user() {
        let temp_dir = TempDir::new().unwrap();
        let gen = ClaudeGlobalMcpGenerator;

        create_file(
            temp_dir.path(),
            ".claude.json",
            r#"{"mcpServers":{"user-server":{"command":"mine"},"air-old":{"command":"old"}}}"#,
        );

        gen.clean_mcp(temp_dir.path()).unwrap();

        let content = std::fs::read_to_string(temp_dir.path().join(".claude.json")).unwrap();
        let json: Value = serde_json::from_str(&content).unwrap();
        let servers = json["mcpServers"].as_object().unwrap();

        assert!(servers.contains_key("user-server"));
        assert!(!servers.contains_key("air-old"));
    }

    #[test]
    fn test_global_mcp_check_in_sync() {
        let temp_dir = TempDir::new().unwrap();
        let gen = ClaudeGlobalMcpGenerator;

        create_file(
            temp_dir.path(),
            "ai-rules/mcp.json",
            r#"{"mcpServers":{"my-server":{"command":"npx"}}}"#,
        );

        let files = gen.generate_mcp(temp_dir.path());
        for (path, content) in &files {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }

        assert!(gen.check_mcp(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_global_mcp_check_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();
        let gen = ClaudeGlobalMcpGenerator;

        create_file(
            temp_dir.path(),
            "ai-rules/mcp.json",
            r#"{"mcpServers":{"my-server":{"command":"npx"}}}"#,
        );
        create_file(temp_dir.path(), ".claude.json", r#"{"mcpServers":{}}"#);

        assert!(!gen.check_mcp(temp_dir.path()).unwrap());
    }

    #[test]
    fn test_global_mcp_gitignore_patterns_empty() {
        let gen = ClaudeGlobalMcpGenerator;
        assert!(gen.mcp_gitignore_patterns().is_empty());
    }

    #[test]
    fn test_gitignore_patterns_includes_skills() {
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", true, false);
        let patterns = generator.gitignore_patterns();

        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"CLAUDE.md".to_string()));
        assert!(patterns.contains(&".claude/skills/ai-rules-generated-*".to_string()));
    }

    #[test]
    fn test_gitignore_patterns_no_skills_mode() {
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", false, false);
        let patterns = generator.gitignore_patterns();

        assert_eq!(patterns.len(), 1);
        assert!(patterns.contains(&"CLAUDE.md".to_string()));
    }

    #[test]
    fn test_generate_agent_contents_creates_both() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", true, false);
        let source_files = vec![
            create_test_source_file(
                "always1",
                "Always",
                true,
                vec!["**/*.ts".to_string()],
                "Always content",
            ),
            create_test_source_file(
                "optional1",
                "Optional",
                false,
                vec!["**/*.js".to_string()],
                "Optional content",
            ),
        ];

        let files = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(files.len(), 2);

        let claude_md_path = temp_dir.path().join("CLAUDE.md");
        let claude_content = files.get(&claude_md_path).expect("CLAUDE.md should exist");
        // In skills mode, CLAUDE.md should contain inlined required content with description header
        assert_eq!(claude_content, "# Always\n\nAlways content\n");

        let skill_path = temp_dir
            .path()
            .join(".claude/skills/ai-rules-generated-optional1/SKILL.md");
        let skill_content = files.get(&skill_path).expect("Skill file should exist");
        assert!(skill_content.contains("name: optional"));
        assert!(skill_content.contains("description: Optional"));
        assert!(
            skill_content.contains("@ai-rules/.generated-ai-rules/ai-rules-generated-optional1.md")
        );
    }

    #[test]
    fn test_generate_agent_contents_non_skills_mode() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", false, false);
        let source_files = vec![
            create_test_source_file(
                "always1",
                "Always",
                true,
                vec!["**/*.ts".to_string()],
                "Always content",
            ),
            create_test_source_file(
                "optional1",
                "Optional",
                false,
                vec!["**/*.js".to_string()],
                "Optional content",
            ),
        ];

        let files = generator.generate_agent_contents(&source_files, temp_dir.path());

        // In non-skills mode, generate_agent_contents returns empty
        // (symlinks handle output via generate_inlined_symlink)
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_check_agent_contents_validates_both() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ClaudeGenerator::new("claude", "CLAUDE.md", true, false);
        let source_files = vec![
            create_test_source_file("always1", "Always", true, vec![], "Always content"),
            create_test_source_file("optional1", "Optional", false, vec![], "Optional content"),
        ];

        // Initially not in sync (no files)
        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(!result);

        // Create CLAUDE.md with inlined required content
        let claude_content = generate_inlined_required_content(&source_files);
        create_file(temp_dir.path(), "CLAUDE.md", &claude_content);

        // Still not in sync (missing skill)
        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(!result);

        // Create skill file
        let skill_dir = temp_dir
            .path()
            .join(".claude/skills/ai-rules-generated-optional1");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: optional\ndescription: Optional\n---\n\n@ai-rules/.generated-ai-rules/ai-rules-generated-optional1.md",
        )
        .unwrap();

        // Now in sync
        let result = generator
            .check_agent_contents(&source_files, temp_dir.path())
            .unwrap();
        assert!(result);
    }
}

use crate::agents::AgentToolRegistry;
use crate::operations;
use crate::utils::file_utils;
use anyhow::Result;
use std::path::Path;

pub fn run_clean(current_dir: &Path, nested_depth: usize, use_claude_skills: bool) -> Result<()> {
    println!("📋 Cleaning files for all agents, nested_depth: {nested_depth}");
    let registry = AgentToolRegistry::new(use_claude_skills);

    let agents: Vec<String> = registry.get_all_tool_names();

    file_utils::traverse_project_directories(current_dir, nested_depth, 0, &mut |dir| {
        operations::clean_generated_files(dir, &agents, &registry)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::AGENTS_MD_FILENAME;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    const NESTED_DEPTH: usize = 6;

    const CLEAN_NESTED_DEPTH: usize = NESTED_DEPTH;

    #[test]
    fn test_run_clean_removes_generated_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        create_file(project_path, ".generated-ai-rules/.gitkeep", "");
        create_file(project_path, "CLAUDE.md", "Generated content");
        create_file(
            project_path,
            ".cursor/rules/ai-rules-generated-test.mdc",
            "Cursor rules",
        );
        create_file(project_path, AGENTS_MD_FILENAME, "Goose rules");

        create_file(project_path, "ai-rules/test.md", "Original rule");
        create_file(project_path, "src/main.ts", "console.log('test');");

        let result = run_clean(project_path, CLEAN_NESTED_DEPTH, false);
        assert!(result.is_ok());

        assert_file_not_exists(project_path, "CLAUDE.md");
        assert_file_not_exists(project_path, ".cursor/rules/ai-rules-generated-test.mdc");
        assert_file_not_exists(project_path, AGENTS_MD_FILENAME);

        assert_file_exists(project_path, "ai-rules/test.md");
        assert_file_exists(project_path, "src/main.ts");
    }

    #[test]
    fn test_run_clean_nested_folders() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        create_file(project_path, "subproject1/.generated-ai-rules/.gitkeep", "");
        create_file(project_path, "subproject1/CLAUDE.md", "Generated content");
        create_file(
            project_path,
            "subproject1/.cursor/rules/ai-rules-generated-test.mdc",
            "Cursor rules",
        );
        create_file(
            project_path,
            "nested/deep/subproject2/CLAUDE.md",
            "Deep generated content",
        );
        create_file(
            project_path,
            &format!("nested/deep/subproject2/{AGENTS_MD_FILENAME}"),
            "Deep goose rules",
        );

        create_file(
            project_path,
            "subproject1/ai-rules/rule.md",
            "Original rule",
        );
        create_file(project_path, "nested/deep/subproject2/src/code.ts", "code");

        let result = run_clean(project_path, CLEAN_NESTED_DEPTH, false);
        assert!(result.is_ok());

        assert_file_not_exists(project_path, "subproject1/CLAUDE.md");
        assert_file_not_exists(
            project_path,
            "subproject1/.cursor/rules/ai-rules-generated-test.mdc",
        );
        assert_file_not_exists(project_path, "nested/deep/subproject2/CLAUDE.md");
        assert_file_not_exists(project_path, "nested/deep/subproject2/AGENTS.md");

        // Verify source files remain
        assert_file_exists(project_path, "subproject1/ai-rules/rule.md");
        assert_file_exists(project_path, "nested/deep/subproject2/src/code.ts");
    }

    #[test]
    fn test_run_clean_generated_files_without_ai_rules() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        create_file(project_path, ".generated-ai-rules/.gitkeep", "");
        create_file(project_path, "CLAUDE.md", "Generated content");
        create_file(
            project_path,
            ".cursor/rules/ai-rules-generated-orphan.mdc",
            "Orphaned cursor rules",
        );
        create_file(project_path, AGENTS_MD_FILENAME, "Orphaned goose rules");

        create_file(project_path, "src/main.rs", "fn main() {}");

        let result = run_clean(project_path, CLEAN_NESTED_DEPTH, false);
        assert!(result.is_ok());

        assert_file_not_exists(project_path, "CLAUDE.md");
        assert_file_not_exists(project_path, ".cursor/rules/ai-rules-generated-orphan.mdc");
        assert_file_not_exists(project_path, AGENTS_MD_FILENAME);

        assert_file_exists(project_path, "src/main.rs");

        assert_file_not_exists(project_path, "ai-rules");
    }

    const TEST_RULE_CONTENT: &str = r#"---
description: Test rule
alwaysApply: true
fileMatching: "**/*.ts"
---
Test rule content"#;

    #[test]
    fn test_run_clean_depth_0_after_generate_depth_2() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        create_file(project_path, "ai-rules/root-rule.md", TEST_RULE_CONTENT);
        create_file(
            project_path,
            "level1/ai-rules/level1-rule.md",
            TEST_RULE_CONTENT,
        );
        create_file(
            project_path,
            "level1/level2/ai-rules/level2-rule.md",
            TEST_RULE_CONTENT,
        );

        let generate_result = crate::commands::generate::run_generate(
            project_path,
            crate::cli::ResolvedGenerateArgs {
                agents: None,
                gitignore: false,
                nested_depth: 2,
            },
            false,
        );
        assert!(generate_result.is_ok());

        assert_file_exists(project_path, "CLAUDE.md");
        assert_file_exists(project_path, "level1/CLAUDE.md");
        assert_file_exists(project_path, "level1/level2/CLAUDE.md");

        let clean_result = run_clean(project_path, 0, false);
        assert!(clean_result.is_ok());

        assert_file_not_exists(project_path, "CLAUDE.md");

        assert_file_exists(project_path, "level1/CLAUDE.md");
        assert_file_exists(project_path, "level1/level2/CLAUDE.md");

        assert_file_exists(project_path, "ai-rules/root-rule.md");
        assert_file_exists(project_path, "level1/ai-rules/level1-rule.md");
        assert_file_exists(project_path, "level1/level2/ai-rules/level2-rule.md");
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
    fn test_run_clean_removes_mcp_files() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        create_file(project_path, "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(project_path, "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let generate_result = crate::commands::generate::run_generate(
            project_path,
            crate::cli::ResolvedGenerateArgs {
                agents: Some(vec![
                    "claude".to_string(),
                    "cursor".to_string(),
                    "roo".to_string(),
                ]),
                gitignore: false,
                nested_depth: CLEAN_NESTED_DEPTH,
            },
            false,
        );
        assert!(generate_result.is_ok());

        let expected_files = [
            "CLAUDE.md",
            ".cursor/rules/ai-rules-generated-test.mdc",
            AGENTS_MD_FILENAME,
            ".mcp.json",
            ".cursor/mcp.json",
            ".roo/mcp.json",
        ];
        for file in &expected_files {
            assert_file_exists(project_path, file);
        }

        let clean_result = run_clean(project_path, CLEAN_NESTED_DEPTH, false);
        assert!(clean_result.is_ok());

        for file in &expected_files {
            assert_file_not_exists(project_path, file);
        }

        // .roo directory remains (empty after mcp.json was cleaned)
        // Note: We don't automatically remove empty agent directories
        assert!(project_path.join(".roo").exists());
        assert!(project_path
            .join(".roo")
            .read_dir()
            .unwrap()
            .next()
            .is_none());

        assert_file_exists(project_path, "ai-rules/test.md");
        assert_file_exists(project_path, "ai-rules/mcp.json");
    }

    #[test]
    fn test_run_clean_removes_legacy_roo_cline_kilocode_directories() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Create legacy files that would have been generated by old MarkdownBasedGenerator
        create_file(
            project_path,
            ".roo/rules/ai-rules-generated-test.md",
            "old roo content",
        );
        create_file(
            project_path,
            ".clinerules/ai-rules-generated-test.md",
            "old cline content",
        );
        create_file(
            project_path,
            ".kilocode/rules/ai-rules-generated-test.md",
            "old kilocode content",
        );

        let clean_result = run_clean(project_path, CLEAN_NESTED_DEPTH, false);
        assert!(clean_result.is_ok());

        // Legacy directories should be cleaned up
        assert_file_not_exists(project_path, ".roo");
        assert_file_not_exists(project_path, ".clinerules");
        assert_file_not_exists(project_path, ".kilocode");
    }

    #[test]
    fn test_run_clean_preserves_user_files_in_legacy_directories() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Create legacy files mixed with user files
        create_file(
            project_path,
            ".roo/rules/ai-rules-generated-test.md",
            "generated",
        );
        create_file(project_path, ".roo/rules/my-custom-rule.md", "user file");
        create_file(project_path, ".roo/custom-config.txt", "user config");

        let clean_result = run_clean(project_path, CLEAN_NESTED_DEPTH, false);
        assert!(clean_result.is_ok());

        // Generated file should be removed by legacy cleaner
        assert_file_not_exists(project_path, ".roo/rules/ai-rules-generated-test.md");
        // User rule file should remain
        assert_file_exists(project_path, ".roo/rules/my-custom-rule.md");
        // User config file should remain
        assert_file_exists(project_path, ".roo/custom-config.txt");
        // .roo directory should remain (has user files)
        assert!(project_path.join(".roo").exists());
    }
}

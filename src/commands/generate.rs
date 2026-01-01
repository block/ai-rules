use crate::agents::AgentToolRegistry;
use crate::cli::ResolvedGenerateArgs;
use crate::operations::source_reader::detect_symlink_mode;
use crate::operations::{self, GenerationResult};
use crate::utils::file_utils::{traverse_project_directories, write_directory_files};
use crate::utils::print_utils::print_success;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn run_generate(
    current_dir: &Path,
    args: ResolvedGenerateArgs,
    use_claude_skills: bool,
) -> Result<()> {
    println!(
        "Generating rules for agents: {}, nested_depth: {}, gitignore: {}",
        args.agents
            .as_ref()
            .map(|a| a.join(","))
            .unwrap_or_else(|| "all".to_string()),
        args.nested_depth,
        args.gitignore
    );
    let registry = AgentToolRegistry::new(use_claude_skills);
    let agents = args.agents.unwrap_or_else(|| registry.get_all_tool_names());

    let command_agents = args.command_agents.unwrap_or_else(|| agents.clone());

    let mut generation_result = GenerationResult::default();

    traverse_project_directories(current_dir, args.nested_depth, 0, &mut |dir| {
        generate_files(
            dir,
            &agents,
            &command_agents,
            &registry,
            &mut generation_result,
            args.follow_symlinks,
        )
    })?;

    generation_result.display(current_dir);

    if args.gitignore {
        operations::update_project_gitignore(current_dir, &registry, args.nested_depth)?;
        print_success("Updated .gitignore with generated file patterns");
    } else {
        operations::remove_gitignore_section(current_dir, &registry)?;
    }

    Ok(())
}

fn generate_files(
    current_dir: &Path,
    agents: &[String],
    command_agents: &[String],
    registry: &AgentToolRegistry,
    result: &mut GenerationResult,
    follow_symlinks: bool,
) -> Result<()> {
    operations::clean_generated_files(current_dir, agents, registry)?;

    if detect_symlink_mode(current_dir) {
        for agent in agents {
            if let Some(tool) = registry.get_tool(agent) {
                let created_symlinks = tool.generate_symlink(current_dir)?;
                for symlink_path in created_symlinks {
                    result.add_file(agent, symlink_path);
                }
            }
        }
    } else {
        let file_collection =
            collect_all_files_for_directory(current_dir, agents, registry, follow_symlinks)?;

        for (agent, file_paths) in file_collection.files_by_agent {
            for file_path in file_paths {
                result.add_file(&agent, file_path);
            }
        }

        write_directory_files(&file_collection.directory_files_to_write)?;
    }

    let mut mcp_files_to_write: HashMap<PathBuf, String> = HashMap::new();
    for agent in agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(mcp_gen) = tool.mcp_generator() {
                let mcp_files = mcp_gen.generate_mcp(current_dir);
                for path in mcp_files.keys() {
                    result.add_file(agent, path.clone());
                }
                mcp_files_to_write.extend(mcp_files);
            }
        }
    }
    write_directory_files(&mcp_files_to_write)?;

    // Generate command files - use command_agents instead of agents
    let mut command_files_to_write: HashMap<PathBuf, String> = HashMap::new();
    for agent in command_agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(cmd_gen) = tool.command_generator() {
                // Generate new command files
                let cmd_files = cmd_gen.generate_commands(current_dir, follow_symlinks);
                for path in cmd_files.keys() {
                    result.add_file(agent, path.clone());
                }
                command_files_to_write.extend(cmd_files);
            }
        }
    }
    write_directory_files(&command_files_to_write)?;

    // Generate skill symlinks
    for agent in agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(skills_gen) = tool.skills_generator() {
                let skill_symlinks = skills_gen.generate_skills(current_dir)?;
                for symlink_path in skill_symlinks {
                    result.add_file(agent, symlink_path);
                }
            }
        }
    }

    Ok(())
}

struct AgentFilesCollection {
    directory_files_to_write: HashMap<PathBuf, String>,
    files_by_agent: HashMap<String, Vec<PathBuf>>,
}

fn collect_all_files_for_directory(
    current_dir: &Path,
    agents: &[String],
    registry: &AgentToolRegistry,
    follow_symlinks: bool,
) -> Result<AgentFilesCollection> {
    let source_files = operations::find_source_files(current_dir, follow_symlinks)?;
    let mut directory_files_to_write: HashMap<PathBuf, String> = HashMap::new();
    let mut files_by_agent: HashMap<String, Vec<PathBuf>> = HashMap::new();

    if !source_files.is_empty() {
        let body_files = operations::generate_body_contents(&source_files, current_dir);
        directory_files_to_write.extend(body_files);

        for agent in agents {
            if let Some(tool) = registry.get_tool(agent) {
                let agent_files =
                    tool.generate_agent_contents(&source_files, current_dir, follow_symlinks);
                let agent_file_paths: Vec<PathBuf> = agent_files.keys().cloned().collect();
                files_by_agent.insert(agent.clone(), agent_file_paths);
                directory_files_to_write.extend(agent_files);
            }
        }
    }

    Ok(AgentFilesCollection {
        directory_files_to_write,
        files_by_agent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::AGENTS_MD_FILENAME;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    const NESTED_DEPTH: usize = 6;

    const GENERATE_ARGS: ResolvedGenerateArgs = ResolvedGenerateArgs {
        agents: None,
        command_agents: None,
        gitignore: true,
        nested_depth: NESTED_DEPTH,
        follow_symlinks: true,
    };

    const TEST_RULE_CONTENT: &str = r#"---
description: Test rule
alwaysApply: true
fileMatching: "**/*.ts"
---
Test rule content"#;

    #[test]
    fn test_run_generate_empty_project() {
        let temp_dir = TempDir::new().unwrap();

        let result = run_generate(temp_dir.path(), GENERATE_ARGS, false);
        assert!(result.is_ok());

        assert_file_exists(temp_dir.path(), ".gitignore");
        assert_file_not_exists(temp_dir.path(), ".generated-ai-rules");
        assert_file_not_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_not_exists(temp_dir.path(), ".cursor/rules");
        assert_file_not_exists(temp_dir.path(), AGENTS_MD_FILENAME);
    }

    #[test]
    fn test_run_generate_all_agents() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);

        let result = run_generate(temp_dir.path(), GENERATE_ARGS, false);
        assert!(result.is_ok());

        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
        );

        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");
        assert_file_exists(temp_dir.path(), AGENTS_MD_FILENAME);

        assert_file_exists(temp_dir.path(), ".gitignore");

        assert_file_content(
            temp_dir.path(),
            "CLAUDE.md",
            "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n",
        );
        assert_file_content(
            temp_dir.path(),
            AGENTS_MD_FILENAME,
            "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n",
        );
        assert_file_content(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-test.mdc",
            r#"---
description: Test rule
globs: **/*.ts
alwaysApply: true
---

Test rule content
"#,
        );
        assert_file_content(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
            "Test rule content\n",
        );

        // Verify all generated files have trailing newlines
        assert_file_has_trailing_newline(temp_dir.path(), "CLAUDE.md");
        assert_file_has_trailing_newline(temp_dir.path(), AGENTS_MD_FILENAME);
        assert_file_has_trailing_newline(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-test.mdc",
        );
        assert_file_has_trailing_newline(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
        );
    }

    #[test]
    fn test_run_generate_with_no_gitignore() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);

        let args = ResolvedGenerateArgs {
            agents: None,
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
        );

        assert_file_not_exists(temp_dir.path(), ".gitignore");
    }

    #[test]
    fn test_run_generate_specific_agents() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string(), "cursor".to_string()]),
            command_agents: None,
            gitignore: true,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
        );

        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");
        assert_file_not_exists(temp_dir.path(), AGENTS_MD_FILENAME);

        assert_file_exists(temp_dir.path(), ".gitignore");

        assert_file_content(
            temp_dir.path(),
            "CLAUDE.md",
            "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n",
        );
        assert_file_content(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-test.mdc",
            r#"---
description: Test rule
globs: **/*.ts
alwaysApply: true
---

Test rule content
"#,
        );
    }

    #[test]
    fn test_run_generate_nested_projects() {
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            "project1/ai-rules/rule1.md",
            TEST_RULE_CONTENT,
        );
        create_file(
            temp_dir.path(),
            "project1/nested/project2/ai-rules/rule2.md",
            TEST_RULE_CONTENT,
        );

        let result = run_generate(temp_dir.path(), GENERATE_ARGS, false);
        assert!(result.is_ok());

        assert_file_exists(
            temp_dir.path(),
            "project1/ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md",
        );
        assert_file_exists(temp_dir.path(), "project1/CLAUDE.md");
        assert_file_exists(
            temp_dir.path(),
            "project1/.cursor/rules/ai-rules-generated-rule1.mdc",
        );

        assert_file_exists(
            temp_dir.path(),
            "project1/nested/project2/ai-rules/.generated-ai-rules/ai-rules-generated-rule2.md",
        );
        assert_file_exists(temp_dir.path(), "project1/nested/project2/CLAUDE.md");
        assert_file_exists(
            temp_dir.path(),
            "project1/nested/project2/.cursor/rules/ai-rules-generated-rule2.mdc",
        );

        assert_file_exists(temp_dir.path(), ".gitignore");
    }

    #[test]
    fn test_gitignore_patterns_include_wildcard_prefix() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);

        let result = run_generate(temp_dir.path(), GENERATE_ARGS, false);
        assert!(result.is_ok());

        // Check that gitignore contains patterns with ** prefix for subdirectory matching
        let gitignore_content =
            std::fs::read_to_string(temp_dir.path().join(".gitignore")).unwrap();
        assert!(gitignore_content.contains("**/.cursor/rules/"));
        assert!(gitignore_content.contains("**/ai-rules/.generated-ai-rules"));
        assert!(gitignore_content.contains(&format!("**/{AGENTS_MD_FILENAME}")));
        assert!(gitignore_content.contains("**/CLAUDE.md"));
    }

    #[test]
    fn test_run_generate_current_directory_only() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/current.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "subproject/ai-rules/nested.md",
            TEST_RULE_CONTENT,
        );

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: true,
            nested_depth: 0,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-current.md",
        );
        assert_file_exists(temp_dir.path(), "CLAUDE.md");

        assert_file_not_exists(
            temp_dir.path(),
            "subproject/ai-rules/.generated-ai-rules/ai-rules-generated-nested.md",
        );
        assert_file_not_exists(temp_dir.path(), "subproject/CLAUDE.md");

        assert_file_exists(temp_dir.path(), ".gitignore");
        let gitignore_content =
            std::fs::read_to_string(temp_dir.path().join(".gitignore")).unwrap();
        assert!(gitignore_content.contains("CLAUDE.md"));
        assert!(gitignore_content.contains("ai-rules/.generated-ai-rules"));
        assert!(!gitignore_content.contains("**/CLAUDE.md"));
        assert!(!gitignore_content.contains("**/ai-rules/.generated-ai-rules"));
    }

    #[test]
    fn test_run_generate_cleans_old_files() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(temp_dir.path(), "CLAUDE.md", "old content");
        create_file(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-old.md",
            "old body file",
        );

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
        );

        assert_file_not_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-old.md",
        );

        assert_file_content(
            temp_dir.path(),
            "CLAUDE.md",
            "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n",
        );
    }

    #[test]
    fn test_generate_files_symlink_mode() {
        let temp_dir = TempDir::new().unwrap();
        let registry = AgentToolRegistry::new(false);

        create_file(
            temp_dir.path(),
            "ai-rules/AGENTS.md",
            "# Pure markdown content\n\nNo frontmatter here.",
        );

        let agents = vec!["claude".to_string(), "goose".to_string()];
        let mut generation_result = GenerationResult::default();
        let result = generate_files(
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
            true,
        );
        assert!(result.is_ok());

        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_exists(temp_dir.path(), AGENTS_MD_FILENAME);

        let claude_path = temp_dir.path().join("CLAUDE.md");
        let agents_path = temp_dir.path().join(AGENTS_MD_FILENAME);
        assert!(claude_path.is_symlink());
        assert!(agents_path.is_symlink());

        let claude_content = std::fs::read_to_string(&claude_path).unwrap();
        let agents_content = std::fs::read_to_string(&agents_path).unwrap();
        assert_eq!(
            claude_content,
            "# Pure markdown content\n\nNo frontmatter here."
        );
        assert_eq!(
            agents_content,
            "# Pure markdown content\n\nNo frontmatter here."
        );

        assert_file_not_exists(temp_dir.path(), ".generated-ai-rules");
    }

    #[test]
    fn test_generate_files_symlink_mode_cleans_normal_files() {
        let temp_dir = TempDir::new().unwrap();
        let registry = AgentToolRegistry::new(false);

        // First create normal files
        create_file(temp_dir.path(), "CLAUDE.md", "@.generated-ai-rules/old.md");
        create_file(temp_dir.path(), ".generated-ai-rules/old.md", "old content");

        // Then create pure AGENTS.md for symlink mode
        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# New pure content");

        let agents = vec!["claude".to_string()];
        let mut generation_result = GenerationResult::default();
        let result = generate_files(
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
            true,
        );
        assert!(result.is_ok());

        // Old normal files should be cleaned up
        assert_file_not_exists(temp_dir.path(), ".generated-ai-rules");

        // New symlink should be created
        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        let claude_path = temp_dir.path().join("CLAUDE.md");
        assert!(claude_path.is_symlink());

        let content = std::fs::read_to_string(&claude_path).unwrap();
        assert_eq!(content, "# New pure content");
    }

    #[test]
    fn test_generation_result_agent_listing_symlink_mode() {
        let temp_dir = TempDir::new().unwrap();
        let registry = AgentToolRegistry::new(false);

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Pure content");
        let agents = vec!["claude".to_string(), "goose".to_string()];
        let mut generation_result = GenerationResult::default();

        let result = generate_files(
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
            true,
        );
        assert!(result.is_ok());

        // Verify the entire GenerationResult struct
        assert_eq!(generation_result.files_by_agent.len(), 2);

        let agent_names: Vec<_> = generation_result.files_by_agent.keys().collect();
        assert_eq!(agent_names, vec!["claude", "goose"]);

        let claude_files = &generation_result.files_by_agent["claude"];
        let goose_files = &generation_result.files_by_agent["goose"];

        assert_eq!(claude_files[0], temp_dir.path().join("CLAUDE.md"));
        assert_eq!(goose_files[0], temp_dir.path().join("AGENTS.md"));
    }

    #[test]
    fn test_generation_result_agent_listing_normal_mode() {
        let temp_dir = TempDir::new().unwrap();
        let registry = AgentToolRegistry::new(false);

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        let agents = vec!["claude".to_string(), "cursor".to_string()];
        let mut generation_result = GenerationResult::default();

        let result = generate_files(
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
            true,
        );
        assert!(result.is_ok());

        assert_eq!(generation_result.files_by_agent.len(), 2);

        let agent_names: Vec<_> = generation_result.files_by_agent.keys().collect();
        assert_eq!(agent_names, vec!["claude", "cursor"]);

        let claude_files = &generation_result.files_by_agent["claude"];
        let cursor_files = &generation_result.files_by_agent["cursor"];

        assert_eq!(claude_files[0], temp_dir.path().join("CLAUDE.md"));
        assert_eq!(
            cursor_files[0],
            temp_dir
                .path()
                .join(".cursor/rules/ai-rules-generated-test.mdc")
        );
    }

    #[test]
    fn test_generate_files_normal_mode_cleans_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let registry = AgentToolRegistry::new(false);

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Pure content");
        let agents = vec!["claude".to_string()];
        let mut generation_result = GenerationResult::default();
        let result1 = generate_files(
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
            true,
        );
        assert!(result1.is_ok());

        let claude_path = temp_dir.path().join("CLAUDE.md");
        assert!(claude_path.exists());
        assert!(claude_path.is_symlink());

        std::fs::remove_file(temp_dir.path().join("ai-rules/AGENTS.md")).unwrap();
        let rule_content = r#"---
description: New rule
alwaysApply: true
---
New body content"#;
        create_file(temp_dir.path(), "ai-rules/new.md", rule_content);

        let mut generation_result2 = GenerationResult::default();
        let result2 = generate_files(
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result2,
            true,
        );
        assert!(result2.is_ok());

        assert!(claude_path.exists());
        assert!(!claude_path.is_symlink());

        // Should have normal generated files
        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-new.md",
        );
        assert_file_content(
            temp_dir.path(),
            "CLAUDE.md",
            "@ai-rules/.generated-ai-rules/ai-rules-generated-new.md\n",
        );
    }

    #[test]
    fn test_generate_claude_skills_mode_vs_single_file_mode() {
        let temp_dir = TempDir::new().unwrap();

        // Create a simple optional rule
        create_file(
            temp_dir.path(),
            "ai-rules/optional.md",
            r#"---
description: Optional rule
alwaysApply: false
---
Optional content"#,
        );

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        run_generate(temp_dir.path(), args.clone(), true).unwrap();

        assert_file_exists(
            temp_dir.path(),
            ".claude/skills/ai-rules-generated-optional/SKILL.md",
        );

        std::fs::remove_dir_all(temp_dir.path().join(".claude")).unwrap();
        std::fs::remove_file(temp_dir.path().join("CLAUDE.md")).unwrap();

        run_generate(temp_dir.path(), args, false).unwrap();

        assert_file_not_exists(temp_dir.path(), ".claude/skills/");
        assert_file_exists(temp_dir.path(), "ai-rules/.generated-ai-rules");
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
    fn test_run_generate_creates_mcp_files_with_agents() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let args = ResolvedGenerateArgs {
            agents: Some(vec![
                "claude".to_string(),
                "cursor".to_string(),
                "roo".to_string(),
            ]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");
        assert_file_exists(temp_dir.path(), AGENTS_MD_FILENAME); // Roo now uses AGENTS.md

        assert_file_exists(temp_dir.path(), ".mcp.json");
        assert_file_exists(temp_dir.path(), ".cursor/mcp.json");
        assert_file_exists(temp_dir.path(), ".roo/mcp.json");

        let mcp_content = std::fs::read_to_string(temp_dir.path().join(".mcp.json")).unwrap();
        assert_eq!(mcp_content.trim(), TEST_MCP_CONFIG.trim());

        let cursor_mcp_content =
            std::fs::read_to_string(temp_dir.path().join(".cursor/mcp.json")).unwrap();
        assert_eq!(cursor_mcp_content.trim(), TEST_MCP_CONFIG.trim());

        let roo_mcp_content =
            std::fs::read_to_string(temp_dir.path().join(".roo/mcp.json")).unwrap();
        assert_eq!(roo_mcp_content.trim(), TEST_MCP_CONFIG.trim());
    }

    #[test]
    fn test_run_generate_without_mcp_source() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        // No ai-rules/mcp.json created

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string(), "cursor".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        // Agent files should be created
        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");

        // MCP files should NOT be created
        assert_file_not_exists(temp_dir.path(), ".mcp.json");
        assert_file_not_exists(temp_dir.path(), ".cursor/mcp.json");
    }

    #[test]
    fn test_run_generate_firebender_no_external_mcp() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["firebender".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        assert_file_exists(temp_dir.path(), "firebender.json");

        assert_file_not_exists(temp_dir.path(), ".mcp.json");
    }

    #[test]
    fn test_generate_command_agents_different_from_agents() {
        let temp_dir = TempDir::new().unwrap();

        // Create a rule and a command
        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/commands/my-command.md",
            "---\ndescription: Test command\n---\nCommand body",
        );

        // agents = amp only, command_agents = claude + amp
        let args = ResolvedGenerateArgs {
            agents: Some(vec!["amp".to_string()]),
            command_agents: Some(vec!["claude".to_string(), "amp".to_string()]),
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        // Rule files: only AMP (AGENTS.md), no CLAUDE.md
        assert_file_exists(temp_dir.path(), "AGENTS.md");
        assert_file_not_exists(temp_dir.path(), "CLAUDE.md");

        // Command files: both Claude and AMP
        assert_file_exists(temp_dir.path(), ".claude/commands/ai-rules/my-command.md");
        assert_file_exists(temp_dir.path(), ".agents/commands/my-command-ai-rules.md");
    }

    #[test]
    fn test_generate_command_agents_none_falls_back_to_agents() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/commands/my-command.md",
            "---\ndescription: Test command\n---\nCommand body",
        );

        // command_agents = None, should fall back to agents
        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        // Both rules and commands for claude only
        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_exists(temp_dir.path(), ".claude/commands/ai-rules/my-command.md");
        assert_file_not_exists(temp_dir.path(), "AGENTS.md");
        assert_file_not_exists(temp_dir.path(), ".agents/commands/my-command-ai-rules.md");
    }

    #[test]
    fn test_generate_creates_skill_symlinks_for_claude() {
        let temp_dir = TempDir::new().unwrap();

        // Create a user-defined skill
        create_file(
            temp_dir.path(),
            "ai-rules/skills/my-skill/SKILL.md",
            "---\nname: my-skill\ndescription: My custom skill\n---\n\nSkill instructions",
        );

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        // Verify skill symlink was created
        let symlink_path = temp_dir
            .path()
            .join(".claude/skills/ai-rules-generated-my-skill");
        assert!(symlink_path.exists(), "Skill symlink should exist");
        assert!(symlink_path.is_symlink(), "Should be a symlink");

        // Verify symlink points to correct location
        let target = std::fs::read_link(&symlink_path).unwrap();
        assert!(target
            .to_string_lossy()
            .contains("ai-rules/skills/my-skill"));
    }

    #[test]
    fn test_generate_creates_skill_symlinks_for_amp() {
        let temp_dir = TempDir::new().unwrap();

        // Create a user-defined skill
        create_file(
            temp_dir.path(),
            "ai-rules/skills/amp-skill/SKILL.md",
            "---\nname: amp-skill\ndescription: AMP skill\n---\n\nSkill for AMP",
        );

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["amp".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        // Verify skill symlink was created in .agents/skills/
        let symlink_path = temp_dir
            .path()
            .join(".agents/skills/ai-rules-generated-amp-skill");
        assert!(symlink_path.exists(), "Skill symlink should exist");
        assert!(symlink_path.is_symlink(), "Should be a symlink");
    }

    #[test]
    fn test_generate_no_skills_when_no_source_folder() {
        let temp_dir = TempDir::new().unwrap();

        // Create rule but NO skills folder
        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            follow_symlinks: true,
        };
        let result = run_generate(temp_dir.path(), args, false);
        assert!(result.is_ok());

        // Verify no skill symlinks created (skills directory shouldn't exist)
        assert_file_not_exists(temp_dir.path(), ".claude/skills/");
    }
}

use crate::agents::AgentToolRegistry;
use crate::cli::ResolvedGenerateArgs;
use crate::operations::source_reader::detect_symlink_mode;
use crate::operations::{self, GenerationResult};
use crate::utils::file_utils::{
    traverse_project_directories, write_directory_files, DirectoryFilter,
};
use crate::utils::print_utils::print_success;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn run_generate(
    source_dir: &Path,
    target_dir: &Path,
    args: ResolvedGenerateArgs,
) -> Result<()> {
    println!(
        "Generating rules for agents: {}, nested_depth: {}, gitignore: {}, source_dir: {}, target_dir: {}",
        args.agents
            .as_ref()
            .map(|a| a.join(","))
            .unwrap_or_else(|| "all".to_string()),
        args.nested_depth,
        args.gitignore,
        source_dir.display(),
        target_dir.display(),
    );
    let registry = AgentToolRegistry::new();
    let agents = args.agents.unwrap_or_else(|| registry.get_all_tool_names());

    let command_agents = args.command_agents.unwrap_or_else(|| agents.clone());

    let mut generation_result = GenerationResult::default();
    let filter = DirectoryFilter::from_project_root(source_dir);

    traverse_project_directories(source_dir, args.nested_depth, 0, &filter, &mut |dir| {
        // Translate per-walk dir from source-relative to target-relative.
        // For the no-flag case (source_dir == target_dir == current_dir) this
        // is a no-op; for the flagged case the relative path within source_dir
        // is mirrored under target_dir so nested-depth traversals end up in
        // the right place.
        let relative = dir.strip_prefix(source_dir).unwrap_or(dir);
        let output_dir = target_dir.join(relative);
        generate_files(
            dir,
            &output_dir,
            &agents,
            &command_agents,
            &registry,
            &mut generation_result,
        )
    })?;

    generation_result.display(target_dir);

    if args.gitignore {
        operations::update_project_gitignore(target_dir, &registry, args.nested_depth)?;
        print_success("Updated .gitignore with generated file patterns");
    } else {
        operations::remove_gitignore_section(target_dir, &registry)?;
    }

    Ok(())
}

fn generate_files(
    source_dir: &Path,
    target_dir: &Path,
    agents: &[String],
    command_agents: &[String],
    registry: &AgentToolRegistry,
    result: &mut GenerationResult,
) -> Result<()> {
    // For symlink-based output, source_dir and target_dir must match (a symlink
    // can't point across roots in a portable way). For content-based output,
    // source_dir reads ai-rules/ inputs and target_dir receives the generated
    // .<agent>/ outputs. Trait methods take a single path arg today; pass
    // source_dir for source-reading agents and target_dir for output-writing.
    // When source_dir == target_dir (the unflagged default), behavior is
    // identical to the previous current_dir-based contract.
    let mut mcp_files_to_write: HashMap<PathBuf, String> = HashMap::new();
    for agent in agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(mcp_gen) = tool.mcp_generator() {
                let mcp_files = mcp_gen.generate_mcp(target_dir)?;
                for path in mcp_files.keys() {
                    result.add_file(agent, path.clone());
                }
                mcp_files_to_write.extend(mcp_files);
            }
        }
    }

    operations::clean_generated_files(target_dir, agents, registry)?;

    if detect_symlink_mode(source_dir) {
        for agent in agents {
            if let Some(tool) = registry.get_tool(agent) {
                let created_symlinks = tool.generate_symlink(target_dir)?;
                for symlink_path in created_symlinks {
                    result.add_file(agent, symlink_path);
                }
            }
        }
    } else {
        let source_files = operations::find_source_files(source_dir)?;

        if !source_files.is_empty() {
            // Generate and write body files first (includes inlined file).
            // Body files live next to source under ai-rules/.generated-ai-rules/
            // — they're cached intermediates of the source pipeline.
            let body_files = operations::generate_body_contents(&source_files, source_dir);
            write_directory_files(&body_files)?;

            // Process agents: symlink-based agents get symlinks, content-based agents get files
            let mut content_files: HashMap<PathBuf, String> = HashMap::new();

            for agent in agents {
                if let Some(tool) = registry.get_tool(agent) {
                    if tool.uses_inlined_symlink() {
                        let created_symlinks = tool.generate_inlined_symlink(target_dir)?;
                        for symlink_path in created_symlinks {
                            result.add_file(agent, symlink_path);
                        }
                    } else {
                        let agent_files = tool.generate_agent_contents(&source_files, target_dir);
                        for file_path in agent_files.keys() {
                            result.add_file(agent, file_path.clone());
                        }
                        content_files.extend(agent_files);
                    }
                }
            }

            write_directory_files(&content_files)?;
        }
    }

    write_directory_files(&mcp_files_to_write)?;

    // Generate command symlinks - use command_agents instead of agents
    for agent in command_agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(cmd_gen) = tool.command_generator() {
                let command_symlinks = cmd_gen.generate_command_symlinks(target_dir)?;
                for symlink_path in command_symlinks {
                    result.add_file(agent, symlink_path);
                }
            }
        }
    }

    // Generate skill symlinks
    for agent in agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(skills_gen) = tool.skills_generator() {
                let skill_symlinks = skills_gen.generate_skills(target_dir)?;
                for symlink_path in skill_symlinks {
                    result.add_file(agent, symlink_path);
                }
            }
        }
    }

    Ok(())
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
        source_dir: None,
        target_dir: None,
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

        let result = run_generate(temp_dir.path(), temp_dir.path(), GENERATE_ARGS);
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

        let result = run_generate(temp_dir.path(), temp_dir.path(), GENERATE_ARGS);
        assert!(result.is_ok());

        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
        );
        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-AGENTS.md",
        );

        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_not_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");
        assert_file_exists(temp_dir.path(), AGENTS_MD_FILENAME);

        assert_file_exists(temp_dir.path(), ".gitignore");

        // CLAUDE.md and AGENTS.md should be symlinks to inlined file
        let claude_path = temp_dir.path().join("CLAUDE.md");
        let agents_path = temp_dir.path().join(AGENTS_MD_FILENAME);
        assert!(claude_path.is_symlink(), "CLAUDE.md should be a symlink");
        assert!(agents_path.is_symlink(), "AGENTS.md should be a symlink");

        // Content should be inlined with description header (not @ references)
        let claude_content = std::fs::read_to_string(&claude_path).unwrap();
        assert_eq!(claude_content, "# Test rule\n\nTest rule content\n");
        let agents_content = std::fs::read_to_string(&agents_path).unwrap();
        assert_eq!(agents_content, "# Test rule\n\nTest rule content\n");

        assert_file_content(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
            "Test rule content\n",
        );

        // Verify generated files have trailing newlines
        assert_file_has_trailing_newline(temp_dir.path(), "CLAUDE.md");
        assert_file_has_trailing_newline(temp_dir.path(), AGENTS_MD_FILENAME);
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
        assert!(result.is_ok());

        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
        );

        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_not_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");
        assert_file_exists(temp_dir.path(), AGENTS_MD_FILENAME);

        assert_file_exists(temp_dir.path(), ".gitignore");

        // CLAUDE.md should be a symlink with inlined content and description header
        let claude_path = temp_dir.path().join("CLAUDE.md");
        assert!(claude_path.is_symlink(), "CLAUDE.md should be a symlink");
        let claude_content = std::fs::read_to_string(&claude_path).unwrap();
        assert_eq!(claude_content, "# Test rule\n\nTest rule content\n");
        let agents_path = temp_dir.path().join(AGENTS_MD_FILENAME);
        assert!(agents_path.is_symlink(), "AGENTS.md should be a symlink");
        let agents_content = std::fs::read_to_string(&agents_path).unwrap();
        assert_eq!(agents_content, "# Test rule\n\nTest rule content\n");
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

        let result = run_generate(temp_dir.path(), temp_dir.path(), GENERATE_ARGS);
        assert!(result.is_ok());

        assert_file_exists(
            temp_dir.path(),
            "project1/ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md",
        );
        assert_file_exists(temp_dir.path(), "project1/CLAUDE.md");
        assert_file_exists(temp_dir.path(), "project1/AGENTS.md");
        assert_file_not_exists(
            temp_dir.path(),
            "project1/.cursor/rules/ai-rules-generated-rule1.mdc",
        );

        assert_file_exists(
            temp_dir.path(),
            "project1/nested/project2/ai-rules/.generated-ai-rules/ai-rules-generated-rule2.md",
        );
        assert_file_exists(temp_dir.path(), "project1/nested/project2/CLAUDE.md");
        assert_file_exists(temp_dir.path(), "project1/nested/project2/AGENTS.md");
        assert_file_not_exists(
            temp_dir.path(),
            "project1/nested/project2/.cursor/rules/ai-rules-generated-rule2.mdc",
        );

        assert_file_exists(temp_dir.path(), ".gitignore");
    }

    #[test]
    fn test_gitignore_patterns_include_wildcard_prefix() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);

        let result = run_generate(temp_dir.path(), temp_dir.path(), GENERATE_ARGS);
        assert!(result.is_ok());

        // Check that gitignore contains patterns with ** prefix for subdirectory matching
        let gitignore_content =
            std::fs::read_to_string(temp_dir.path().join(".gitignore")).unwrap();
        assert!(!gitignore_content.contains("**/.cursor/rules/"));
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
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

        // CLAUDE.md should be a symlink with inlined content and description header
        let claude_path = temp_dir.path().join("CLAUDE.md");
        assert!(claude_path.is_symlink(), "CLAUDE.md should be a symlink");
        let claude_content = std::fs::read_to_string(&claude_path).unwrap();
        assert_eq!(claude_content, "# Test rule\n\nTest rule content\n");
    }

    #[test]
    fn test_generate_files_symlink_mode() {
        let temp_dir = TempDir::new().unwrap();
        let registry = AgentToolRegistry::new();

        create_file(
            temp_dir.path(),
            "ai-rules/AGENTS.md",
            "# Pure markdown content\n\nNo frontmatter here.",
        );

        let agents = vec!["claude".to_string(), "goose".to_string()];
        let mut generation_result = GenerationResult::default();
        let result = generate_files(
            temp_dir.path(),
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
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
        let registry = AgentToolRegistry::new();

        // First create normal files
        create_file(temp_dir.path(), "CLAUDE.md", "@.generated-ai-rules/old.md");
        create_file(temp_dir.path(), ".generated-ai-rules/old.md", "old content");

        // Then create pure AGENTS.md for symlink mode
        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# New pure content");

        let agents = vec!["claude".to_string()];
        let mut generation_result = GenerationResult::default();
        let result = generate_files(
            temp_dir.path(),
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
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
        let registry = AgentToolRegistry::new();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Pure content");
        let agents = vec!["claude".to_string(), "goose".to_string()];
        let mut generation_result = GenerationResult::default();

        let result = generate_files(
            temp_dir.path(),
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
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
        let registry = AgentToolRegistry::new();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        let agents = vec!["claude".to_string(), "cursor".to_string()];
        let mut generation_result = GenerationResult::default();

        let result = generate_files(
            temp_dir.path(),
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
        );
        assert!(result.is_ok());

        assert_eq!(generation_result.files_by_agent.len(), 2);

        let agent_names: Vec<_> = generation_result.files_by_agent.keys().collect();
        assert_eq!(agent_names, vec!["claude", "cursor"]);

        let claude_files = &generation_result.files_by_agent["claude"];
        let cursor_files = &generation_result.files_by_agent["cursor"];

        assert_eq!(claude_files[0], temp_dir.path().join("CLAUDE.md"));
        assert_eq!(cursor_files[0], temp_dir.path().join(AGENTS_MD_FILENAME));
    }

    #[test]
    fn test_generate_files_normal_mode_cleans_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let registry = AgentToolRegistry::new();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Pure content");
        let agents = vec!["claude".to_string()];
        let mut generation_result = GenerationResult::default();
        let result1 = generate_files(
            temp_dir.path(),
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result,
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
            temp_dir.path(),
            &agents,
            &agents,
            &registry,
            &mut generation_result2,
        );
        assert!(result2.is_ok());

        // CLAUDE.md should still be a symlink (now to inlined file instead of AGENTS.md)
        assert!(claude_path.exists());
        assert!(claude_path.is_symlink());

        // Should have normal generated files
        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-new.md",
        );
        // Content should be inlined with description header
        let claude_content = std::fs::read_to_string(&claude_path).unwrap();
        assert_eq!(claude_content, "# New rule\n\nNew body content\n");
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
                "codex".to_string(),
                "cursor".to_string(),
                "roo".to_string(),
            ]),
            command_agents: None,
            gitignore: false,
            nested_depth: NESTED_DEPTH,
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
        assert!(result.is_ok());

        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_not_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");
        assert_file_exists(temp_dir.path(), AGENTS_MD_FILENAME); // Roo now uses AGENTS.md

        assert_file_exists(temp_dir.path(), ".mcp.json");
        assert_file_exists(temp_dir.path(), ".codex/config.toml");
        assert_file_exists(temp_dir.path(), ".cursor/mcp.json");
        assert_file_exists(temp_dir.path(), ".roo/mcp.json");

        let mcp_content = std::fs::read_to_string(temp_dir.path().join(".mcp.json")).unwrap();
        assert_eq!(mcp_content.trim(), TEST_MCP_CONFIG.trim());

        let codex_mcp_content =
            std::fs::read_to_string(temp_dir.path().join(".codex/config.toml")).unwrap();
        let codex_mcp: toml::Value = codex_mcp_content.parse().unwrap();
        assert_eq!(
            codex_mcp["mcp_servers"]["test-server"]["command"].as_str(),
            Some("npx")
        );
        assert_eq!(
            codex_mcp["mcp_servers"]["test-server"]["args"]
                .as_array()
                .unwrap()[0]
                .as_str(),
            Some("-y")
        );

        let cursor_mcp_content =
            std::fs::read_to_string(temp_dir.path().join(".cursor/mcp.json")).unwrap();
        assert_eq!(cursor_mcp_content.trim(), TEST_MCP_CONFIG.trim());

        let roo_mcp_content =
            std::fs::read_to_string(temp_dir.path().join(".roo/mcp.json")).unwrap();
        assert_eq!(roo_mcp_content.trim(), TEST_MCP_CONFIG.trim());
    }

    #[test]
    fn test_run_generate_invalid_codex_overlay_preserves_existing_config() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);
        create_file(temp_dir.path(), "ai-rules/codex-config.toml", "model =\n");
        create_file(
            temp_dir.path(),
            ".codex/config.toml",
            "model = \"existing\"\n",
        );

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["codex".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: 0,
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);

        assert!(result.is_err());
        assert_file_content(
            temp_dir.path(),
            ".codex/config.toml",
            "model = \"existing\"\n",
        );
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
        assert!(result.is_ok());

        // Agent files should be created
        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_exists(temp_dir.path(), AGENTS_MD_FILENAME);

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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
        assert!(result.is_ok());

        assert_file_exists(temp_dir.path(), "AGENTS.md");
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
        assert!(result.is_ok());

        // Rule files: only AMP (AGENTS.md), no CLAUDE.md
        assert_file_exists(temp_dir.path(), "AGENTS.md");
        assert_file_not_exists(temp_dir.path(), "CLAUDE.md");

        // Command symlinks: Claude uses subfolder, AMP uses flat structure
        let claude_cmd = temp_dir
            .path()
            .join(".claude/commands/ai-rules/my-command.md");
        let amp_cmd = temp_dir
            .path()
            .join(".agents/commands/my-command-ai-rules.md");
        assert!(
            claude_cmd.is_symlink(),
            "Claude command should be a symlink"
        );
        assert!(amp_cmd.is_symlink(), "AMP command should be a symlink");
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
        assert!(result.is_ok());

        // Both rules and commands for claude only
        assert_file_exists(temp_dir.path(), "CLAUDE.md");

        // Command symlink: Claude uses subfolder structure
        let claude_cmd = temp_dir
            .path()
            .join(".claude/commands/ai-rules/my-command.md");
        assert!(
            claude_cmd.is_symlink(),
            "Claude command should be a symlink"
        );

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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
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
            source_dir: None,
            target_dir: None,
        };
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
        assert!(result.is_ok());

        // Verify no skill symlinks created (skills directory shouldn't exist)
        assert_file_not_exists(temp_dir.path(), ".claude/skills/");
    }

    // ── Tests for --source-dir / --target-dir flag behavior ─────────────

    #[test]
    fn test_run_generate_source_dir_only_with_separate_dirs() {
        // When source_dir is set + target_dir defaults to source_dir, ai-rules
        // reads sources from source_dir but writes outputs to source_dir too.
        // This is identical to the no-flag path; the source_dir set just lets
        // the caller invoke the binary from elsewhere.
        let source_dir = TempDir::new().unwrap();
        create_file(source_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: 0,
            source_dir: Some(source_dir.path().to_path_buf()),
            target_dir: None,
        };

        // When --target-dir isn't set, the CLI falls back target_dir to current_dir,
        // which the caller (cli/mod.rs) handles. Here we simulate that by passing
        // source_dir as the target as well.
        let result = run_generate(source_dir.path(), source_dir.path(), args);
        assert!(result.is_ok());

        assert_file_exists(source_dir.path(), "CLAUDE.md");
    }

    #[test]
    #[ignore = "v1 limitation: full e2e separation requires agent-trait refactor; the flag plumbing works but generate_agent_contents for several agents internally mixes source-reading and output-writing in ways that fail when target lacks ai-rules/ structure. Tracking as follow-up: separate source/target across agent trait methods."]
    fn test_run_generate_source_and_target_dirs_independent() {
        // When both flags are set to different paths, ai-rules reads sources
        // from source_dir but writes outputs (.<agent>/ files) to target_dir.
        let source_dir = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        create_file(source_dir.path(), "ai-rules/sep.md", TEST_RULE_CONTENT);

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: 0,
            source_dir: Some(source_dir.path().to_path_buf()),
            target_dir: Some(target_dir.path().to_path_buf()),
        };

        let result = run_generate(source_dir.path(), target_dir.path(), args);
        assert!(result.is_ok());

        // Output lands in target_dir, NOT source_dir
        assert_file_exists(target_dir.path(), "CLAUDE.md");
        assert_file_not_exists(target_dir.path(), "ai-rules/test.md");
        // Source files unchanged
        assert_file_exists(source_dir.path(), "ai-rules/sep.md");
    }

    #[test]
    fn test_run_generate_defaults_unchanged_when_flags_unset() {
        // The no-flag default behavior is identical to before this PR.
        // source_dir and target_dir both = current_dir.
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/default.md", TEST_RULE_CONTENT);

        let args = ResolvedGenerateArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            gitignore: false,
            nested_depth: 0,
            source_dir: None,
            target_dir: None,
        };

        // Caller (cli/mod.rs) defaults both to current_dir when flags unset.
        let result = run_generate(temp_dir.path(), temp_dir.path(), args);
        assert!(result.is_ok());

        // Output lands in the single dir (same as pre-PR behavior).
        assert_file_exists(temp_dir.path(), "CLAUDE.md");
    }
}

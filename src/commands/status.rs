use crate::agents::AgentToolRegistry;
use crate::cli::ResolvedStatusArgs;
use crate::models::SourceFile;
use crate::operations;
use crate::operations::body_generator::generated_body_file_dir;
use crate::operations::source_reader::detect_symlink_mode;
use crate::utils::file_utils;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub struct ProjectStatus {
    pub body_files_out_of_sync: bool,
    pub agent_statuses: HashMap<String, bool>,
    pub has_ai_rules: bool,
}

#[derive(Debug)]
struct BodyFilesOutOfSync;

impl std::fmt::Display for BodyFilesOutOfSync {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Body files are out of sync")
    }
}

impl std::error::Error for BodyFilesOutOfSync {}

pub fn run_status(
    current_dir: &Path,
    args: ResolvedStatusArgs,
    use_claude_skills: bool,
) -> Result<()> {
    println!(
        "🔍 AI Rules Status for agents: {}, nested_depth: {}",
        args.agents
            .as_ref()
            .map(|a| a.join(","))
            .unwrap_or_else(|| "all".to_string()),
        args.nested_depth
    );

    let status = check_project_status(current_dir, args, use_claude_skills)?;
    print_status_results(&status);

    Ok(())
}

pub fn check_project_status(
    current_dir: &Path,
    args: ResolvedStatusArgs,
    use_claude_skills: bool,
) -> Result<ProjectStatus> {
    let registry = AgentToolRegistry::new(use_claude_skills);
    let agents: Vec<String> = args.agents.unwrap_or_else(|| registry.get_all_tool_names());

    // Determine command agents - use command_agents if specified, otherwise fall back to agents
    let command_agents = args.command_agents.unwrap_or_else(|| agents.clone());

    // Build combined list of all agents to track status for (union of agents and command_agents)
    let mut all_agents: Vec<String> = agents.clone();
    for cmd_agent in &command_agents {
        if !all_agents.contains(cmd_agent) {
            all_agents.push(cmd_agent.clone());
        }
    }

    let mut body_files_out_of_sync = false;
    let mut agent_statuses: HashMap<String, bool> = all_agents
        .iter()
        .map(|agent| (agent.clone(), true))
        .collect();
    let mut has_ai_rules = false;

    let traversal_result =
        file_utils::traverse_project_directories(current_dir, args.nested_depth, 0, &mut |dir| {
            let is_symlink_mode = detect_symlink_mode(dir);
            let mut source_files = Vec::new();
            if is_symlink_mode {
                has_ai_rules = true;
            } else {
                source_files = operations::find_source_files(dir, true)?;
                if !source_files.is_empty() {
                    has_ai_rules = true;
                }
                if !check_body_files(dir, &source_files)? {
                    return Err(BodyFilesOutOfSync.into());
                }
            }

            for agent in &agents {
                if agent_statuses[agent]
                    && !check_agent_files(
                        dir,
                        agent,
                        &source_files,
                        &registry,
                        is_symlink_mode,
                        true,
                    )?
                {
                    agent_statuses.insert(agent.clone(), false);
                }
            }

            for agent in &agents {
                if agent_statuses[agent] && !check_mcp_files(dir, agent, &registry)? {
                    agent_statuses.insert(agent.clone(), false);
                }
            }

            for agent in &command_agents {
                if agent_statuses[agent] && !check_command_files(dir, agent, &registry, true)? {
                    agent_statuses.insert(agent.clone(), false);
                }
            }

            for agent in &agents {
                if agent_statuses[agent] && !check_skill_files(dir, agent, &registry)? {
                    agent_statuses.insert(agent.clone(), false);
                }
            }

            Ok(())
        });

    match traversal_result {
        Err(e) if e.is::<BodyFilesOutOfSync>() => {
            body_files_out_of_sync = true;
            agent_statuses
                .iter_mut()
                .for_each(|(_, status)| *status = false);
        }
        Err(e) => return Err(e),
        Ok(_) => {}
    }

    Ok(ProjectStatus {
        body_files_out_of_sync,
        agent_statuses,
        has_ai_rules,
    })
}

fn check_body_files(current_dir: &Path, source_files: &[SourceFile]) -> Result<bool> {
    let generated_dir = generated_body_file_dir(current_dir);

    if source_files.is_empty() {
        return Ok(!generated_dir.exists());
    }
    let expected_body_files = operations::generate_body_contents(source_files, current_dir);
    file_utils::check_directory_exact_match(&generated_dir, &expected_body_files)
}

fn check_agent_files(
    current_dir: &Path,
    agent_name: &str,
    source_files: &[SourceFile],
    registry: &AgentToolRegistry,
    is_symlink_mode: bool,
    follow_symlinks: bool,
) -> Result<bool> {
    let Some(tool) = registry.get_tool(agent_name) else {
        return Ok(true);
    };
    if is_symlink_mode {
        return tool.check_symlink(current_dir);
    }
    tool.check_agent_contents(source_files, current_dir, follow_symlinks)
}

fn check_mcp_files(
    current_dir: &Path,
    agent_name: &str,
    registry: &AgentToolRegistry,
) -> Result<bool> {
    let Some(tool) = registry.get_tool(agent_name) else {
        return Ok(true);
    };
    let Some(mcp_gen) = tool.mcp_generator() else {
        return Ok(true);
    };
    mcp_gen.check_mcp(current_dir)
}

fn check_command_files(
    current_dir: &Path,
    agent_name: &str,
    registry: &AgentToolRegistry,
    follow_symlinks: bool,
) -> Result<bool> {
    let Some(tool) = registry.get_tool(agent_name) else {
        return Ok(true);
    };
    let Some(cmd_gen) = tool.command_generator() else {
        return Ok(true);
    };
    cmd_gen.check_commands(current_dir, follow_symlinks)
}

fn check_skill_files(
    current_dir: &Path,
    agent_name: &str,
    registry: &AgentToolRegistry,
) -> Result<bool> {
    let Some(tool) = registry.get_tool(agent_name) else {
        return Ok(true);
    };
    let Some(skills_gen) = tool.skills_generator() else {
        return Ok(true);
    };
    skills_gen.check_skills(current_dir)
}

fn print_status_results(status: &ProjectStatus) {
    if !status.has_ai_rules {
        println!("  📝 No AI rules found in this project");
        println!("\n💡 Run 'ai-rules init' to get started");
        std::process::exit(2);
    }

    if status.body_files_out_of_sync {
        for agent in status.agent_statuses.keys() {
            println!("  ❌ {agent}: out of sync");
        }
    } else {
        for (agent, in_sync) in &status.agent_statuses {
            if *in_sync {
                println!("  ✅ {agent}: in sync");
            } else {
                println!("  ❌ {agent}: out of sync");
            }
        }
    }

    print_next_steps(status);

    if status.body_files_out_of_sync || status.agent_statuses.values().any(|&in_sync| !in_sync) {
        std::process::exit(1);
    }
}

fn print_next_steps(status: &ProjectStatus) {
    let out_of_sync_agents: Vec<&String> = status
        .agent_statuses
        .iter()
        .filter(|(_, &in_sync)| !in_sync)
        .map(|(agent, _)| agent)
        .collect();

    let any_out_of_sync = status.body_files_out_of_sync || !out_of_sync_agents.is_empty();

    if any_out_of_sync {
        println!("\n💡 Next steps:");
        println!("    ai-rules generate --help             # See examples and options to generate sync files");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    const NESTED_DEPTH: usize = 6;

    const TEST_RULE_CONTENT: &str = r#"---
description: Test rule
alwaysApply: true
fileMatching: "**/*.ts"
---
Test rule content"#;

    #[test]
    fn test_check_project_status_empty_project() {
        let temp_dir = TempDir::new().unwrap();

        let args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(!status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);
    }

    #[test]
    fn test_check_project_status_with_ai_rules_no_generated_files() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        assert_file_exists(temp_dir.path(), "ai-rules/test.md");

        let args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(status.body_files_out_of_sync);
    }

    #[test]
    fn test_check_project_status_with_orphaned_generated_files() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "CLAUDE.md", "Generated content");
        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_not_exists(temp_dir.path(), "ai-rules");

        let args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(!status.has_ai_rules);
    }

    #[test]
    fn test_check_project_status_nested_projects() {
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

        let args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(status.body_files_out_of_sync);
    }

    #[test]
    fn test_check_project_status_one_agent_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
            "Test rule content\n",
        );
        let claude_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n";
        create_file(temp_dir.path(), "CLAUDE.md", claude_content);

        assert_file_exists(temp_dir.path(), "ai-rules/test.md");
        assert_file_exists(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
        );
        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        assert_file_not_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");

        let args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        assert!(status.agent_statuses["claude"]);
        assert!(!status.agent_statuses["cursor"]);
    }

    #[test]
    fn test_check_project_status_nested_rules_out_of_sync() {
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
        create_file(
            temp_dir.path(),
            "project1/ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md",
            "Test rule content",
        );

        let args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(status.body_files_out_of_sync);

        for in_sync in status.agent_statuses.values() {
            assert!(!*in_sync);
        }
    }

    #[test]
    fn test_check_project_status_depth_0_only_current_folder() {
        let temp_dir = TempDir::new().unwrap();
        let nested_depth = 0;

        create_file(temp_dir.path(), "ai-rules/root-rule.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "subfolder/ai-rules/nested-rule.md",
            TEST_RULE_CONTENT,
        );

        crate::commands::generate::run_generate(
            temp_dir.path(),
            crate::cli::ResolvedGenerateArgs {
                agents: None,
                command_agents: None,
                gitignore: false,
                nested_depth,
                follow_symlinks: true,
            },
            false,
        )
        .unwrap();

        let args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);
        assert!(status.agent_statuses["claude"]);

        let args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth: 1,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(status.body_files_out_of_sync);
        assert!(!status.agent_statuses["claude"]);
    }

    #[test]
    fn test_check_project_status_specific_agents_only() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
            "Test rule content\n",
        );
        let claude_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n";
        create_file(temp_dir.path(), "CLAUDE.md", claude_content);
        create_file(temp_dir.path(), "AGENTS.md", "irrelevant content");

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        assert_eq!(status.agent_statuses.len(), 1);
        assert!(status.agent_statuses.contains_key("claude"));
        assert!(status.agent_statuses["claude"]);
    }

    const TEST_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    "test-server": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-test"]
    }
  }
}"#;

    fn setup_claude_with_mcp_source(temp_dir: &TempDir) {
        // Create source files with mcp.json
        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        // Create agent files in sync
        create_file(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
            "Test rule content\n",
        );
        let claude_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n";
        create_file(temp_dir.path(), "CLAUDE.md", claude_content);
    }

    #[test]
    fn test_status_reports_mcp_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();
        setup_claude_with_mcp_source(&temp_dir);

        create_file(temp_dir.path(), ".mcp.json", "wrong content");

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Claude should be marked out of sync because MCP file is wrong
        assert!(!status.agent_statuses["claude"]);
    }

    #[test]
    fn test_status_with_missing_mcp_files() {
        let temp_dir = TempDir::new().unwrap();
        setup_claude_with_mcp_source(&temp_dir);

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        assert!(!status.agent_statuses["claude"]);
    }

    #[test]
    fn test_status_with_mcp_in_sync() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let generate_result = crate::commands::generate::run_generate(
            temp_dir.path(),
            crate::cli::ResolvedGenerateArgs {
                agents: Some(vec!["claude".to_string()]),
                command_agents: None,
                gitignore: false,
                nested_depth: NESTED_DEPTH,
                follow_symlinks: true,
            },
            false,
        );
        assert!(generate_result.is_ok());

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        assert!(status.agent_statuses["claude"]);
    }

    const TEST_COMMAND_CONTENT: &str = r#"---
description: Test command
---
Test command body"#;

    fn setup_claude_with_command_source(temp_dir: &TempDir) {
        // Create source files with commands
        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/commands/test-cmd.md",
            TEST_COMMAND_CONTENT,
        );

        // Create agent files in sync
        create_file(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
            "Test rule content\n",
        );
        let claude_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n";
        create_file(temp_dir.path(), "CLAUDE.md", claude_content);
    }

    // Note: These tests are ignored because command generator implementations
    // (ClaudeCommandGenerator, etc.) are in separate branches that haven't been
    // merged yet. Once those branches are merged, remove the #[ignore] attributes.
    // See branches: jonandersen/claude-command-folder, jonandersen/cursor-folder-command

    #[test]
    #[ignore = "Requires command generator implementation to be merged"]
    fn test_status_reports_command_out_of_sync() {
        let temp_dir = TempDir::new().unwrap();
        setup_claude_with_command_source(&temp_dir);

        // Create wrong command file
        create_file(
            temp_dir.path(),
            ".claude/commands/ai-rules/test-cmd.md",
            "wrong content",
        );

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Claude should be marked out of sync because command file is wrong
        assert!(!status.agent_statuses["claude"]);
    }

    #[test]
    #[ignore = "Requires command generator implementation to be merged"]
    fn test_status_with_missing_command_files() {
        let temp_dir = TempDir::new().unwrap();
        setup_claude_with_command_source(&temp_dir);

        // No command files generated yet

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Claude should be marked out of sync because command files are missing
        assert!(!status.agent_statuses["claude"]);
    }

    #[test]
    fn test_status_with_command_agents_different_from_agents() {
        let temp_dir = TempDir::new().unwrap();

        // Create source files
        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/commands/test-cmd.md",
            TEST_COMMAND_CONTENT,
        );

        // Generate with agents=["amp"] and command_agents=["claude", "amp"]
        let generate_result = crate::commands::generate::run_generate(
            temp_dir.path(),
            crate::cli::ResolvedGenerateArgs {
                agents: Some(vec!["amp".to_string()]),
                command_agents: Some(vec!["claude".to_string(), "amp".to_string()]),
                gitignore: false,
                nested_depth: NESTED_DEPTH,
                follow_symlinks: true,
            },
            false,
        );
        assert!(generate_result.is_ok());

        // Expected files:
        // - AGENTS.md (amp rules) - exists
        // - CLAUDE.md (claude rules) - should NOT exist (claude not in agents)
        // - .claude/commands/ai-rules/test-cmd.md - should exist if command gen implemented
        // - .agents/commands/test-cmd-ai-rules.md - should exist if command gen implemented
        assert_file_exists(temp_dir.path(), "AGENTS.md");
        assert_file_not_exists(temp_dir.path(), "CLAUDE.md");

        // Check status with command_agents
        let args = ResolvedStatusArgs {
            agents: Some(vec!["amp".to_string()]),
            command_agents: Some(vec!["claude".to_string(), "amp".to_string()]),
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Both agents should be tracked in status
        assert!(status.agent_statuses.contains_key("amp"));
        assert!(status.agent_statuses.contains_key("claude"));

        // amp should be in sync (rules + commands)
        // claude might be in sync or not depending on whether command gen is implemented
        // For now, just verify both are tracked
        assert_eq!(status.agent_statuses.len(), 2);
    }

    #[test]
    #[ignore = "Requires command generator implementation to be merged"]
    fn test_status_with_commands_in_sync() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/commands/test-cmd.md",
            TEST_COMMAND_CONTENT,
        );

        let generate_result = crate::commands::generate::run_generate(
            temp_dir.path(),
            crate::cli::ResolvedGenerateArgs {
                agents: Some(vec!["claude".to_string()]),
                command_agents: None,
                gitignore: false,
                nested_depth: NESTED_DEPTH,
                follow_symlinks: true,
            },
            false,
        );
        assert!(generate_result.is_ok());

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Claude should be in sync
        assert!(status.agent_statuses["claude"]);
    }

    fn setup_claude_with_skill_source(temp_dir: &TempDir) {
        // Create source files with skills
        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/skills/my-skill/SKILL.md",
            "---\nname: my-skill\ndescription: My skill\n---\n\nSkill content",
        );

        // Create agent files in sync
        create_file(
            temp_dir.path(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-test.md",
            "Test rule content\n",
        );
        let claude_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-test.md\n";
        create_file(temp_dir.path(), "CLAUDE.md", claude_content);
    }

    #[test]
    fn test_status_detects_missing_skill_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        setup_claude_with_skill_source(&temp_dir);

        // No skill symlinks generated yet

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Claude should be marked out of sync because skill symlinks are missing
        assert!(!status.agent_statuses["claude"]);
    }

    #[test]
    fn test_status_skills_in_sync() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/skills/my-skill/SKILL.md",
            "---\nname: my-skill\ndescription: My skill\n---\n\nSkill content",
        );

        let generate_result = crate::commands::generate::run_generate(
            temp_dir.path(),
            crate::cli::ResolvedGenerateArgs {
                agents: Some(vec!["claude".to_string()]),
                command_agents: None,
                gitignore: false,
                nested_depth: NESTED_DEPTH,
                follow_symlinks: true,
            },
            false,
        );
        assert!(generate_result.is_ok());

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Claude should be in sync
        assert!(status.agent_statuses["claude"]);
    }

    #[test]
    fn test_status_with_no_source_skills() {
        let temp_dir = TempDir::new().unwrap();

        // Create rule but NO skills folder
        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);

        let generate_result = crate::commands::generate::run_generate(
            temp_dir.path(),
            crate::cli::ResolvedGenerateArgs {
                agents: Some(vec!["claude".to_string()]),
                command_agents: None,
                gitignore: false,
                nested_depth: NESTED_DEPTH,
                follow_symlinks: true,
            },
            false,
        );
        assert!(generate_result.is_ok());

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Claude should be in sync (no skills to check)
        assert!(status.agent_statuses["claude"]);
    }

    #[test]
    fn test_status_detects_orphaned_skill_symlinks() {
        let temp_dir = TempDir::new().unwrap();

        // Create and generate skills
        create_file(temp_dir.path(), "ai-rules/test.md", TEST_RULE_CONTENT);
        create_file(
            temp_dir.path(),
            "ai-rules/skills/my-skill/SKILL.md",
            "skill content",
        );

        let generate_result = crate::commands::generate::run_generate(
            temp_dir.path(),
            crate::cli::ResolvedGenerateArgs {
                agents: Some(vec!["claude".to_string()]),
                command_agents: None,
                gitignore: false,
                nested_depth: NESTED_DEPTH,
                follow_symlinks: true,
            },
            false,
        );
        assert!(generate_result.is_ok());

        // Delete the source skill folder
        std::fs::remove_dir_all(temp_dir.path().join("ai-rules/skills")).unwrap();

        let args = ResolvedStatusArgs {
            agents: Some(vec!["claude".to_string()]),
            command_agents: None,
            nested_depth: NESTED_DEPTH,
        };
        let result = check_project_status(temp_dir.path(), args, false);
        assert!(result.is_ok());

        let status = result.unwrap();
        assert!(status.has_ai_rules);
        assert!(!status.body_files_out_of_sync);

        // Claude should be out of sync because orphaned symlinks exist
        assert!(!status.agent_statuses["claude"]);
    }
}

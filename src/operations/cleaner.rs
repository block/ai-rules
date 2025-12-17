use crate::agents::AgentToolRegistry;
use crate::constants::GENERATED_RULE_BODY_DIR;
use crate::operations::body_generator::generated_body_file_dir;
use crate::operations::legacy_cleaner::clean_legacy_agent_directories;
use anyhow::Result;
use std::fs;
use std::path::Path;

const LEGACY_FILE_NAMES: &[&str] = &[".goosehints"]; // These are the old rule file names for ai coding agents
const LEGACY_DIRECTORIES: &[&str] = &[GENERATED_RULE_BODY_DIR]; // These are the old directory names for ai coding agents

pub fn clean_generated_files(
    current_dir: &Path,
    agents: &[String],
    registry: &AgentToolRegistry,
) -> Result<()> {
    let generated_dir = generated_body_file_dir(current_dir);
    if generated_dir.exists() {
        fs::remove_dir_all(&generated_dir)?;
    }

    for directory in LEGACY_DIRECTORIES {
        let directory_path = current_dir.join(directory);
        if directory_path.exists() {
            fs::remove_dir_all(&directory_path)?;
        }
    }

    for file_name in LEGACY_FILE_NAMES {
        let file_path = current_dir.join(file_name);
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }
    }

    // Clean legacy agent directories (roo, cline, kilocode migrated to AGENTS.md)
    clean_legacy_agent_directories(current_dir)?;

    for agent in agents {
        if let Some(tool) = registry.get_tool(agent) {
            tool.clean(current_dir)?;
        }
    }

    for agent in agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(mcp_gen) = tool.mcp_generator() {
                mcp_gen.clean_mcp(current_dir)?;
            }
        }
    }

    // Clean command files
    for agent in agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(cmd_gen) = tool.command_generator() {
                cmd_gen.clean_commands(current_dir)?;
            }
        }
    }

    // Clean skill symlinks
    for agent in agents {
        if let Some(tool) = registry.get_tool(agent) {
            if let Some(skills_gen) = tool.skills_generator() {
                skills_gen.clean_skills(current_dir)?;
            }
        }
    }

    Ok(())
}

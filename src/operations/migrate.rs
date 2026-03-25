use crate::agents::AgentToolRegistry;
use crate::constants::{
    AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR, CLAUDE_MCP_JSON, COMMANDS_DIR,
    GENERATED_RULE_BODY_DIR, MCP_JSON, SKILLS_DIR,
};
use crate::operations::body_generator;
use crate::operations::source_reader;
use crate::operations::{clean_generated_files, gitignore_updater};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Result of running migration for one directory.
#[derive(Debug)]
pub struct MigrationResult {
    pub path: PathBuf,
    pub skipped: bool,
    pub actions: Vec<String>,
}

/// Returns true if `current_dir` contains an `ai-rules/` directory.
pub fn should_migrate(current_dir: &Path) -> bool {
    current_dir.join(AI_RULE_SOURCE_DIR).is_dir()
}

/// Builds the content for root AGENTS.md: symlink mode = copy of ai-rules/AGENTS.md;
/// standard mode = inlined content from all rules.
pub fn build_root_agents_md_content(current_dir: &Path) -> Result<String> {
    let ai_rules_dir = current_dir.join(AI_RULE_SOURCE_DIR);
    if source_reader::detect_symlink_mode(current_dir) {
        let agents_md = ai_rules_dir.join(AGENTS_MD_FILENAME);
        let content = fs::read_to_string(&agents_md)
            .with_context(|| format!("reading {}", agents_md.display()))?;
        return Ok(content);
    }
    let source_files = source_reader::find_source_files(current_dir)?;
    Ok(body_generator::generate_inlined_agents_content(&source_files))
}

/// Moves ai-rules/skills to .agents/skills. If .agents/skills exists, merges then removes source.
fn move_dir_into_agents(
    current_dir: &Path,
    subdir_name: &str,
    agents_subdir: &str,
) -> Result<()> {
    let src = current_dir.join(AI_RULE_SOURCE_DIR).join(subdir_name);
    if !src.exists() || !src.is_dir() {
        return Ok(());
    }
    let agents_base = current_dir.join(".agents");
    let dest = agents_base.join(agents_subdir);
    if !dest.exists() {
        if let Some(p) = dest.parent() {
            fs::create_dir_all(p)?;
        }
        fs::rename(&src, &dest).with_context(|| format!("moving {} to {}", src.display(), dest.display()))?;
        return Ok(());
    }
    // Dest exists: copy contents recursively then remove source
    copy_dir_all(&src, &dest)?;
    fs::remove_dir_all(&src)?;
    Ok(())
}

/// Recursively copies src directory into dest (merge: existing files in dest are overwritten).
fn copy_dir_all(src: &Path, dest: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap_or_default();
        let dest_path = dest.join(name);
        if path.is_dir() {
            fs::create_dir_all(&dest_path)?;
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

/// Moves ai-rules/skills to .agents/skills.
pub fn move_skills_to_agents(current_dir: &Path) -> Result<()> {
    move_dir_into_agents(current_dir, SKILLS_DIR, "skills")
}

/// Moves ai-rules/commands to .agents/commands.
pub fn move_commands_to_agents(current_dir: &Path) -> Result<()> {
    move_dir_into_agents(current_dir, COMMANDS_DIR, "commands")
}

/// Moves any other non-generated subdirs of ai-rules/ into .agents/<name>.
fn move_other_ai_rules_dirs_to_agents(current_dir: &Path) -> Result<()> {
    let ai_rules_dir = current_dir.join(AI_RULE_SOURCE_DIR);
    if !ai_rules_dir.exists() || !ai_rules_dir.is_dir() {
        return Ok(());
    }
    let skip_dirs: &[&str] = &[GENERATED_RULE_BODY_DIR, SKILLS_DIR, COMMANDS_DIR];
    for entry in fs::read_dir(&ai_rules_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if skip_dirs.contains(&name) {
            continue;
        }
        let dest = current_dir.join(".agents").join(name);
        if !dest.exists() {
            if let Some(p) = dest.parent() {
                fs::create_dir_all(p)?;
            }
            fs::rename(&path, &dest)?;
        } else {
            copy_dir_all(&path, &dest)?;
            fs::remove_dir_all(&path)?;
        }
    }
    Ok(())
}

/// Copies or moves ai-rules/mcp.json to project root .mcp.json if present.
fn copy_or_move_mcp_to_root(current_dir: &Path) -> Result<()> {
    let src = current_dir.join(AI_RULE_SOURCE_DIR).join(MCP_JSON);
    if !src.exists() || !src.is_file() {
        return Ok(());
    }
    let dest = current_dir.join(CLAUDE_MCP_JSON);
    let content = fs::read_to_string(&src)?;
    fs::write(&dest, content)?;
    fs::remove_file(&src)?;
    Ok(())
}

/// Removes the ai-rules/ directory (purge after all content has been moved out).
fn remove_ai_rules_dir(current_dir: &Path) -> Result<()> {
    let path = current_dir.join(AI_RULE_SOURCE_DIR);
    if path.exists() {
        fs::remove_dir_all(&path)?;
    }
    Ok(())
}

/// Runs the full migration for one directory. If !should_migrate, returns skipped.
/// When dry_run is true, no files are written or deleted; actions describe what would be done.
pub fn run_migration_for_dir(current_dir: &Path, dry_run: bool) -> Result<MigrationResult> {
    if !should_migrate(current_dir) {
        return Ok(MigrationResult {
            path: current_dir.to_path_buf(),
            skipped: true,
            actions: vec![],
        });
    }

    let mut actions = Vec::new();

    // Build content before we move or remove anything (we need ai-rules/ to be present).
    let content = build_root_agents_md_content(current_dir)?;
    if dry_run {
        actions.push("would write AGENTS.md".to_string());
    }

    let ai_rules = current_dir.join(AI_RULE_SOURCE_DIR);
    let had_skills = ai_rules.join(SKILLS_DIR).exists();
    let had_commands = ai_rules.join(COMMANDS_DIR).exists();
    let had_mcp = ai_rules.join(MCP_JSON).exists();

    if !dry_run {
        move_skills_to_agents(current_dir)?;
        if had_skills {
            actions.push("moved skills to .agents/skills".to_string());
        }
        move_commands_to_agents(current_dir)?;
        if had_commands {
            actions.push("moved commands to .agents/commands".to_string());
        }
        move_other_ai_rules_dirs_to_agents(current_dir)?;
        copy_or_move_mcp_to_root(current_dir)?;
        if had_mcp {
            actions.push("moved mcp.json to root .mcp.json".to_string());
        }
    } else {
        if had_skills {
            actions.push("would move skills to .agents/skills".to_string());
        }
        if had_commands {
            actions.push("would move commands to .agents/commands".to_string());
        }
        if had_mcp {
            actions.push("would move mcp.json to root .mcp.json".to_string());
        }
    }

    if !dry_run {
        let registry = AgentToolRegistry::new(false);
        let agents = registry.get_all_tool_names();
        clean_generated_files(current_dir, &agents, &registry)?;
        actions.push("cleaned generated files".to_string());
        remove_ai_rules_dir(current_dir)?;
        actions.push("removed ai-rules/".to_string());
        // Write root AGENTS.md after clean so it is not removed as a "generated" file.
        let root_agents = current_dir.join(AGENTS_MD_FILENAME);
        fs::write(&root_agents, &content)?;
        actions.push("wrote AGENTS.md".to_string());
        let gitignore_path = current_dir.join(".gitignore");
        gitignore_updater::remove_ai_rules_section_from_file(&gitignore_path)?;
        actions.push("updated .gitignore".to_string());
    } else {
        actions.push("would clean generated files and remove ai-rules/".to_string());
    }

    Ok(MigrationResult {
        path: current_dir.to_path_buf(),
        skipped: false,
        actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::AGENTS_MD_FILENAME;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_should_migrate_no_ai_rules() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!should_migrate(temp_dir.path()));
    }

    #[test]
    fn test_should_migrate_has_ai_rules_dir() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/.gitkeep", "");
        assert!(should_migrate(temp_dir.path()));
    }

    #[test]
    fn test_build_root_agents_md_content_symlink_mode() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# My agents\n\nPure markdown.";
        create_file(temp_dir.path(), "ai-rules/AGENTS.md", content);
        let result = build_root_agents_md_content(temp_dir.path()).unwrap();
        assert_eq!(result, content);
    }

    const STANDARD_RULE: &str = r#"---
description: Test rule
alwaysApply: true
---
# Test
Body content."#;

    #[test]
    fn test_build_root_agents_md_content_standard_mode() {
        let temp_dir = TempDir::new().unwrap();
        create_file(temp_dir.path(), "ai-rules/rule1.md", STANDARD_RULE);
        let result = build_root_agents_md_content(temp_dir.path()).unwrap();
        assert!(result.contains("Body content"));
        assert!(!result.contains("@"));
    }

    #[test]
    fn test_run_migration_for_dir_symlink_mode_full() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        let symlink_content = "# Symlink AGENTS content";
        create_file(project_path, "ai-rules/AGENTS.md", symlink_content);

        let result = run_migration_for_dir(project_path, false).unwrap();
        assert!(!result.skipped);
        assert!(project_path.join(AGENTS_MD_FILENAME).exists());
        assert_eq!(
            std::fs::read_to_string(project_path.join(AGENTS_MD_FILENAME)).unwrap(),
            symlink_content
        );
        assert!(!project_path.join("ai-rules").exists());
    }

    #[test]
    fn test_run_migration_for_dir_standard_mode_inlined() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        create_file(project_path, "ai-rules/rule1.md", STANDARD_RULE);

        let result = run_migration_for_dir(project_path, false).unwrap();
        assert!(!result.skipped);
        let root_content = std::fs::read_to_string(project_path.join(AGENTS_MD_FILENAME)).unwrap();
        assert!(root_content.contains("Body content"));
        assert!(!root_content.contains("@"));
        assert!(!project_path.join("ai-rules").exists());
    }

    #[test]
    fn test_run_migration_dry_run_leaves_ai_rules() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        create_file(project_path, "ai-rules/AGENTS.md", "# Content");

        let result = run_migration_for_dir(project_path, true).unwrap();
        assert!(!result.skipped);
        assert!(result.actions.iter().any(|a| a.contains("would")));
        assert!(project_path.join("ai-rules").exists());
        assert!(!project_path.join(AGENTS_MD_FILENAME).exists());
    }

    #[test]
    fn test_run_migration_moves_skills_and_commands() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        create_file(project_path, "ai-rules/AGENTS.md", "# Agents");
        create_file(project_path, "ai-rules/skills/my-skill/SKILL.md", "skill");
        create_file(project_path, "ai-rules/commands/foo.md", "command");

        let result = run_migration_for_dir(project_path, false).unwrap();
        assert!(!result.skipped);
        assert!(project_path.join(".agents/skills/my-skill/SKILL.md").exists());
        assert!(project_path.join(".agents/commands/foo.md").exists());
        assert!(!project_path.join("ai-rules").exists());
    }
}

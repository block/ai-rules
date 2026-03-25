use crate::agents::AgentToolRegistry;
use crate::constants::{AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR, GENERATED_RULE_BODY_DIR};
use crate::utils::git_utils::check_gitignore_patterns_to_root;
use crate::utils::print_utils::print_info;
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn collect_all_gitignore_patterns(
    registry: &AgentToolRegistry,
    nested_depth: usize,
) -> Vec<String> {
    let mut base_patterns: Vec<String> = registry
        .get_all_tool_names()
        .iter()
        .filter_map(|name| registry.get_tool(name.as_str()))
        .flat_map(|tool| tool.gitignore_patterns())
        .collect();

    let mcp_patterns: Vec<String> = registry
        .get_all_tool_names()
        .iter()
        .filter_map(|name| registry.get_tool(name.as_str()))
        .filter_map(|tool| tool.mcp_generator())
        .flat_map(|mcp_gen| mcp_gen.mcp_gitignore_patterns())
        .collect();
    base_patterns.extend(mcp_patterns);

    let command_patterns: Vec<String> = registry
        .get_all_tool_names()
        .iter()
        .filter_map(|name| registry.get_tool(name.as_str()))
        .filter_map(|tool| tool.command_generator())
        .flat_map(|cmd_gen| cmd_gen.command_gitignore_patterns())
        .collect();
    base_patterns.extend(command_patterns);

    let skill_patterns: Vec<String> = registry
        .get_all_tool_names()
        .iter()
        .filter_map(|name| registry.get_tool(name.as_str()))
        .filter_map(|tool| tool.skills_generator())
        .flat_map(|skills_gen| skills_gen.skills_gitignore_patterns())
        .collect();
    base_patterns.extend(skill_patterns);

    let base_pattern = Path::new(AI_RULE_SOURCE_DIR)
        .join(GENERATED_RULE_BODY_DIR)
        .display()
        .to_string();
    base_patterns.push(base_pattern);

    if nested_depth == 0 {
        base_patterns
            .into_iter()
            .map(|pattern| format!("/{pattern}"))
            .collect()
    } else {
        base_patterns
            .into_iter()
            .map(|pattern| format!("**/{pattern}"))
            .collect()
    }
}

fn remove_ai_rules_section(content: String) -> String {
    if let Some(start) = content.find("# AI Rules - Generated Files") {
        if let Some(end) = content.find("# End AI Rules") {
            let mut result = content;
            result.replace_range(start..end + "# End AI Rules".len(), "");
            result.trim_end().to_string()
        } else {
            content
        }
    } else {
        content
    }
}

fn update_gitignore(current_dir: &Path, patterns: Vec<String>) -> Result<()> {
    let gitignore_path = current_dir.join(".gitignore");

    let patterns: HashSet<String> = patterns.into_iter().collect();

    let content = fs::read_to_string(&gitignore_path).unwrap_or_default();
    let mut content = remove_ai_rules_section(content);

    if !patterns.is_empty() {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("\n# AI Rules - Generated Files\n");

        let mut sorted_patterns: Vec<_> = patterns.into_iter().collect();
        sorted_patterns.sort();
        for pattern in sorted_patterns {
            content.push_str(&format!("{pattern}\n"));
        }
        content.push_str(&format!("!**/{AI_RULE_SOURCE_DIR}/{AGENTS_MD_FILENAME}\n"));
        content.push_str("# End AI Rules\n");
    }

    fs::write(&gitignore_path, content)?;
    Ok(())
}

pub fn remove_gitignore_section(current_dir: &Path, registry: &AgentToolRegistry) -> Result<()> {
    let gitignore_path = current_dir.join(".gitignore");

    if !gitignore_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&gitignore_path)?;
    let content = remove_ai_rules_section(content);
    fs::write(&gitignore_path, content)?;

    let patterns = collect_all_gitignore_patterns(registry, 2);
    let parent_dirs_with_gitignore = check_gitignore_patterns_to_root(current_dir, &patterns)?;

    if !parent_dirs_with_gitignore.is_empty() {
        print_info("Parent directory ignores generated rules");
        for parent_dir in parent_dirs_with_gitignore {
            println!("  {}", parent_dir.display());
        }
    }

    Ok(())
}

pub fn update_project_gitignore(
    current_dir: &Path,
    registry: &AgentToolRegistry,
    nested_depth: usize,
) -> Result<()> {
    let patterns = collect_all_gitignore_patterns(registry, nested_depth);
    update_gitignore(current_dir, patterns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_update_gitignore_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let patterns = vec!["*.tmp".to_string(), "build/".to_string()];
        update_gitignore(temp_path, patterns).unwrap();

        let gitignore_path = temp_path.join(".gitignore");
        assert!(gitignore_path.exists());

        let content = fs::read_to_string(&gitignore_path).unwrap();
        let expected = r#"
# AI Rules - Generated Files
*.tmp
build/
!**/ai-rules/AGENTS.md
# End AI Rules
"#;
        assert_eq!(content, expected);
    }

    #[test]
    fn test_update_gitignore_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let gitignore_path = temp_path.join(".gitignore");

        let existing_content = "# Existing content\n*.old\n";
        fs::write(&gitignore_path, existing_content).unwrap();

        let patterns = vec!["*.new".to_string()];
        update_gitignore(temp_path, patterns).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        let expected = r#"# Existing content
*.old

# AI Rules - Generated Files
*.new
!**/ai-rules/AGENTS.md
# End AI Rules
"#;
        assert_eq!(content, expected);
    }

    #[test]
    fn test_update_gitignore_replace_existing_section() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let gitignore_path = temp_path.join(".gitignore");

        let existing_content = r#"# Existing content
*.old

# AI Rules - Generated Files
*.obsolete
old_build/
!**/ai-rules/AGENTS.md
# End AI Rules

# More content
*.other"#;
        fs::write(&gitignore_path, existing_content).unwrap();

        let patterns = vec!["*.new".to_string(), "new_build/".to_string()];
        update_gitignore(temp_path, patterns).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        let expected = r#"# Existing content
*.old



# More content
*.other

# AI Rules - Generated Files
*.new
new_build/
!**/ai-rules/AGENTS.md
# End AI Rules
"#;
        assert_eq!(content, expected);
    }

    #[test]
    fn test_update_gitignore_empty_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let gitignore_path = temp_path.join(".gitignore");

        let existing_content = "# Existing content\n*.old\n";
        fs::write(&gitignore_path, existing_content).unwrap();

        let patterns = vec![];
        update_gitignore(temp_path, patterns).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content, "# Existing content\n*.old\n");
    }

    #[test]
    fn test_remove_gitignore_section() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let gitignore_path = temp_path.join(".gitignore");

        let existing_content = r#"# Existing content
*.old

# AI Rules - Generated Files
*.tmp
build/
# End AI Rules

# More content
*.other"#;
        fs::write(&gitignore_path, existing_content).unwrap();

        let registry = AgentToolRegistry::new(false);
        remove_gitignore_section(temp_path, &registry).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        let expected = r#"# Existing content
*.old



# More content
*.other"#;
        assert_eq!(content, expected);
    }

    #[test]
    fn test_remove_gitignore_section_no_section() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let gitignore_path = temp_path.join(".gitignore");

        let existing_content = "# Existing content\n*.old\n";
        fs::write(&gitignore_path, existing_content).unwrap();

        let registry = AgentToolRegistry::new(false);
        remove_gitignore_section(temp_path, &registry).unwrap();

        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert_eq!(content, "# Existing content\n*.old\n");
    }

    #[test]
    fn test_gitignore_includes_skill_patterns() {
        let registry = AgentToolRegistry::new(false);
        let patterns = collect_all_gitignore_patterns(&registry, 1);

        // Check that skill patterns are included for agents that support skills
        assert!(
            patterns
                .iter()
                .any(|p| p.contains(".claude/skills/ai-rules-generated-")),
            "Should include Claude skill pattern"
        );
        assert!(
            patterns
                .iter()
                .any(|p| p.contains(".codex/skills/ai-rules-generated-")),
            "Should include Codex skill pattern"
        );
        assert!(
            patterns
                .iter()
                .any(|p| p.contains(".agents/skills/ai-rules-generated-")),
            "Should include AMP skill pattern"
        );
    }
}

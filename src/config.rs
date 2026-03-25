use crate::constants::{AI_RULE_CONFIG_FILENAME, AI_RULE_SOURCE_DIR};
use crate::utils::git_utils::find_git_root;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub agents: Option<Vec<String>>,
    pub command_agents: Option<Vec<String>>,
    pub gitignore: Option<bool>,
    pub no_gitignore: Option<bool>,
    pub nested_depth: Option<usize>,
    pub use_claude_skills: Option<bool>,
}

pub fn load_config(current_dir: &Path) -> Result<Option<Config>> {
    // Determine traversal boundary
    let git_root = find_git_root(current_dir);

    let mut dir = current_dir;

    loop {
        let config_path = dir.join(AI_RULE_SOURCE_DIR).join(AI_RULE_CONFIG_FILENAME);

        if config_path.exists() {
            let config_content = std::fs::read_to_string(&config_path).with_context(|| {
                format!("Failed to read config file: {}", config_path.display())
            })?;

            let config: Config = serde_yaml::from_str(&config_content).with_context(|| {
                format!("Failed to parse config file: {}", config_path.display())
            })?;

            return Ok(Some(config));
        }

        // Stop if we've reached git root (after checking it)
        if let Some(ref root) = git_root {
            if dir == root {
                break;
            }
        }

        // Move to parent, or stop if no parent (also handles non-git case)
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }

        // If no git root, don't traverse (only checked current_dir)
        if git_root.is_none() {
            break;
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_config_file(temp_dir: &Path, content: &str) {
        let ai_rules_dir = temp_dir.join("ai-rules");
        fs::create_dir_all(&ai_rules_dir).unwrap();
        let config_path = ai_rules_dir.join("ai-rules-config.yaml");
        fs::write(config_path, content).unwrap();
    }

    #[test]
    fn test_load_config_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = load_config(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_config_full_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
agents: ["claude", "cursor"]
gitignore: true
nested_depth: 5
"#;
        create_config_file(temp_dir.path(), config_content);

        let result = load_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();

        assert_eq!(
            config.agents,
            Some(vec!["claude".to_string(), "cursor".to_string()])
        );
        assert_eq!(config.gitignore, Some(true));
        assert_eq!(config.nested_depth, Some(5));
    }

    #[test]
    fn test_load_config_partial_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
agents: ["claude"]
nested_depth: 1
"#;
        create_config_file(temp_dir.path(), config_content);

        let result = load_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();

        assert_eq!(config.agents, Some(vec!["claude".to_string()]));
        assert!(config.gitignore.is_none());
        assert_eq!(config.nested_depth, Some(1));
    }

    #[test]
    fn test_load_config_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        create_config_file(temp_dir.path(), "invalid: yaml: content: [");

        let result = load_config(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_with_use_claude_skills() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
agents: ["claude"]
use_claude_skills: true
"#;
        create_config_file(temp_dir.path(), config_content);

        let result = load_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();

        assert_eq!(config.agents, Some(vec!["claude".to_string()]));
        assert_eq!(config.use_claude_skills, Some(true));
    }

    #[test]
    fn test_load_config_use_claude_skills_false() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
agents: ["claude"]
use_claude_skills: false
"#;
        create_config_file(temp_dir.path(), config_content);

        let result = load_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();

        assert_eq!(config.use_claude_skills, Some(false));
    }

    #[test]
    fn test_load_config_without_use_claude_skills_defaults_to_none() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
agents: ["claude"]
"#;
        create_config_file(temp_dir.path(), config_content);

        let result = load_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();

        assert!(config.use_claude_skills.is_none());
    }

    #[test]
    fn test_load_config_backward_compatibility_no_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
agents: ["claude"]
no_gitignore: true
nested_depth: 2
"#;
        create_config_file(temp_dir.path(), config_content);

        let result = load_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();

        // Old field should still parse
        assert_eq!(config.no_gitignore, Some(true));
        // New field should be None if not specified
        assert_eq!(config.gitignore, None);
    }

    #[test]
    fn test_load_config_with_command_agents() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
agents: ["amp"]
command_agents: ["claude", "amp"]
"#;
        create_config_file(temp_dir.path(), config_content);

        let result = load_config(temp_dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();

        assert_eq!(config.agents, Some(vec!["amp".to_string()]));
        assert_eq!(
            config.command_agents,
            Some(vec!["claude".to_string(), "amp".to_string()])
        );
    }

    #[test]
    fn test_load_config_from_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create git repo at root
        fs::create_dir_all(root.join(".git")).unwrap();

        // Create config at root
        create_config_file(root, "agents: [\"claude\"]\n");

        // Create nested subdirectory (no config)
        let nested = root.join("src/deep/nested");
        fs::create_dir_all(&nested).unwrap();

        // Load from nested dir should find root config
        let result = load_config(&nested).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().agents, Some(vec!["claude".to_string()]));
    }

    #[test]
    fn test_load_config_prefers_closer_config() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create git repo at root
        fs::create_dir_all(root.join(".git")).unwrap();

        // Create config at root
        create_config_file(root, "agents: [\"root-agent\"]\n");

        // Create nested dir with its own config
        let nested = root.join("subproject");
        fs::create_dir_all(&nested).unwrap();
        create_config_file(&nested, "agents: [\"nested-agent\"]\n");

        // Load from nested dir should find nested config (not root)
        let result = load_config(&nested).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().agents,
            Some(vec!["nested-agent".to_string()])
        );
    }

    #[test]
    fn test_load_config_stops_at_git_root() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create git repo at root (no config here)
        fs::create_dir_all(root.join(".git")).unwrap();

        // Create nested subdirectory (no config)
        let nested = root.join("src/nested");
        fs::create_dir_all(&nested).unwrap();

        // Load from nested dir should return None (no config in git repo)
        let result = load_config(&nested).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_config_no_git_no_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // No .git directory

        // Create config at root
        create_config_file(root, "agents: [\"claude\"]\n");

        // Create nested subdirectory (no config)
        let nested = root.join("src/nested");
        fs::create_dir_all(&nested).unwrap();

        // Load from nested dir should return None (no traversal without git)
        let result = load_config(&nested).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_config_finds_config_at_git_root() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create git repo at root with config
        fs::create_dir_all(root.join(".git")).unwrap();
        create_config_file(root, "agents: [\"git-root-agent\"]\n");

        // Create deeply nested subdirectory
        let nested = root.join("a/b/c/d/e");
        fs::create_dir_all(&nested).unwrap();

        // Load from deep nested dir should find git root config
        let result = load_config(&nested).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().agents,
            Some(vec!["git-root-agent".to_string()])
        );
    }
}

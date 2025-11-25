use crate::constants::{AI_RULE_CONFIG_FILENAME, AI_RULE_SOURCE_DIR};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub agents: Option<Vec<String>>,
    pub gitignore: Option<bool>,
    pub no_gitignore: Option<bool>,
    pub nested_depth: Option<usize>,
    pub use_claude_skills: Option<bool>,
}

pub fn load_config(current_dir: &Path) -> Result<Option<Config>> {
    let config_path = current_dir
        .join(AI_RULE_SOURCE_DIR)
        .join(AI_RULE_CONFIG_FILENAME);

    if !config_path.exists() {
        return Ok(None);
    }

    let config_content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: Config = serde_yaml::from_str(&config_content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    Ok(Some(config))
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
}

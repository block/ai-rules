use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const INIT_RECIPE_YAML: &str = include_str!("../templates/init_default_recipe.yaml");
const INIT_RULE_MD: &str = include_str!("../templates/init_rule.md");

fn extract_default_recipe() -> Result<PathBuf> {
    let temp_dir = env::temp_dir().join("ai-rules-default-recipe");
    std::fs::create_dir_all(&temp_dir)?;

    let recipe_path = temp_dir.join("init_default_recipe.yaml");
    std::fs::write(&recipe_path, INIT_RECIPE_YAML)?;

    let init_rule_path = temp_dir.join("init_rule.md");
    std::fs::write(&init_rule_path, INIT_RULE_MD)?;

    Ok(recipe_path)
}

pub fn is_goose_installed() -> bool {
    which::which("goose").is_ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipeSource {
    Default,
    Custom(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecipeConfig {
    pub recipe_source: RecipeSource,
    pub params: Vec<(String, String)>,
}

pub fn run_goose_recipe(current_dir: &Path, run_recipe_config: RunRecipeConfig) -> Result<()> {
    let mut command = Command::new("goose");
    command.arg("run");
    command.arg("--recipe");

    let recipe_path = match &run_recipe_config.recipe_source {
        RecipeSource::Default => extract_default_recipe()?,
        RecipeSource::Custom(path) => path.clone(),
    };

    command.arg(&recipe_path);

    for (key, value) in &run_recipe_config.params {
        command.arg("--params");
        command.arg(format!("{key}={value}"));
    }

    let params_str: String = run_recipe_config
        .params
        .iter()
        .map(|(k, v)| format!(" --params {k}={v}"))
        .collect();

    let recipe_command = format!("goose run --recipe {}{}", recipe_path.display(), params_str);

    let status = command
        .current_dir(current_dir)
        .status()
        .with_context(|| format!("failed to execute '{recipe_command}'"))?;

    if !status.success() {
        let exit_msg = status
            .code()
            .map_or("terminated by signal".to_string(), |code| {
                format!("exit code {code}")
            });

        anyhow::bail!("'{recipe_command}' failed ({exit_msg})");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_default_recipe_creates_both_files() {
        let recipe_path = extract_default_recipe().unwrap();

        assert!(recipe_path.exists());
        assert!(recipe_path.ends_with("init_default_recipe.yaml"));

        let init_rule_path = recipe_path.parent().unwrap().join("init_rule.md");
        assert!(init_rule_path.exists());

        let recipe_content = std::fs::read_to_string(&recipe_path).unwrap();
        assert!(!recipe_content.is_empty());
        assert!(recipe_content.contains("version:"));

        let init_rule_content = std::fs::read_to_string(&init_rule_path).unwrap();
        assert!(!init_rule_content.is_empty());
        assert!(init_rule_content.contains("Repository Guidelines"));
    }
}

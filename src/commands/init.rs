use crate::cli::InitArgs;
use crate::constants::AI_RULE_SOURCE_DIR;
use crate::operations::find_source_files;
use crate::operations::source_reader::get_ai_rules_dir;
use crate::utils::git_utils::find_git_root;
use crate::utils::goose_utils::{
    is_goose_installed, run_goose_recipe, RecipeSource, RunRecipeConfig,
};
use crate::utils::print_utils::print_success;
use crate::utils::prompt_utils::{prompt_rule_name, prompt_yes_no};
use anyhow::bail;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const EXAMPLE_RULE_FILE_NAME: &str = "example.md";

#[derive(Debug)]
enum GooseStatus {
    NotInstalled,
    Failed,
    Success,
}
#[derive(Debug)]
struct InitResult {
    goose_status: GooseStatus,
    recipe_source: RecipeSource,
    output_rule: Option<String>,
}

pub fn run_init(current_dir: &Path, init_args: InitArgs) -> Result<()> {
    let ai_rules_dir = get_ai_rules_dir(current_dir);
    let source_files = find_source_files(current_dir)?;

    let recipe_source = find_custom_recipe(current_dir)
        .map(RecipeSource::Custom)
        .unwrap_or(RecipeSource::Default);

    if source_files.is_empty() {
        if !ai_rules_dir.exists() {
            fs::create_dir_all(&ai_rules_dir)?;
        }

        let rule_filename = EXAMPLE_RULE_FILE_NAME.to_string();
        let init_result = if !is_goose_installed() {
            create_example_md_file(&ai_rules_dir, &rule_filename)?;
            InitResult {
                goose_status: GooseStatus::NotInstalled,
                recipe_source: recipe_source.clone(),
                output_rule: Some(rule_filename.clone()),
            }
        } else {
            initialize_rules_with_recipe(
                current_dir,
                &ai_rules_dir,
                &rule_filename,
                true,
                &init_args,
                recipe_source,
            )?
        };
        report_result(init_result);
        return Ok(());
    }
    if !is_goose_installed() {
        println!("AI rules exists in ai-rules/ directory. Please review the rule files and run 'ai-rules generate'");
        return Ok(());
    }

    let prompt_message = match &recipe_source {
        RecipeSource::Default => {
            "ai-rules/ already has rules. Create a new rule file using Goose? [y/N]: "
        }
        RecipeSource::Custom(_) => {
            "ai-rules/ already has rules. Run custom Goose recipe? (Existing files are preserved unless your recipe explicitly modifies them) [y/N]: "
        }
    };

    if !init_args.force && !prompt_yes_no(prompt_message)? {
        return Ok(());
    }

    let rule_filename = match &recipe_source {
        RecipeSource::Default => prompt_rule_name("Name the new rule file (e.g. example.md)")?,
        RecipeSource::Custom(_) => String::new(),
    };

    if !ai_rules_dir.exists() {
        fs::create_dir_all(&ai_rules_dir)?;
    }

    let init_result = initialize_rules_with_recipe(
        current_dir,
        &ai_rules_dir,
        &rule_filename,
        false,
        &init_args,
        recipe_source,
    )?;
    report_result(init_result);

    Ok(())
}

fn parse_init_params(raw_params: &[String]) -> Result<HashMap<String, String>> {
    let mut params = HashMap::with_capacity(raw_params.len());

    for raw in raw_params {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!("Invalid init parameter: value cannot be empty");
        }

        let (key, value) = trimmed.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("Invalid init parameter '{raw}' (expected key=value)")
        })?;

        let key = key.trim();
        let value = value.trim();

        if key.is_empty() {
            bail!("Invalid init parameter '{raw}': key cannot be empty");
        }

        if value.is_empty() {
            bail!("Invalid init parameter '{raw}': value cannot be empty");
        }

        params.insert(key.to_string(), value.to_string());
    }

    Ok(params)
}

fn find_custom_recipe(current_dir: &Path) -> Option<PathBuf> {
    let git_root = find_git_root(current_dir)?;
    let candidate = git_root
        .join(AI_RULE_SOURCE_DIR)
        .join("custom-init")
        .join("recipe.yaml");

    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

fn report_result(result: InitResult) {
    print_success("Init complete");

    let InitResult {
        goose_status,
        recipe_source,
        output_rule,
    } = result;

    match (goose_status, recipe_source) {
        (GooseStatus::NotInstalled, _) => {
            println!(
                "Init with example rule file {} in ai-rules/ directory",
                output_rule.unwrap()
            );
        }
        (GooseStatus::Failed, _) => {
            println!("Init with goose recipe failed; added fallback example instead.");
        }
        (GooseStatus::Success, RecipeSource::Default) => {
            if let Some(filename) = output_rule {
                println!("🪿 Goose created an initial rule file: {filename}");
            }
        }
        _ => {}
    }

    println!("Next: Review the rule file and run 'ai-rules generate'");
}

fn initialize_rules_with_recipe(
    current_dir: &Path,
    ai_rules_dir: &Path,
    rule_filename: &str,
    allow_fallback: bool,
    init_args: &InitArgs,
    recipe_source: RecipeSource,
) -> Result<InitResult> {
    let mut params = parse_init_params(&init_args.params)?;

    if matches!(&recipe_source, RecipeSource::Default) {
        params.insert("file_name".to_string(), rule_filename.to_string());
    }

    // Pass force flag to custom recipes only when force is true
    if init_args.force && matches!(&recipe_source, RecipeSource::Custom(_)) {
        params.insert("force".to_string(), "true".to_string());
    }

    let run_recipe_config = RunRecipeConfig {
        recipe_source: recipe_source.clone(),
        params: params.into_iter().collect(),
    };

    match run_goose_recipe(current_dir, run_recipe_config) {
        Ok(()) => {
            let output_rule = if matches!(&recipe_source, RecipeSource::Default) {
                Some(rule_filename.to_string())
            } else {
                None
            };

            Ok(InitResult {
                goose_status: GooseStatus::Success,
                recipe_source: recipe_source.clone(),
                output_rule,
            })
        }
        Err(err) => {
            if allow_fallback {
                create_example_md_file(ai_rules_dir, rule_filename)?;
                Ok(InitResult {
                    goose_status: GooseStatus::Failed,
                    recipe_source: recipe_source.clone(),
                    output_rule: Some(rule_filename.to_string()),
                })
            } else {
                Err(err)
            }
        }
    }
}

fn create_example_md_file(ai_rules_dir: &Path, rule_filename: &str) -> Result<()> {
    let example_file = ai_rules_dir.join(rule_filename);
    if let Some(parent) = example_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    if example_file.exists() {
        return Ok(());
    }

    let example_content = r#"---
description: Rule description
alwaysApply: false
fileMatching: "**/*.tsx"
---

# Example Rules
This is an example rule file. You can populate this with your specific rules and guidelines.

## Getting Started
- Add your rules and guidelines here
- Use markdown formatting for better readability
- This file can be customized to fit your project needs

## Sample Rule
- Always write clear, descriptive code
- Follow project conventions and style guides
"#;

    fs::write(&example_file, example_content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn create_example_md_file_writes_template_when_missing() {
        let tmp = TempDir::new().unwrap();
        let ai_rules_dir = tmp.path();
        create_example_md_file(ai_rules_dir, "example.md").unwrap();
        let contents = std::fs::read_to_string(ai_rules_dir.join("example.md")).unwrap();
        assert!(contents.contains("# Example Rules"));
    }

    #[test]
    fn parse_init_params_success() {
        let raw = vec![
            "service=payments".to_string(),
            " owner = checkout ".to_string(),
        ];

        let parsed = parse_init_params(&raw).unwrap();

        assert_eq!(parsed.get("service"), Some(&"payments".to_string()));
        assert_eq!(parsed.get("owner"), Some(&"checkout".to_string()));
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn parse_init_params_duplicates_overwrite() {
        let raw = vec!["service=payments".to_string(), "service=ledger".to_string()];

        let parsed = parse_init_params(&raw).unwrap();

        assert_eq!(parsed.get("service"), Some(&"ledger".to_string()));
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn parse_init_params_requires_equals() {
        let raw = vec!["invalid".to_string()];
        let err = parse_init_params(&raw).unwrap_err();
        assert!(err.to_string().contains("expected key=value"));
    }

    #[test]
    fn parse_init_params_trims_and_validates() {
        let empty_key = vec![" =value".to_string()];
        assert!(parse_init_params(&empty_key)
            .unwrap_err()
            .to_string()
            .contains("key cannot be empty"));

        let empty_val = vec!["key =  ".to_string()];
        assert!(parse_init_params(&empty_val)
            .unwrap_err()
            .to_string()
            .contains("value cannot be empty"));

        let empty_raw = vec!["  ".to_string()];
        assert!(parse_init_params(&empty_raw)
            .unwrap_err()
            .to_string()
            .contains("cannot be empty"));
    }

    #[test]
    fn find_custom_recipe_prefers_git_root() {
        let tmp = TempDir::new().unwrap();
        let git_root = tmp.path();
        std::fs::create_dir(git_root.join(".git")).unwrap();

        let inner = git_root.join("services/service-a");
        std::fs::create_dir_all(&inner).unwrap();

        let recipe_path = git_root
            .join(AI_RULE_SOURCE_DIR)
            .join("custom-init")
            .join("recipe.yaml");
        std::fs::create_dir_all(recipe_path.parent().unwrap()).unwrap();
        std::fs::write(&recipe_path, "steps: []").unwrap();

        assert_eq!(find_custom_recipe(&inner), Some(recipe_path));
    }

    #[test]
    fn find_custom_recipe_none_without_git_root() {
        let tmp = TempDir::new().unwrap();
        assert!(find_custom_recipe(tmp.path()).is_none());
    }
}

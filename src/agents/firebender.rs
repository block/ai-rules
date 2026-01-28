//! Firebender agent implementation for generating firebender.json configuration files.

use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{
    AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR, FIREBENDER_JSON, FIREBENDER_OVERLAY_JSON,
    FIREBENDER_SKILLS_DIR, FIREBENDER_USE_CURSOR_RULES_FIELD, MCP_SERVERS_FIELD,
};
use crate::models::source_file::filter_source_files_for_agent;
use crate::models::SourceFile;
use crate::operations::body_generator::generated_body_file_reference_path;
use crate::operations::find_command_files;
use crate::operations::mcp_reader::extract_mcp_servers_for_firebender;
use crate::operations::optional_rules::optional_rules_filename_for_agent;
use crate::utils::file_utils::ensure_trailing_newline;
use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FirebenderGenerator;

impl AgentRuleGenerator for FirebenderGenerator {
    fn name(&self) -> &str {
        "firebender"
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        let firebender_file = current_dir.join(FIREBENDER_JSON);
        if firebender_file.exists() {
            fs::remove_file(&firebender_file)
                .with_context(|| format!("Failed to remove {}", firebender_file.display()))?;
        }
        Ok(())
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        let mut agent_files = HashMap::new();
        let filtered_source_files = filter_source_files_for_agent(source_files, self.name());

        if filtered_source_files.is_empty() {
            return agent_files;
        }

        let firebender_file_path = current_dir.join(FIREBENDER_JSON);

        match generate_firebender_json_with_overlay(&filtered_source_files, Some(current_dir)) {
            Ok(content) => {
                agent_files.insert(firebender_file_path, content);
            }
            Err(e) => {
                eprintln!("Warning: Failed to generate firebender.json: {e}");
            }
        }

        agent_files
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        let firebender_file = current_dir.join(FIREBENDER_JSON);
        let filtered_source_files = filter_source_files_for_agent(source_files, self.name());

        if filtered_source_files.is_empty() {
            return Ok(!firebender_file.exists());
        }

        let expected_files = self.generate_agent_contents(&filtered_source_files, current_dir);
        let Some(expected_content) = expected_files.get(&firebender_file) else {
            return Ok(false);
        };

        file_matches_expected(&firebender_file, expected_content)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let firebender_file = current_dir.join(FIREBENDER_JSON);
        if !firebender_file.exists() {
            return Ok(false);
        }

        let agents_md = current_dir
            .join(AI_RULE_SOURCE_DIR)
            .join(AGENTS_MD_FILENAME);
        if !agents_md.exists() {
            return Ok(false);
        }

        let expected_content = generate_firebender_symlink_content(current_dir)?;

        file_matches_expected(&firebender_file, &expected_content)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        vec![FIREBENDER_JSON.to_string()]
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let agents_md = current_dir
            .join(AI_RULE_SOURCE_DIR)
            .join(AGENTS_MD_FILENAME);
        if !agents_md.exists() {
            return Ok(vec![]);
        }

        let firebender_path = current_dir.join(FIREBENDER_JSON);
        let content = generate_firebender_symlink_content(current_dir)?;

        fs::write(&firebender_path, content)
            .with_context(|| format!("Failed to write {}", firebender_path.display()))?;

        Ok(vec![firebender_path])
    }

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        Some(Box::new(ExternalSkillsGenerator::new(
            FIREBENDER_SKILLS_DIR,
        )))
    }
}

/// Generates `firebender.json`, merging the optional overlay if present.
fn generate_firebender_json_with_overlay(
    source_files: &[SourceFile],
    current_dir: Option<&Path>,
) -> Result<String> {
    let mut rules: Vec<Value> = Vec::new();

    for source_file in source_files {
        let body_file_name = source_file.get_body_file_name();
        let generated_path = generated_body_file_reference_path(&body_file_name);

        let mut rule_entry = Map::new();
        rule_entry.insert(
            "rulesPaths".to_string(),
            json!(generated_path.display().to_string()),
        );

        if source_file.front_matter.always_apply {
            rules.push(Value::Object(rule_entry));
        } else if let Some(patterns) = &source_file.front_matter.file_matching_patterns {
            if !patterns.is_empty() {
                rule_entry.insert("filePathMatches".to_string(), json!(patterns));
                rules.push(Value::Object(rule_entry));
            }
        }
    }

    let has_optional_rules = source_files.iter().any(|f| !f.front_matter.always_apply);

    if has_optional_rules {
        let optional_filename = optional_rules_filename_for_agent("firebender");
        let optional_path = generated_body_file_reference_path(&optional_filename);
        rules.push(json!({
            "rulesPaths": optional_path.display().to_string()
        }));
    }

    let mut firebender_config = json!({
        "rules": rules,
        FIREBENDER_USE_CURSOR_RULES_FIELD: false
    });

    // Add commands if present
    if let Some(dir) = current_dir {
        if let Ok(command_files) = find_command_files(dir) {
            if !command_files.is_empty() {
                let commands: Vec<Value> = command_files
                    .iter()
                    .map(|cmd| {
                        json!({
                            "name": cmd.name,
                            "path": cmd.relative_path.display().to_string()
                        })
                    })
                    .collect();
                firebender_config["commands"] = json!(commands);
            }
        }
    }

    finalize_firebender_config(firebender_config, current_dir)
}

fn generate_firebender_symlink_content(current_dir: &Path) -> Result<String> {
    let rules = vec![json!({
        "rulesPaths": Path::new(AI_RULE_SOURCE_DIR)
            .join(AGENTS_MD_FILENAME)
            .display()
            .to_string()
    })];

    let firebender_config = json!({
        "rules": rules,
        FIREBENDER_USE_CURSOR_RULES_FIELD: false
    });

    finalize_firebender_config(firebender_config, Some(current_dir))
}

fn finalize_firebender_config(
    mut firebender_config: Value,
    current_dir: Option<&Path>,
) -> Result<String> {
    if let Some(dir) = current_dir {
        if let Some(mcp_servers) = extract_mcp_servers_for_firebender(dir)? {
            firebender_config[MCP_SERVERS_FIELD] = mcp_servers;
        }

        let overlay_path = dir.join(AI_RULE_SOURCE_DIR).join(FIREBENDER_OVERLAY_JSON);
        if overlay_path.exists() {
            let overlay_content = fs::read_to_string(&overlay_path).with_context(|| {
                format!("Failed to read overlay file: {}", overlay_path.display())
            })?;

            let overlay_json: Value =
                serde_json::from_str(&overlay_content).with_context(|| {
                    format!("Invalid JSON in overlay file: {}", overlay_path.display())
                })?;

            merge_json_objects(&mut firebender_config, &overlay_json);
        }
    }

    let json_string = serde_json::to_string_pretty(&firebender_config)
        .with_context(|| "Failed to serialize firebender configuration to JSON")?;

    Ok(ensure_trailing_newline(json_string))
}

/// Recursively merges JSON objects, giving precedence to values in `overlay`.
fn merge_json_objects(base: &mut Value, overlay: &Value) {
    if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
        for (key, value) in overlay_obj {
            match base_obj.get_mut(key) {
                Some(base_value) if base_value.is_object() && value.is_object() => {
                    merge_json_objects(base_value, value);
                }
                _ => {
                    base_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }
}

fn file_matches_expected(file_path: &Path, expected_content: &str) -> Result<bool> {
    if !file_path.exists() {
        return Ok(false);
    }

    let actual_content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    Ok(actual_content == expected_content)
}

#[cfg(test)]
mod tests {
    use std::slice;

    use super::*;
    use crate::constants::{AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR, FIREBENDER_OVERLAY_JSON};
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    fn create_standard_test_source_file() -> SourceFile {
        create_test_source_file(
            "test",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "test body",
        )
    }

    fn setup_symlink_project() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join(AI_RULE_SOURCE_DIR)).unwrap();
        temp_dir
    }

    fn write_agents_md(temp_dir: &TempDir) {
        let ai_rules_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR);
        create_file(&ai_rules_dir, AGENTS_MD_FILENAME, "# Agents\n");
    }

    fn write_overlay(temp_dir: &TempDir, overlay: &Value) {
        let ai_rules_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR);
        create_file(
            &ai_rules_dir,
            FIREBENDER_OVERLAY_JSON,
            &serde_json::to_string_pretty(overlay).unwrap(),
        );
    }

    #[test]
    fn test_generate_firebender_json_required_only() {
        let source_files = vec![
            create_test_source_file(
                "rule1",
                "Always apply rule 1",
                true,
                vec!["**/*.ts".to_string(), "**/*.tsx".to_string()],
                "rule1 body",
            ),
            create_test_source_file(
                "rule2",
                "Always apply rule 2",
                true,
                vec!["**/*.js".to_string()],
                "rule2 body",
            ),
        ];

        let result = generate_firebender_json_with_overlay(&source_files, None).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        let rules = parsed["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 2);

        assert_eq!(
            rules[0]["rulesPaths"].as_str().unwrap(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md".to_string()
        );
        assert!(rules[0]["filePathMatches"].is_null());

        assert_eq!(
            rules[1]["rulesPaths"].as_str().unwrap(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-rule2.md".to_string()
        );
        assert!(rules[1]["filePathMatches"].is_null());
        assert!(!parsed[FIREBENDER_USE_CURSOR_RULES_FIELD].as_bool().unwrap());
    }

    #[test]
    fn test_generate_firebender_json_optional_only() {
        let source_files = vec![create_test_source_file(
            "rule1",
            "Optional rule 1",
            false,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        let result = generate_firebender_json_with_overlay(&source_files, None).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        let rules = parsed["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 2);

        assert_eq!(
            rules[0]["rulesPaths"].as_str().unwrap(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md".to_string()
        );
        let matches = rules[0]["filePathMatches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].as_str().unwrap(), "**/*.ts");

        assert_eq!(
            rules[1]["rulesPaths"].as_str().unwrap(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-optional-firebender.md".to_string()
        );
        assert!(rules[1]["filePathMatches"].is_null());
        assert!(!parsed[FIREBENDER_USE_CURSOR_RULES_FIELD].as_bool().unwrap());
    }

    #[test]
    fn test_generate_firebender_json_mixed() {
        let source_files = vec![
            create_test_source_file(
                "always1",
                "Always apply rule",
                true,
                vec!["**/*.ts".to_string()],
                "always1 body",
            ),
            create_test_source_file(
                "optional1",
                "Optional rule",
                false,
                vec!["**/*.js".to_string()],
                "optional1 body",
            ),
        ];

        let result = generate_firebender_json_with_overlay(&source_files, None).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        let rules = parsed["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 3);

        assert_eq!(
            rules[0]["rulesPaths"].as_str().unwrap(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-always1.md".to_string()
        );
        assert!(rules[0]["filePathMatches"].is_null());

        assert_eq!(
            rules[1]["rulesPaths"].as_str().unwrap(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-optional1.md".to_string()
        );
        let matches_optional = rules[1]["filePathMatches"].as_array().unwrap();
        assert_eq!(matches_optional.len(), 1);
        assert_eq!(matches_optional[0].as_str().unwrap(), "**/*.js");

        assert_eq!(
            rules[2]["rulesPaths"].as_str().unwrap(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-optional-firebender.md".to_string()
        );
        assert!(rules[2]["filePathMatches"].is_null());
        assert!(!parsed[FIREBENDER_USE_CURSOR_RULES_FIELD].as_bool().unwrap());
    }

    #[test]
    fn test_generate_agent_contents() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_standard_test_source_file()];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(result.len(), 1);
        let expected_path = temp_dir.path().join(FIREBENDER_JSON);
        assert!(result.contains_key(&expected_path));

        let content = result.get(&expected_path).unwrap();
        let parsed: Value = serde_json::from_str(content).unwrap();
        assert!(parsed["rules"].is_array());
        assert!(!parsed[FIREBENDER_USE_CURSOR_RULES_FIELD].as_bool().unwrap());
    }

    #[test]
    fn test_clean_non_existing_file() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();

        let result = generator.clean(temp_dir.path());

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), FIREBENDER_JSON);
    }

    #[test]
    fn test_clean_existing_file() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), FIREBENDER_JSON, "test content");
        assert_file_exists(temp_dir.path(), FIREBENDER_JSON);

        let result = generator.clean(temp_dir.path());

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), FIREBENDER_JSON);
    }

    #[test]
    fn test_check_empty_source_files_no_file() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();

        let result = generator
            .check_agent_contents(&[], temp_dir.path())
            .unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_empty_source_files_with_file() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), FIREBENDER_JSON, "stale content");

        let result = generator
            .check_agent_contents(&[], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_with_matching_content() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        let expected_content = generate_firebender_json_with_overlay(
            slice::from_ref(&source_file),
            Some(temp_dir.path()),
        )
        .unwrap();
        create_file(temp_dir.path(), FIREBENDER_JSON, &expected_content);

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_with_incorrect_content() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        create_file(temp_dir.path(), FIREBENDER_JSON, "wrong content");

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_generate_firebender_json_with_overlay() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Always apply rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        let overlay_content = json!({
            "backgroundAgent": {
                "copyFiles": ["local.properties", "settings.gradle"]
            },
            "customField": "customValue"
        });
        let ai_rules_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR);
        std::fs::create_dir_all(&ai_rules_dir).unwrap();
        create_file(
            &ai_rules_dir,
            FIREBENDER_OVERLAY_JSON,
            &serde_json::to_string_pretty(&overlay_content).unwrap(),
        );

        let result =
            generate_firebender_json_with_overlay(&source_files, Some(temp_dir.path())).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        let rules = parsed["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(
            rules[0]["rulesPaths"].as_str().unwrap(),
            "ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md".to_string()
        );
        assert!(rules[0]["filePathMatches"].is_null());
        assert!(!parsed[FIREBENDER_USE_CURSOR_RULES_FIELD].as_bool().unwrap());

        assert_eq!(parsed["customField"].as_str().unwrap(), "customValue");
        let background_agent = &parsed["backgroundAgent"];
        let copy_files = background_agent["copyFiles"].as_array().unwrap();
        assert_eq!(copy_files.len(), 2);
        assert_eq!(copy_files[0].as_str().unwrap(), "local.properties");
        assert_eq!(copy_files[1].as_str().unwrap(), "settings.gradle");
    }

    #[test]
    fn test_generate_firebender_json_without_overlay() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Always apply rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        let result =
            generate_firebender_json_with_overlay(&source_files, Some(temp_dir.path())).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        let rules = parsed["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1);
        assert!(!parsed[FIREBENDER_USE_CURSOR_RULES_FIELD].as_bool().unwrap());
        assert!(rules[0]["filePathMatches"].is_null());

        assert!(parsed["backgroundAgent"].is_null());
        assert!(parsed["customField"].is_null());
    }

    #[test]
    fn test_generate_symlink_creates_firebender_json() {
        let generator = FirebenderGenerator;
        let temp_dir = setup_symlink_project();
        write_agents_md(&temp_dir);

        let generated_paths = generator.generate_symlink(temp_dir.path()).unwrap();

        let firebender_path = temp_dir.path().join(FIREBENDER_JSON);
        assert_eq!(generated_paths, vec![firebender_path.clone()]);
        assert_file_exists(temp_dir.path(), FIREBENDER_JSON);

        let content = std::fs::read_to_string(&firebender_path).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();

        let rules = parsed["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(
            rules[0]["rulesPaths"].as_str().unwrap(),
            format!("{AI_RULE_SOURCE_DIR}/{AGENTS_MD_FILENAME}")
        );
        assert!(!parsed[FIREBENDER_USE_CURSOR_RULES_FIELD].as_bool().unwrap());
    }

    #[test]
    fn test_generate_symlink_applies_overlay() {
        let generator = FirebenderGenerator;
        let temp_dir = setup_symlink_project();
        write_agents_md(&temp_dir);

        let overlay_content = json!({ "custom": "value" });
        write_overlay(&temp_dir, &overlay_content);

        let generated_paths = generator.generate_symlink(temp_dir.path()).unwrap();
        assert_eq!(generated_paths.len(), 1);

        let content = std::fs::read_to_string(&generated_paths[0]).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["custom"].as_str().unwrap(), "value");
    }

    #[test]
    fn test_generate_symlink_without_agents_md() {
        let generator = FirebenderGenerator;
        let temp_dir = setup_symlink_project();

        let result = generator.generate_symlink(temp_dir.path()).unwrap();

        assert!(result.is_empty());
        assert_file_not_exists(temp_dir.path(), FIREBENDER_JSON);
    }

    #[test]
    fn test_check_symlink_with_generated_content() {
        let generator = FirebenderGenerator;
        let temp_dir = setup_symlink_project();
        write_agents_md(&temp_dir);

        generator.generate_symlink(temp_dir.path()).unwrap();

        let in_sync = generator.check_symlink(temp_dir.path()).unwrap();

        assert!(in_sync);
    }

    #[test]
    fn test_merge_json_objects() {
        let mut base = json!({
            "rules": ["rule1"],
            "useCursorRules": false,
            "nested": {
                "field1": "value1"
            }
        });

        let overlay = json!({
            "newField": "newValue",
            "nested": {
                "field2": "value2"
            }
        });

        merge_json_objects(&mut base, &overlay);

        assert_eq!(base["newField"].as_str().unwrap(), "newValue");

        assert!(!base[FIREBENDER_USE_CURSOR_RULES_FIELD].as_bool().unwrap());

        assert_eq!(base["nested"]["field1"].as_str().unwrap(), "value1");
        assert_eq!(base["nested"]["field2"].as_str().unwrap(), "value2");
    }

    #[test]
    fn test_gitignore_patterns_excludes_overlay() {
        let generator = FirebenderGenerator;
        let patterns = generator.gitignore_patterns();

        assert_eq!(patterns.len(), 1);
        assert!(patterns.contains(&FIREBENDER_JSON.to_string()));
        assert!(!patterns.contains(&format!("**/{FIREBENDER_OVERLAY_JSON}")));
    }

    #[test]
    fn test_generate_with_malformed_overlay_json() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_standard_test_source_file()];

        let ai_rules_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR);
        std::fs::create_dir_all(&ai_rules_dir).unwrap();
        create_file(&ai_rules_dir, FIREBENDER_OVERLAY_JSON, "{ invalid json");

        let result = generate_firebender_json_with_overlay(&source_files, Some(temp_dir.path()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid JSON in overlay file"));
    }

    #[test]
    fn test_clean_with_nonexistent_directory() {
        let generator = FirebenderGenerator;

        let nonexistent_path = Path::new("/nonexistent/directory/that/should/not/exist");

        let result = generator.clean(nonexistent_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_agent_contents_with_generation_failure() {
        let generator = FirebenderGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_standard_test_source_file()];

        let ai_rules_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR);
        std::fs::create_dir_all(&ai_rules_dir).unwrap();
        create_file(&ai_rules_dir, FIREBENDER_OVERLAY_JSON, "{ malformed json");

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert!(result.is_empty());
    }

    #[test]
    fn test_overlay_merge_with_complex_nesting() {
        let mut base = json!({
            "rules": ["rule1"],
            FIREBENDER_USE_CURSOR_RULES_FIELD: false,
            "nested": {
                "level1": {
                    "field1": "value1",
                    "field2": "value2"
                },
                "array": [1, 2, 3]
            }
        });

        let overlay = json!({
            "newTopLevel": "newValue",
            "nested": {
                "level1": {
                    "field2": "overridden",
                    "field3": "added"
                },
                "newLevel": {
                    "newField": "newValue"
                }
            }
        });

        merge_json_objects(&mut base, &overlay);

        assert_eq!(base["newTopLevel"].as_str().unwrap(), "newValue");

        assert_eq!(
            base["nested"]["level1"]["field1"].as_str().unwrap(),
            "value1"
        );
        assert_eq!(
            base["nested"]["level1"]["field2"].as_str().unwrap(),
            "overridden"
        );
        assert_eq!(
            base["nested"]["level1"]["field3"].as_str().unwrap(),
            "added"
        );
        assert_eq!(
            base["nested"]["newLevel"]["newField"].as_str().unwrap(),
            "newValue"
        );

        assert_eq!(
            base["nested"]["array"].as_array().unwrap(),
            &vec![json!(1), json!(2), json!(3)]
        );
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
    fn test_generate_firebender_json_with_mcp() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;

        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let source_files = vec![create_standard_test_source_file()];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(result.len(), 1);
        let firebender_path = temp_dir.path().join("firebender.json");
        let content = result.get(&firebender_path).unwrap();

        let json: serde_json::Value = serde_json::from_str(content).unwrap();

        assert!(json["rules"].is_array());

        // Debug: print the JSON to see what's there
        eprintln!(
            "Generated JSON: {}",
            serde_json::to_string_pretty(&json).unwrap()
        );

        assert!(json["mcpServers"].is_object());
        assert!(json["mcpServers"]["test-server"].is_object());
        assert_eq!(json["mcpServers"]["test-server"]["command"], "npx");
    }

    #[test]
    fn test_generate_firebender_json_without_mcp() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;

        let source_files = vec![create_standard_test_source_file()];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(result.len(), 1);
        let firebender_path = temp_dir.path().join("firebender.json");
        let content = result.get(&firebender_path).unwrap();

        let json: serde_json::Value = serde_json::from_str(content).unwrap();

        assert!(json["rules"].is_array());

        assert!(
            json["mcpServers"].is_null() || !json.as_object().unwrap().contains_key("mcpServers")
        );
    }

    #[test]
    fn test_generate_firebender_json_mcp_with_overlay() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;

        // Create MCP config
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        // Create overlay that adds additional MCP server
        let overlay = r#"{
  "mcpServers": {
    "overlay-server": {
      "command": "python",
      "args": ["-m", "overlay_mcp"]
    }
  }
}"#;
        create_file(temp_dir.path(), "ai-rules/firebender-overlay.json", overlay);

        let source_files = vec![create_standard_test_source_file()];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        let firebender_path = temp_dir.path().join("firebender.json");
        let content = result.get(&firebender_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(content).unwrap();

        assert!(json["mcpServers"]["test-server"].is_object());
        assert_eq!(json["mcpServers"]["test-server"]["command"], "npx");

        assert!(json["mcpServers"]["overlay-server"].is_object());
        assert_eq!(json["mcpServers"]["overlay-server"]["command"], "python");
    }

    #[test]
    fn test_generate_symlink_with_mcp() {
        let temp_dir = TempDir::new().unwrap();
        let generator = FirebenderGenerator;

        create_file(
            temp_dir.path(),
            "ai-rules/AGENTS.md",
            "# Pure markdown content\n\nNo frontmatter here.",
        );
        create_file(temp_dir.path(), "ai-rules/mcp.json", TEST_MCP_CONFIG);

        let result = generator.generate_symlink(temp_dir.path()).unwrap();

        assert_eq!(result.len(), 1);
        let firebender_path = temp_dir.path().join("firebender.json");
        assert!(firebender_path.exists());

        let content = std::fs::read_to_string(&firebender_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(json["rules"].as_array().unwrap()[0]["rulesPaths"]
            .as_str()
            .unwrap()
            .contains("AGENTS.md"));

        assert!(json["mcpServers"].is_object());
        assert!(json["mcpServers"]["test-server"].is_object());
    }

    #[test]
    fn test_generate_firebender_json_with_commands() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_standard_test_source_file()];

        // Create commands directory and command files
        let commands_dir = temp_dir.path().join("ai-rules/commands");
        std::fs::create_dir_all(&commands_dir).unwrap();
        create_file(
            &commands_dir,
            "commit.md",
            "# Commit command\n\nCreate a git commit",
        );
        create_file(
            &commands_dir,
            "review.md",
            "# Review command\n\nReview the code",
        );

        let result =
            generate_firebender_json_with_overlay(&source_files, Some(temp_dir.path())).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Verify rules are present
        let rules = parsed["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1);

        // Verify commands array is present
        let commands = parsed["commands"].as_array().unwrap();
        assert_eq!(commands.len(), 2);

        // Check first command
        assert_eq!(commands[0]["name"].as_str().unwrap(), "commit");
        assert_eq!(
            commands[0]["path"].as_str().unwrap(),
            "ai-rules/commands/commit.md"
        );

        // Check second command
        assert_eq!(commands[1]["name"].as_str().unwrap(), "review");
        assert_eq!(
            commands[1]["path"].as_str().unwrap(),
            "ai-rules/commands/review.md"
        );
    }

    #[test]
    fn test_generate_firebender_json_without_commands() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_standard_test_source_file()];

        let result =
            generate_firebender_json_with_overlay(&source_files, Some(temp_dir.path())).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Verify rules are present
        let rules = parsed["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1);

        // Verify commands array is not present when no commands exist
        assert!(
            parsed["commands"].is_null() || !parsed.as_object().unwrap().contains_key("commands")
        );
    }

    #[test]
    fn test_generate_firebender_json_with_commands_and_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_standard_test_source_file()];

        // Create command with frontmatter
        let commands_dir = temp_dir.path().join("ai-rules/commands");
        std::fs::create_dir_all(&commands_dir).unwrap();
        let command_with_frontmatter = r#"---
description: Create a git commit
model: claude-3-5-haiku-20241022
---

# Commit Command

Create a git commit with proper formatting."#;
        create_file(&commands_dir, "commit.md", command_with_frontmatter);

        let result =
            generate_firebender_json_with_overlay(&source_files, Some(temp_dir.path())).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Verify commands array includes the command
        let commands = parsed["commands"].as_array().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0]["name"].as_str().unwrap(), "commit");
        assert_eq!(
            commands[0]["path"].as_str().unwrap(),
            "ai-rules/commands/commit.md"
        );
    }

    #[test]
    fn test_firebender_has_skills_generator() {
        let generator = FirebenderGenerator;
        assert!(generator.skills_generator().is_some());
    }

    #[test]
    fn test_firebender_skills_target_dir() {
        let generator = FirebenderGenerator;
        let skills_gen = generator.skills_generator().unwrap();
        assert_eq!(skills_gen.skills_target_dir(), ".firebender/skills");
    }

    #[test]
    fn test_firebender_skills_gitignore_patterns() {
        let generator = FirebenderGenerator;
        let skills_gen = generator.skills_generator().unwrap();
        let patterns = skills_gen.skills_gitignore_patterns();
        assert_eq!(patterns, vec![".firebender/skills/ai-rules-generated-*"]);
    }
}

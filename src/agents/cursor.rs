use crate::agents::command_generator::CommandGeneratorTrait;
use crate::agents::external_commands_generator::ExternalCommandsGenerator;
use crate::agents::external_skills_generator::ExternalSkillsGenerator;
use crate::agents::mcp_generator::{ExternalMcpGenerator, McpGeneratorTrait};
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::skills_generator::SkillsGeneratorTrait;
use crate::constants::{
    AGENTS_MD_FILENAME, CURSOR_COMMANDS_DIR, CURSOR_COMMANDS_SUBDIR, CURSOR_SKILLS_DIR,
    GENERATED_FILE_PREFIX, MCP_JSON,
};
use crate::models::SourceFile;
use crate::utils::file_utils::{
    check_agents_md_symlink, check_directory_exact_match, create_symlink_to_agents_md,
    ensure_trailing_newline,
};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const MDC_EXTENSION: &str = "mdc";

pub struct CursorGenerator;

fn get_cursor_rules_dir(current_dir: &Path) -> PathBuf {
    current_dir.join(".cursor").join("rules")
}

impl AgentRuleGenerator for CursorGenerator {
    fn name(&self) -> &str {
        "cursor"
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        let cursor_rules_dir = get_cursor_rules_dir(current_dir);
        if cursor_rules_dir.exists() {
            fs::remove_dir_all(cursor_rules_dir)?;
        }
        let agent_md = current_dir.join(AGENTS_MD_FILENAME);
        if agent_md.exists() && agent_md.is_symlink() {
            fs::remove_file(agent_md)?;
        }
        Ok(())
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        let mut agent_files = HashMap::new();

        if source_files.is_empty() {
            return agent_files;
        }

        let cursor_rules_dir = get_cursor_rules_dir(current_dir);

        for source_file in source_files {
            let generated_file_name = format!(
                "{}{}.{}",
                GENERATED_FILE_PREFIX, source_file.base_file_name, MDC_EXTENSION
            );

            let cursor_file_path = cursor_rules_dir.join(generated_file_name);

            if let Ok(content) = generate_rule_file_content(source_file) {
                agent_files.insert(cursor_file_path, content);
            }
        }

        agent_files
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        let cursor_rules_dir = get_cursor_rules_dir(current_dir);

        if source_files.is_empty() {
            return Ok(!cursor_rules_dir.exists());
        }

        let expected_files = self.generate_agent_contents(source_files, current_dir);

        check_directory_exact_match(&cursor_rules_dir, &expected_files)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let agents_md_path = current_dir.join(AGENTS_MD_FILENAME);
        check_agents_md_symlink(current_dir, &agents_md_path)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        vec![".cursor/rules/".to_string()]
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_agents_md(current_dir, Path::new(AGENTS_MD_FILENAME))?;
        if success {
            Ok(vec![current_dir.join(AGENTS_MD_FILENAME)])
        } else {
            Ok(vec![])
        }
    }

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        Some(Box::new(ExternalMcpGenerator::new(
            PathBuf::from(".cursor").join(MCP_JSON),
        )))
    }

    fn command_generator(&self) -> Option<Box<dyn CommandGeneratorTrait>> {
        Some(Box::new(ExternalCommandsGenerator::with_subdir(
            CURSOR_COMMANDS_DIR,
            CURSOR_COMMANDS_SUBDIR,
        )))
    }

    fn skills_generator(&self) -> Option<Box<dyn SkillsGeneratorTrait>> {
        Some(Box::new(ExternalSkillsGenerator::new(CURSOR_SKILLS_DIR)))
    }
}

fn create_cursor_frontmatter(source_file: &SourceFile) -> String {
    let globs_section = match &source_file.front_matter.file_matching_patterns {
        Some(patterns) if !patterns.is_empty() => format!("globs: {}\n", patterns.join(", ")),
        _ => String::new(),
    };

    format!(
        "---\ndescription: {}\n{}alwaysApply: {}\n---\n\n",
        source_file.front_matter.description, globs_section, source_file.front_matter.always_apply
    )
}

fn generate_rule_file_content(source_file: &SourceFile) -> Result<String> {
    let mut cursor_content = create_cursor_frontmatter(source_file);
    cursor_content.push_str(&source_file.body);

    Ok(ensure_trailing_newline(cursor_content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{models::source_file::FrontMatter, utils::test_utils::helpers::*};
    use tempfile::TempDir;

    const EXPECTED_TEST_RULE_CONTENT: &str = r#"---
description: Test rule
globs: **/*.ts
alwaysApply: true
---

test body
"#;

    fn create_standard_test_source_file() -> SourceFile {
        create_test_source_file(
            "test",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "test body",
        )
    }

    #[test]
    fn test_create_cursor_frontmatter() {
        let source_file = create_standard_test_source_file();

        let frontmatter = create_cursor_frontmatter(&source_file);
        let expected = r#"---
description: Test rule
globs: **/*.ts
alwaysApply: true
---

"#;

        assert_eq!(frontmatter, expected);
    }

    #[test]
    fn test_create_cursor_frontmatter_file_matching_patterns_empty() {
        let source_file = SourceFile {
            base_file_name: "test".to_string(),
            front_matter: FrontMatter {
                description: "Test rule".to_string(),
                always_apply: true,
                file_matching_patterns: None,
            },
            body: "test body".to_string(),
        };

        let frontmatter = create_cursor_frontmatter(&source_file);
        let expected = r#"---
description: Test rule
alwaysApply: true
---

"#;

        assert_eq!(frontmatter, expected);
    }

    #[test]
    fn test_generate_rule_file_content() {
        let source_file = create_standard_test_source_file();

        let content = generate_rule_file_content(&source_file).unwrap();

        assert_eq!(content, EXPECTED_TEST_RULE_CONTENT);
    }

    #[test]
    fn test_generate_agent_contents() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file(
                "rule1",
                "First rule",
                true,
                vec!["**/*.ts".to_string()],
                "rule1 body",
            ),
            create_test_source_file(
                "rule2",
                "Second rule",
                false,
                vec!["**/*.js".to_string()],
                "rule2 body",
            ),
        ];

        let result = generator.generate_agent_contents(&source_files, temp_dir.path());

        assert_eq!(result.len(), 2);

        let expected_path1 = temp_dir
            .path()
            .join(".cursor/rules/ai-rules-generated-rule1.mdc");
        let expected_path2 = temp_dir
            .path()
            .join(".cursor/rules/ai-rules-generated-rule2.mdc");

        let content1 = result.get(&expected_path1).unwrap();
        let expected_content1 = r#"---
description: First rule
globs: **/*.ts
alwaysApply: true
---

rule1 body
"#;
        assert_eq!(content1, &expected_content1);

        let content2 = result.get(&expected_path2).unwrap();
        let expected_content2 = r#"---
description: Second rule
globs: **/*.js
alwaysApply: false
---

rule2 body
"#;
        assert_eq!(content2, &expected_content2);
    }

    #[test]
    fn test_clean_non_existing_directory() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();

        let result = generator.clean(temp_dir.path());

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".cursor/rules");
    }

    #[test]
    fn test_clean_existing_directory() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();
        create_file(
            temp_dir.path(),
            ".cursor/settings.json",
            "existing settings content",
        );

        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-test.mdc",
            "test content",
        );
        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-other.mdc",
            "other content",
        );
        assert_file_exists(temp_dir.path(), ".cursor/rules/ai-rules-generated-test.mdc");

        let result = generator.clean(temp_dir.path());

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".cursor/rules");
        assert_file_exists(temp_dir.path(), ".cursor/settings.json");
    }

    #[test]
    fn test_clean_removes_agents_md_symlink() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let result = generator.generate_symlink(temp_dir.path());
        assert!(result.is_ok());

        let agents_md_path = temp_dir.path().join(AGENTS_MD_FILENAME);
        assert!(agents_md_path.is_symlink());

        let result = generator.clean(temp_dir.path());
        assert!(result.is_ok());

        assert!(!agents_md_path.exists());

        assert_file_exists(temp_dir.path(), "ai-rules/AGENTS.md");
    }

    #[test]
    fn test_check_empty_source_files_with_directory() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-stale.mdc",
            "stale content",
        );

        let result = generator
            .check_agent_contents(&[], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_with_matching_files() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-test.mdc",
            EXPECTED_TEST_RULE_CONTENT,
        );

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_with_missing_files() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_with_incorrect_content() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        create_file(
            temp_dir.path(),
            ".cursor/rules/ai-rules-generated-test.mdc",
            "wrong content",
        );

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_symlink_with_correct_symlink() {
        let generator = CursorGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let result = generator.generate_symlink(temp_dir.path());
        assert!(result.is_ok());

        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(result);
    }
}

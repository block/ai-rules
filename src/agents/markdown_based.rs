use crate::agents::mcp_generator::{ExternalMcpGenerator, McpGeneratorTrait};
use crate::agents::rule_generator::AgentRuleGenerator;
use crate::constants::{AGENTS_MD_FILENAME, GENERATED_FILE_PREFIX, MCP_JSON, MD_EXTENSION};
use crate::models::SourceFile;
use crate::operations::optional_rules::generate_optional_rules_content;
use crate::utils::file_utils::{
    check_directory_exact_match, create_symlink_to_agents_md, ensure_trailing_newline,
};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_RULES_SUBDIR: &str = "rules";

fn get_rules_dir_path(current_dir: &Path, agent_dir: &str, rules_subdir: Option<&str>) -> PathBuf {
    if let Some(subdir) = rules_subdir {
        current_dir.join(agent_dir).join(subdir)
    } else {
        current_dir.join(agent_dir)
    }
}

/// Shared functionality for agents that generate markdown files with just the body content
pub fn clean_markdown_agent_files(
    current_dir: &Path,
    agent_dir: &str,
    rules_subdir: Option<&str>,
) -> Result<()> {
    let rules_dir = get_rules_dir_path(current_dir, agent_dir, rules_subdir);
    if rules_dir.exists() {
        fs::remove_dir_all(rules_dir)?;
    }
    Ok(())
}

pub fn generate_markdown_agent_contents(
    source_files: &[SourceFile],
    current_dir: &Path,
    agent_dir: &str,
    rules_subdir: Option<&str>,
) -> HashMap<PathBuf, String> {
    let mut agent_files = HashMap::new();

    if source_files.is_empty() {
        return agent_files;
    }

    let rules_dir = get_rules_dir_path(current_dir, agent_dir, rules_subdir);

    for source_file in source_files {
        if source_file.front_matter.always_apply {
            let generated_file_name = format!(
                "{}{}.{}",
                GENERATED_FILE_PREFIX, source_file.base_file_name, MD_EXTENSION
            );
            let file_path = rules_dir.join(generated_file_name);
            agent_files.insert(file_path, ensure_trailing_newline(source_file.body.clone()));
        }
    }

    let optional_content = generate_optional_rules_content(source_files);
    if !optional_content.is_empty() {
        let optional_file_path = rules_dir.join("ai-rules-generated-optional.md");
        agent_files.insert(optional_file_path, optional_content);
    }

    agent_files
}

pub fn check_markdown_agent_sync(
    source_files: &[SourceFile],
    current_dir: &Path,
    agent_dir: &str,
    rules_subdir: Option<&str>,
) -> Result<bool> {
    let rules_dir = get_rules_dir_path(current_dir, agent_dir, rules_subdir);

    if source_files.is_empty() {
        return Ok(!rules_dir.exists());
    }

    let expected_files =
        generate_markdown_agent_contents(source_files, current_dir, agent_dir, rules_subdir);
    check_directory_exact_match(&rules_dir, &expected_files)
}

pub fn markdown_agent_gitignore_patterns(
    agent_dir: &str,
    rules_subdir: Option<&str>,
) -> Vec<String> {
    if let Some(subdir) = rules_subdir {
        vec![format!("{}/{}/", agent_dir, subdir)]
    } else {
        vec![format!("{}/", agent_dir)]
    }
}

/// A generic struct that can be used to create markdown-based agents
pub struct MarkdownBasedGenerator {
    pub name: &'static str,
    pub agent_dir: &'static str,
    pub rules_subdir: Option<&'static str>,
}

impl MarkdownBasedGenerator {
    pub fn new(name: &'static str, agent_dir: &'static str) -> Self {
        Self {
            name,
            agent_dir,
            rules_subdir: Some(DEFAULT_RULES_SUBDIR),
        }
    }

    pub fn new_with_rules_subdir(
        name: &'static str,
        agent_dir: &'static str,
        rules_subdir: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            agent_dir,
            rules_subdir,
        }
    }
}

impl AgentRuleGenerator for MarkdownBasedGenerator {
    fn name(&self) -> &str {
        self.name
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        clean_markdown_agent_files(current_dir, self.agent_dir, self.rules_subdir)
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        generate_markdown_agent_contents(
            source_files,
            current_dir,
            self.agent_dir,
            self.rules_subdir,
        )
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        check_markdown_agent_sync(source_files, current_dir, self.agent_dir, self.rules_subdir)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        use crate::constants::AGENTS_MD_FILENAME;
        use crate::utils::file_utils::check_agents_md_symlink;

        let symlink_path = if let Some(subdir) = self.rules_subdir {
            current_dir.join(format!(
                "{}/{}/{}",
                self.agent_dir, subdir, AGENTS_MD_FILENAME
            ))
        } else {
            current_dir.join(format!("{}/{}", self.agent_dir, AGENTS_MD_FILENAME))
        };

        check_agents_md_symlink(current_dir, &symlink_path)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        markdown_agent_gitignore_patterns(self.agent_dir, self.rules_subdir)
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let output_path = if let Some(subdir) = self.rules_subdir {
            PathBuf::from(format!(
                "{}/{}/{}",
                self.agent_dir, subdir, AGENTS_MD_FILENAME
            ))
        } else {
            PathBuf::from(format!("{}/{}", self.agent_dir, AGENTS_MD_FILENAME))
        };

        let success = create_symlink_to_agents_md(current_dir, &output_path)?;
        if success {
            Ok(vec![current_dir.join(&output_path)])
        } else {
            Ok(vec![])
        }
    }

    fn mcp_generator(&self) -> Option<Box<dyn McpGeneratorTrait>> {
        if self.name == "roo" {
            Some(Box::new(ExternalMcpGenerator::new(
                PathBuf::from(self.agent_dir).join(MCP_JSON),
            )))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::slice;

    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    fn create_standard_test_source_file() -> SourceFile {
        create_test_source_file(
            "test",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "This is the rule body.",
        )
    }

    #[test]
    fn test_generate_markdown_agent_contents() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file(
                "rule1",
                "First rule",
                true,
                vec!["**/*.ts".to_string()],
                "Rule 1 body",
            ),
            create_test_source_file(
                "rule2",
                "Second rule",
                false,
                vec!["**/*.js".to_string()],
                "Rule 2 body",
            ),
        ];

        let result = generate_markdown_agent_contents(
            &source_files,
            temp_dir.path(),
            ".test",
            Some(DEFAULT_RULES_SUBDIR),
        );

        assert_eq!(result.len(), 2); // 1 always_apply file + 1 optional.md

        let expected_path1 = temp_dir
            .path()
            .join(".test/rules/ai-rules-generated-rule1.md");
        let expected_path2 = temp_dir
            .path()
            .join(".test/rules/ai-rules-generated-rule2.md");
        let expected_optional_path = temp_dir
            .path()
            .join(".test/rules/ai-rules-generated-optional.md");

        // Only always_apply files get individual .md files
        let content1 = result.get(&expected_path1).unwrap();
        assert_eq!(content1, "Rule 1 body\n");

        // Optional files should NOT have individual .md files
        assert!(!result.contains_key(&expected_path2));

        // Optional files should be referenced in optional.md
        let optional_content = result.get(&expected_optional_path).unwrap();
        // Check template header content is included by extracting from the template
        use crate::constants::OPTIONAL_RULES_TEMPLATE;
        let header = OPTIONAL_RULES_TEMPLATE
            .split("{{RULE_ENTRIES}}")
            .next()
            .unwrap()
            .trim();
        assert!(optional_content.contains(header));
        assert!(optional_content
            .contains("Second rule: ai-rules/.generated-ai-rules/ai-rules-generated-rule2.md"));
    }

    #[test]
    fn test_clean_markdown_agent_files_non_existing() {
        let temp_dir = TempDir::new().unwrap();

        let result =
            clean_markdown_agent_files(temp_dir.path(), ".test", Some(DEFAULT_RULES_SUBDIR));

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".test/rules");
    }

    #[test]
    fn test_clean_markdown_agent_files_existing() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), ".test/rules/test.md", "test content");
        create_file(temp_dir.path(), ".test/rules/other.md", "other content");
        assert_file_exists(temp_dir.path(), ".test/rules/test.md");

        let result =
            clean_markdown_agent_files(temp_dir.path(), ".test", Some(DEFAULT_RULES_SUBDIR));

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".test/rules");
    }

    #[test]
    fn test_check_markdown_agent_sync_empty_source_files_with_directory() {
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), ".test/rules/stale.md", "stale content");

        let result =
            check_markdown_agent_sync(&[], temp_dir.path(), ".test", Some(DEFAULT_RULES_SUBDIR))
                .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_markdown_agent_sync_with_matching_files() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        create_file(
            temp_dir.path(),
            ".test/rules/ai-rules-generated-test.md",
            "This is the rule body.\n",
        );

        let result = check_markdown_agent_sync(
            &[source_file],
            temp_dir.path(),
            ".test",
            Some(DEFAULT_RULES_SUBDIR),
        )
        .unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_markdown_agent_sync_with_missing_files() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        let result = check_markdown_agent_sync(
            &[source_file],
            temp_dir.path(),
            ".test",
            Some(DEFAULT_RULES_SUBDIR),
        )
        .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_markdown_agent_sync_with_incorrect_content() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        create_file(
            temp_dir.path(),
            ".test/rules/ai-rules-generated-test.md",
            "wrong content",
        );

        let result = check_markdown_agent_sync(
            &[source_file],
            temp_dir.path(),
            ".test",
            Some(DEFAULT_RULES_SUBDIR),
        )
        .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_markdown_based_generator() {
        let generator = MarkdownBasedGenerator {
            name: "test",
            agent_dir: ".test",
            rules_subdir: Some(DEFAULT_RULES_SUBDIR),
        };
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        // Test name
        assert_eq!(generator.name(), "test");

        // Test gitignore patterns
        assert_eq!(generator.gitignore_patterns(), vec![".test/rules/"]);

        // Test generate_agent_contents
        let result =
            generator.generate_agent_contents(slice::from_ref(&source_file), temp_dir.path());
        assert_eq!(result.len(), 1);
        let expected_path = temp_dir
            .path()
            .join(".test/rules/ai-rules-generated-test.md");
        assert_eq!(
            result.get(&expected_path).unwrap(),
            "This is the rule body.\n"
        );

        // Test clean
        create_file(
            temp_dir.path(),
            ".test/rules/ai-rules-generated-test.md",
            "test content",
        );
        assert_file_exists(temp_dir.path(), ".test/rules/ai-rules-generated-test.md");
        let result = generator.clean(temp_dir.path());
        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".test/rules");

        // Test check
        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();
        assert!(!result); // Should be false since we cleaned the files
    }

    #[test]
    fn test_generate_markdown_agent_contents_with_optional_only() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file(
                "optional1",
                "Optional rule 1",
                false,
                vec!["**/*.ts".to_string()],
                "Optional 1 body",
            ),
            create_test_source_file(
                "optional2",
                "Optional rule 2",
                false,
                vec!["**/*.js".to_string()],
                "Optional 2 body",
            ),
        ];

        let result = generate_markdown_agent_contents(
            &source_files,
            temp_dir.path(),
            ".test",
            Some(DEFAULT_RULES_SUBDIR),
        );

        assert_eq!(result.len(), 1); // Only 1 optional.md file

        // No individual files should be created for optional rules
        let expected_path1 = temp_dir
            .path()
            .join(".test/rules/ai-rules-generated-optional1.md");
        let expected_path2 = temp_dir
            .path()
            .join(".test/rules/ai-rules-generated-optional2.md");
        assert!(!result.contains_key(&expected_path1));
        assert!(!result.contains_key(&expected_path2));

        let expected_optional_path = temp_dir
            .path()
            .join(".test/rules/ai-rules-generated-optional.md");
        let optional_content = result.get(&expected_optional_path).unwrap();

        // Check template header content is included by extracting from the template
        use crate::constants::OPTIONAL_RULES_TEMPLATE;
        let header = OPTIONAL_RULES_TEMPLATE
            .split("{{RULE_ENTRIES}}")
            .next()
            .unwrap()
            .trim();
        assert!(optional_content.contains(header));
        assert!(optional_content.contains(
            "Optional rule 1: ai-rules/.generated-ai-rules/ai-rules-generated-optional1.md"
        ));
        assert!(optional_content.contains(
            "Optional rule 2: ai-rules/.generated-ai-rules/ai-rules-generated-optional2.md"
        ));
    }

    #[test]
    fn test_generate_markdown_agent_contents_always_apply_only() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file(
                "always1",
                "Always rule 1",
                true,
                vec!["**/*.ts".to_string()],
                "Always 1 body",
            ),
            create_test_source_file(
                "always2",
                "Always rule 2",
                true,
                vec!["**/*.js".to_string()],
                "Always 2 body",
            ),
        ];

        let result = generate_markdown_agent_contents(
            &source_files,
            temp_dir.path(),
            ".test",
            Some(DEFAULT_RULES_SUBDIR),
        );

        assert_eq!(result.len(), 2); // Only 2 individual files, no optional.md

        let expected_optional_path = temp_dir
            .path()
            .join(".test/rules/ai-rules-generated-optional.md");
        assert!(!result.contains_key(&expected_optional_path));
    }

    #[test]
    fn test_cline_configuration_no_rules_subdir() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        // Test cline configuration (no rules subdirectory)
        let result =
            generate_markdown_agent_contents(&[source_file], temp_dir.path(), ".clinerules", None);

        assert_eq!(result.len(), 1);
        let expected_path = temp_dir
            .path()
            .join(".clinerules")
            .join("ai-rules-generated-test.md");
        assert_eq!(
            result.get(&expected_path).unwrap(),
            "This is the rule body.\n"
        );

        // Test gitignore patterns for cline
        let patterns = markdown_agent_gitignore_patterns(".clinerules", None);
        assert_eq!(patterns, vec![".clinerules/"]);
    }

    #[test]
    fn test_markdown_generator_check_symlink_no_symlink() {
        let generator = MarkdownBasedGenerator {
            name: "test",
            agent_dir: ".test",
            rules_subdir: Some("rules"),
        };
        let temp_dir = TempDir::new().unwrap();

        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_markdown_generator_check_symlink_with_correct_symlink() {
        let generator = MarkdownBasedGenerator {
            name: "test",
            agent_dir: ".test",
            rules_subdir: Some("rules"),
        };
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let symlink_result = generator.generate_symlink(temp_dir.path());
        assert!(symlink_result.is_ok());
        let created_symlinks = symlink_result.unwrap();
        assert!(
            !created_symlinks.is_empty(),
            "generate_symlink should return non-empty Vec when successful"
        );

        // Verify the symlink file exists
        let expected_symlink_path = temp_dir.path().join(".test/rules/AGENTS.md");
        assert!(
            expected_symlink_path.exists(),
            "Symlink file should exist at expected path"
        );
        assert!(
            expected_symlink_path.is_symlink(),
            "File should be a symlink"
        );

        // Check symlink
        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(result);
    }
}

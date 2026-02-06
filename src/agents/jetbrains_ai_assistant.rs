use crate::agents::rule_generator::AgentRuleGenerator;
use crate::constants::{AGENTS_MD_FILENAME, GENERATED_FILE_PREFIX, MD_EXTENSION};
use crate::models::SourceFile;
use crate::utils::file_utils::{
    check_agents_md_symlink, check_directory_exact_match, create_symlink_to_agents_md,
    ensure_trailing_newline,
};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct JetbrainsAiAssistantGenerator;

fn get_rules_dir(current_dir: &Path) -> PathBuf {
    current_dir.join(".aiassistant").join("rules")
}

impl AgentRuleGenerator for JetbrainsAiAssistantGenerator {
    fn name(&self) -> &str {
        "jetbrains-ai-assistant"
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        let rules_dir = get_rules_dir(current_dir);
        if rules_dir.exists() {
            fs::remove_dir_all(rules_dir)?;
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

        let rules_dir = get_rules_dir(current_dir);

        for source_file in source_files {
            let generated_file_name = format!(
                "{}{}.{}",
                GENERATED_FILE_PREFIX, source_file.base_file_name, MD_EXTENSION
            );

            let file_path = rules_dir.join(generated_file_name);
            let content = ensure_trailing_newline(source_file.body.clone());
            agent_files.insert(file_path, content);
        }

        agent_files
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        let rules_dir = get_rules_dir(current_dir);

        if source_files.is_empty() {
            return Ok(!rules_dir.exists());
        }

        let expected_files = self.generate_agent_contents(source_files, current_dir);

        check_directory_exact_match(&rules_dir, &expected_files)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let agents_md_path = current_dir.join(AGENTS_MD_FILENAME);
        check_agents_md_symlink(current_dir, &agents_md_path)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        vec![".aiassistant/rules/".to_string()]
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_agents_md(current_dir, Path::new(AGENTS_MD_FILENAME))?;
        if success {
            Ok(vec![current_dir.join(AGENTS_MD_FILENAME)])
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_name() {
        let generator = JetbrainsAiAssistantGenerator;
        assert_eq!(generator.name(), "jetbrains-ai-assistant");
    }

    #[test]
    fn test_gitignore_patterns() {
        let generator = JetbrainsAiAssistantGenerator;
        let patterns = generator.gitignore_patterns();
        assert_eq!(patterns, vec![".aiassistant/rules/"]);
    }

    #[test]
    fn test_generate_agent_contents_empty() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();

        let result = generator.generate_agent_contents(&[], temp_dir.path());

        assert!(result.is_empty());
    }

    #[test]
    fn test_generate_agent_contents() {
        let generator = JetbrainsAiAssistantGenerator;
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
            .join(".aiassistant/rules/ai-rules-generated-rule1.md");
        let expected_path2 = temp_dir
            .path()
            .join(".aiassistant/rules/ai-rules-generated-rule2.md");

        // No frontmatter - just the body content
        let content1 = result.get(&expected_path1).unwrap();
        assert_eq!(content1, "rule1 body\n");

        let content2 = result.get(&expected_path2).unwrap();
        assert_eq!(content2, "rule2 body\n");
    }

    #[test]
    fn test_clean_non_existing_directory() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();

        let result = generator.clean(temp_dir.path());

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".aiassistant/rules");
    }

    #[test]
    fn test_clean_existing_directory() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            ".aiassistant/rules/ai-rules-generated-test.md",
            "test content",
        );
        create_file(
            temp_dir.path(),
            ".aiassistant/rules/ai-rules-generated-other.md",
            "other content",
        );
        assert_file_exists(
            temp_dir.path(),
            ".aiassistant/rules/ai-rules-generated-test.md",
        );

        let result = generator.clean(temp_dir.path());

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), ".aiassistant/rules");
    }

    #[test]
    fn test_clean_removes_agents_md_symlink() {
        let generator = JetbrainsAiAssistantGenerator;
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
    fn test_check_empty_source_files_no_directory() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();

        let result = generator
            .check_agent_contents(&[], temp_dir.path())
            .unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_empty_source_files_with_directory() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            ".aiassistant/rules/ai-rules-generated-stale.md",
            "stale content",
        );

        let result = generator
            .check_agent_contents(&[], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_with_matching_files() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        create_file(
            temp_dir.path(),
            ".aiassistant/rules/ai-rules-generated-test.md",
            "test body\n",
        );

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_with_missing_files() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_with_incorrect_content() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_standard_test_source_file();

        create_file(
            temp_dir.path(),
            ".aiassistant/rules/ai-rules-generated-test.md",
            "wrong content",
        );

        let result = generator
            .check_agent_contents(&[source_file], temp_dir.path())
            .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_symlink_with_correct_symlink() {
        let generator = JetbrainsAiAssistantGenerator;
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let result = generator.generate_symlink(temp_dir.path());
        assert!(result.is_ok());

        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(result);
    }
}

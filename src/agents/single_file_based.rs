use crate::agents::rule_generator::AgentRuleGenerator;
use crate::models::SourceFile;
use crate::operations::generate_all_rule_references;
use crate::utils::file_utils::{
    check_agents_md_symlink, check_inlined_file_symlink, create_symlink_to_agents_md,
    create_symlink_to_inlined_file, extract_section_content, remove_section_from_content,
    uses_section_merging,
};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SingleFileBasedGenerator {
    name: String,
    output_filename: String,
}

impl SingleFileBasedGenerator {
    pub fn new(name: &str, output_filename: &str) -> Self {
        Self {
            name: name.to_string(),
            output_filename: output_filename.to_string(),
        }
    }
}

impl AgentRuleGenerator for SingleFileBasedGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn clean(&self, current_dir: &Path) -> Result<()> {
        clean_generated_files(current_dir, &self.output_filename)
    }

    fn generate_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> HashMap<PathBuf, String> {
        generate_agent_file_contents(source_files, current_dir, &self.output_filename)
    }

    fn check_agent_contents(
        &self,
        source_files: &[SourceFile],
        current_dir: &Path,
    ) -> Result<bool> {
        check_in_sync(source_files, current_dir, &self.output_filename)
    }

    fn check_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(&self.output_filename);
        check_agents_md_symlink(current_dir, &output_file)
    }

    fn gitignore_patterns(&self) -> Vec<String> {
        vec![self.output_filename.clone()]
    }

    fn generate_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success = create_symlink_to_agents_md(current_dir, Path::new(&self.output_filename))?;
        if success {
            Ok(vec![current_dir.join(&self.output_filename)])
        } else {
            Ok(vec![])
        }
    }

    fn uses_inlined_symlink(&self) -> bool {
        true
    }

    fn generate_inlined_symlink(&self, current_dir: &Path) -> Result<Vec<PathBuf>> {
        let success =
            create_symlink_to_inlined_file(current_dir, Path::new(&self.output_filename))?;
        if success {
            Ok(vec![current_dir.join(&self.output_filename)])
        } else {
            Ok(vec![])
        }
    }

    fn check_inlined_symlink(&self, current_dir: &Path) -> Result<bool> {
        let output_file = current_dir.join(&self.output_filename);
        check_inlined_file_symlink(current_dir, &output_file)
    }
}

pub fn clean_generated_files(current_dir: &Path, output_filename: &str) -> Result<()> {
    let output_file = current_dir.join(output_filename);

    if output_file.is_symlink() {
        fs::remove_file(&output_file)?;
    } else if uses_section_merging(&output_file) {
        if output_file.exists() {
            let content = fs::read_to_string(&output_file)?;
            let cleaned = remove_section_from_content(&content);
            if cleaned.trim().is_empty() {
                fs::remove_file(&output_file)?;
            } else {
                fs::write(&output_file, cleaned)?;
            }
        }
    } else if output_file.exists() {
        fs::remove_file(&output_file)?;
    }

    Ok(())
}

pub fn generate_agent_file_contents(
    source_files: &[SourceFile],
    current_dir: &Path,
    output_filename: &str,
) -> HashMap<PathBuf, String> {
    let mut agent_files = HashMap::new();

    if !source_files.is_empty() {
        let content = generate_all_rule_references(source_files);
        let output_file_path = current_dir.join(output_filename);
        agent_files.insert(output_file_path, content);
    }

    agent_files
}

pub fn check_in_sync(
    source_files: &[SourceFile],
    current_dir: &Path,
    output_filename: &str,
) -> Result<bool> {
    let file_path = current_dir.join(output_filename);

    if source_files.is_empty() {
        if uses_section_merging(&file_path) {
            if file_path.exists() {
                let content = fs::read_to_string(&file_path)?;
                return Ok(extract_section_content(&content).is_none());
            }
            return Ok(true);
        }
        return Ok(!file_path.exists());
    }

    if !file_path.exists() {
        return Ok(false);
    }

    let expected_files = generate_agent_file_contents(source_files, current_dir, output_filename);
    let empty_string = String::new();
    let expected_content = expected_files.get(&file_path).unwrap_or(&empty_string);

    let actual_content = fs::read_to_string(&file_path)?;
    if uses_section_merging(&file_path) {
        let section = extract_section_content(&actual_content);
        Ok(section.as_deref() == Some(expected_content.as_str()))
    } else {
        Ok(actual_content == *expected_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_clean_generated_files_non_existing() {
        let temp_dir = TempDir::new().unwrap();

        let result = clean_generated_files(temp_dir.path(), "CLAUDE.md");

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), "CLAUDE.md");
    }

    #[test]
    fn test_clean_generated_files_existing() {
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            "CLAUDE.md",
            "<!-- ai-rules generated start -->\nexisting content\n<!-- ai-rules generated end -->\n",
        );
        assert_file_exists(temp_dir.path(), "CLAUDE.md");

        let result = clean_generated_files(temp_dir.path(), "CLAUDE.md");

        assert!(result.is_ok());
        assert_file_not_exists(temp_dir.path(), "CLAUDE.md");
    }

    #[test]
    fn test_clean_generated_files_preserves_user_content() {
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            "CLAUDE.md",
            "# User Content\n\n<!-- ai-rules generated start -->\nai rules section\n<!-- ai-rules generated end -->\n",
        );

        let result = clean_generated_files(temp_dir.path(), "CLAUDE.md");

        assert!(result.is_ok());
        assert_file_exists(temp_dir.path(), "CLAUDE.md");
        let content = std::fs::read_to_string(temp_dir.path().join("CLAUDE.md")).unwrap();
        assert!(content.contains("# User Content"));
        assert!(!content.contains("<!-- ai-rules generated start -->"));
    }

    #[test]
    fn test_generate_agent_file_contents_empty() {
        let temp_dir = TempDir::new().unwrap();

        let result = generate_agent_file_contents(&[], temp_dir.path(), "CLAUDE.md");

        assert!(result.is_empty());
    }

    #[test]
    fn test_generate_agent_file_contents_always_apply_only() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file(
                "rule1",
                "Always apply rule",
                true,
                vec!["**/*.ts".to_string()],
                "rule1 body",
            ),
            create_test_source_file(
                "rule2",
                "Another always apply",
                true,
                vec!["**/*.js".to_string()],
                "rule2 body",
            ),
        ];

        let result = generate_agent_file_contents(&source_files, temp_dir.path(), "CLAUDE.md");

        assert_eq!(result.len(), 1);
        let expected_path = temp_dir.path().join("CLAUDE.md");
        let expected_content =
            "@ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md\n@ai-rules/.generated-ai-rules/ai-rules-generated-rule2.md\n";

        assert_eq!(
            result.get(&expected_path),
            Some(&expected_content.to_string())
        );
    }

    #[test]
    fn test_generate_agent_file_contents_optional_only() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file(
                "rule1",
                "Optional rule",
                false,
                vec!["**/*.ts".to_string()],
                "rule1 body",
            ),
            create_test_source_file(
                "rule2",
                "Another optional",
                false,
                vec!["**/*.js".to_string()],
                "rule2 body",
            ),
        ];

        let result = generate_agent_file_contents(&source_files, temp_dir.path(), "CLAUDE.md");

        assert_eq!(result.len(), 1);
        let expected_path = temp_dir.path().join("CLAUDE.md");
        let expected_content = "\n@ai-rules/.generated-ai-rules/ai-rules-generated-optional.md\n";

        assert_eq!(
            result.get(&expected_path),
            Some(&expected_content.to_string())
        );
    }

    #[test]
    fn test_generate_agent_file_contents_mixed() {
        let temp_dir = TempDir::new().unwrap();
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
            create_test_source_file(
                "always2",
                "Another always",
                true,
                vec!["**/*.rs".to_string()],
                "always2 body",
            ),
        ];

        let result = generate_agent_file_contents(&source_files, temp_dir.path(), "CLAUDE.md");

        assert_eq!(result.len(), 1);
        let expected_path = temp_dir.path().join("CLAUDE.md");
        let expected_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-always1.md\n@ai-rules/.generated-ai-rules/ai-rules-generated-always2.md\n\n@ai-rules/.generated-ai-rules/ai-rules-generated-optional.md\n";

        assert_eq!(
            result.get(&expected_path),
            Some(&expected_content.to_string())
        );
    }

    #[test]
    fn test_check_in_sync_empty_source_files_no_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = check_in_sync(&[], temp_dir.path(), "CLAUDE.md").unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_in_sync_empty_source_files_with_file_no_section() {
        let temp_dir = TempDir::new().unwrap();

        // File with only user content (no ai-rules section) is considered in sync
        create_file(temp_dir.path(), "CLAUDE.md", "user content");

        let result = check_in_sync(&[], temp_dir.path(), "CLAUDE.md").unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_in_sync_empty_source_files_with_stale_section() {
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            "CLAUDE.md",
            "<!-- ai-rules generated start -->\nstale content\n<!-- ai-rules generated end -->\n",
        );

        let result = check_in_sync(&[], temp_dir.path(), "CLAUDE.md").unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_in_sync_with_source_files_no_output() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        let result = check_in_sync(&source_files, temp_dir.path(), "CLAUDE.md").unwrap();

        assert!(!result)
    }

    #[test]
    fn test_check_in_sync_mismatched_content() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "rule1",
            "Test rule",
            true,
            vec!["**/*.ts".to_string()],
            "rule1 body",
        )];

        create_file(temp_dir.path(), "CLAUDE.md", "wrong content");

        let result = check_in_sync(&source_files, temp_dir.path(), "CLAUDE.md").unwrap();

        assert!(!result);
    }

    #[test]
    fn test_check_in_sync_match() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file(
                "always1",
                "Always rule",
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

        let expected_content = "@ai-rules/.generated-ai-rules/ai-rules-generated-always1.md\n\n@ai-rules/.generated-ai-rules/ai-rules-generated-optional.md\n";
        let section = format!("<!-- ai-rules generated start -->\n{expected_content}<!-- ai-rules generated end -->\n");
        create_file(temp_dir.path(), "CLAUDE.md", &section);

        let result = check_in_sync(&source_files, temp_dir.path(), "CLAUDE.md").unwrap();

        assert!(result);
    }

    #[test]
    fn test_check_in_sync_match_with_surrounding_user_content() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "always1",
            "Always rule",
            true,
            vec!["**/*.ts".to_string()],
            "always1 body",
        )];

        let expected_content =
            "@ai-rules/.generated-ai-rules/ai-rules-generated-always1.md\n";
        let file_content = format!(
            "# My Rules\n\n<!-- ai-rules generated start -->\n{expected_content}<!-- ai-rules generated end -->\n"
        );
        create_file(temp_dir.path(), "CLAUDE.md", &file_content);

        let result = check_in_sync(&source_files, temp_dir.path(), "CLAUDE.md").unwrap();

        assert!(result);
    }

    #[test]
    fn test_single_file_generator_check_symlink_with_correct_symlink() {
        let generator = SingleFileBasedGenerator::new("test", "CLAUDE.md");
        let temp_dir = TempDir::new().unwrap();

        create_file(temp_dir.path(), "ai-rules/AGENTS.md", "# Source content");

        let result = generator.generate_symlink(temp_dir.path());
        assert!(result.is_ok());

        let result = generator.check_symlink(temp_dir.path()).unwrap();
        assert!(result);
    }
}

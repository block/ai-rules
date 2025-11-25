use crate::constants::{AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR, MD_EXTENSION};
use crate::models::SourceFile;
use crate::utils::file_utils::find_files_by_extension;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_ai_rules_dir(current_dir: &Path) -> PathBuf {
    current_dir.join(AI_RULE_SOURCE_DIR)
}

fn get_md_files_in_ai_rules_dir(current_dir: &Path) -> Result<Vec<PathBuf>> {
    let ai_rules_dir = get_ai_rules_dir(current_dir);

    if !ai_rules_dir.exists() || !ai_rules_dir.is_dir() {
        return Ok(Vec::new());
    }

    find_files_by_extension(&ai_rules_dir, MD_EXTENSION)
}

pub fn find_source_files(current_dir: &Path) -> Result<Vec<SourceFile>> {
    let source_files = get_md_files_in_ai_rules_dir(current_dir)?;
    if source_files.is_empty() {
        return Ok(Vec::new());
    }

    parse_source_files(source_files)
}

fn parse_source_files(original_source_files: Vec<PathBuf>) -> Result<Vec<SourceFile>> {
    let mut source_files = Vec::new();
    for original_source_file in original_source_files {
        let source_file = SourceFile::from_file(&original_source_file)?;
        source_files.push(source_file);
    }
    Ok(source_files)
}

pub fn detect_symlink_mode(current_dir: &Path) -> bool {
    let md_files = match get_md_files_in_ai_rules_dir(current_dir) {
        Ok(files) => files,
        Err(_) => return false,
    };

    if md_files.len() != 1 {
        return false;
    }

    let agents_file = &md_files[0];
    let filename = agents_file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if filename != AGENTS_MD_FILENAME {
        return false;
    }
    is_pure_markdown(agents_file)
}

fn is_pure_markdown(file_path: &Path) -> bool {
    if let Ok(content) = fs::read_to_string(file_path) {
        let trimmed = content.trim_start();
        // Pure markdown doesn't start with YAML frontmatter
        !trimmed.starts_with("---")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_source_files_no_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let result = find_source_files(temp_path).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_find_source_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let ai_rules_dir = temp_path.join("ai-rules");

        fs::create_dir(&ai_rules_dir).unwrap();

        let result = find_source_files(temp_path).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_find_source_files_with_md_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        let ai_rules_dir = temp_path.join("ai-rules");

        fs::create_dir(&ai_rules_dir).unwrap();

        let source_file_content = r#"---
description: Test rule
alwaysApply: true
fileMatching: "**/*.rs"
---

# Test Rule
This is a test rule."#;

        fs::write(ai_rules_dir.join("test1.md"), source_file_content).unwrap();
        fs::write(ai_rules_dir.join("test2.md"), source_file_content).unwrap();
        fs::write(ai_rules_dir.join("readme.txt"), "not an md file").unwrap();

        let result = find_source_files(temp_path).unwrap();

        assert_eq!(result.len(), 2);

        // Sort by base_file_name to ensure consistent ordering
        let mut sorted_result = result;
        sorted_result.sort_by(|a, b| a.base_file_name.cmp(&b.base_file_name));

        assert_eq!(sorted_result[0].base_file_name, "test1");
        assert_eq!(sorted_result[1].base_file_name, "test2");
        assert_eq!(sorted_result[0].front_matter.description, "Test rule");
        assert!(sorted_result[0].front_matter.always_apply);
        assert_eq!(
            sorted_result[0].front_matter.file_matching_patterns,
            Some(vec!["**/*.rs".to_string()])
        );
        assert_eq!(sorted_result[0].body, "# Test Rule\nThis is a test rule.");
    }

    #[test]
    fn test_parse_source_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let source_file_content1 = r#"---
description: First rule
alwaysApply: false
fileMatching: "**/*.js"
---

# First Rule
Content for first rule."#;

        let source_file_content2 = r#"---
description: Second rule
alwaysApply: true
fileMatching: "**/*.ts, **/*.tsx"
---

# Second Rule
Content for second rule."#;

        let file1 = temp_path.join("rule1.md");
        let file2 = temp_path.join("rule2.md");

        fs::write(&file1, source_file_content1).unwrap();
        fs::write(&file2, source_file_content2).unwrap();

        let source_files = vec![file1, file2];
        let result = parse_source_files(source_files).unwrap();

        assert_eq!(result.len(), 2);

        assert_eq!(result[0].base_file_name, "rule1");
        assert_eq!(result[0].front_matter.description, "First rule");
        assert!(!result[0].front_matter.always_apply);
        assert_eq!(
            result[0].front_matter.file_matching_patterns,
            Some(vec!["**/*.js".to_string()])
        );
        assert_eq!(result[0].body, "# First Rule\nContent for first rule.");

        assert_eq!(result[1].base_file_name, "rule2");
        assert_eq!(result[1].front_matter.description, "Second rule");
        assert!(result[1].front_matter.always_apply);
        assert_eq!(
            result[1].front_matter.file_matching_patterns,
            Some(vec!["**/*.ts".to_string(), "**/*.tsx".to_string()])
        );
        assert_eq!(result[1].body, "# Second Rule\nContent for second rule.");
    }

    #[test]
    fn test_parse_source_files_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let invalid_content = r#"---
- description: Second rule
---
# Second Rule
Content for second rule."#;
        let file1 = temp_path.join("invalid.md");

        fs::write(&file1, invalid_content).unwrap();

        let source_files = vec![file1];
        let result = parse_source_files(source_files);

        assert!(result.is_err());
    }

    #[test]
    fn test_detect_symlink_mode_no_ai_rules_dir() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let result = detect_symlink_mode(temp_path);
        assert!(!result);
    }

    #[test]
    fn test_detect_symlink_mode_empty_ai_rules_dir() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(AI_RULE_SOURCE_DIR)).unwrap();

        let result = detect_symlink_mode(temp_path);
        assert!(!result);
    }

    #[test]
    fn test_detect_symlink_mode_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(AI_RULE_SOURCE_DIR)).unwrap();
        fs::write(
            temp_path.join(AI_RULE_SOURCE_DIR).join(AGENTS_MD_FILENAME),
            "# Pure markdown",
        )
        .unwrap();
        fs::write(
            temp_path.join(AI_RULE_SOURCE_DIR).join("other.md"),
            "# Another file",
        )
        .unwrap();

        let result = detect_symlink_mode(temp_path);
        assert!(!result);
    }

    #[test]
    fn test_detect_symlink_mode_wrong_filename() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(AI_RULE_SOURCE_DIR)).unwrap();
        fs::write(
            temp_path.join(AI_RULE_SOURCE_DIR).join("RULES.md"),
            "# Not agents.md",
        )
        .unwrap();

        let result = detect_symlink_mode(temp_path);
        assert!(!result);
    }

    #[test]
    fn test_detect_symlink_mode_agents_md_with_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(AI_RULE_SOURCE_DIR)).unwrap();
        let content_with_frontmatter = r#"---
description: Test rule
alwaysApply: true
---
# This has frontmatter"#;
        fs::write(
            temp_path.join(AI_RULE_SOURCE_DIR).join(AGENTS_MD_FILENAME),
            content_with_frontmatter,
        )
        .unwrap();

        let result = detect_symlink_mode(temp_path);
        assert!(!result);
    }

    #[test]
    fn test_detect_symlink_mode_pure_agents_md() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(AI_RULE_SOURCE_DIR)).unwrap();
        let pure_markdown =
            "# Pure markdown content\n\nThis is just regular markdown without frontmatter.";
        fs::write(
            temp_path.join(AI_RULE_SOURCE_DIR).join(AGENTS_MD_FILENAME),
            pure_markdown,
        )
        .unwrap();

        let result = detect_symlink_mode(temp_path);
        assert!(result);
    }
}

use crate::constants::{CLAUDE_SKILLS_DIR, GENERATED_FILE_PREFIX, SKILL_FILENAME};
use crate::models::source_file::SourceFile;
use crate::operations::body_generator::generated_body_file_reference_path;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Sanitizes a skill name to meet Claude's requirements:
/// - Must use lowercase letters, numbers, and hyphens only
/// - Max 64 characters
///   https://docs.claude.com/en/docs/claude-code/skills#write-skill-md
fn sanitize_skill_name(name: &str) -> String {
    // Convert to lowercase and replace non-alphanumeric characters with hyphens
    let lowercase = name.to_lowercase();
    let re = Regex::new(r"[^a-z0-9-]+").unwrap();
    let with_hyphens = re.replace_all(&lowercase, "-");

    // Collapse multiple consecutive hyphens into one
    let re_multiple = Regex::new(r"-+").unwrap();
    let collapsed = re_multiple.replace_all(&with_hyphens, "-");

    // Remove leading/trailing hyphens and truncate to 64 characters
    let trimmed = collapsed.trim_matches('-');
    let truncated = if trimmed.len() > 64 {
        &trimmed[..64]
    } else {
        trimmed
    };

    truncated.trim_end_matches('-').to_string()
}

pub fn remove_generated_skills(project_root: &Path) -> anyhow::Result<()> {
    let skills_dir = project_root.join(CLAUDE_SKILLS_DIR);
    if !skills_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(&skills_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                if folder_name.starts_with(GENERATED_FILE_PREFIX) {
                    std::fs::remove_dir_all(&path)?;
                }
            }
        }
    }

    Ok(())
}

pub fn generate_skills_for_optional_rules(
    source_files: &[SourceFile],
    project_root: &Path,
) -> anyhow::Result<HashMap<PathBuf, String>> {
    let mut skill_files = HashMap::new();

    let optional_rules: Vec<&SourceFile> = source_files
        .iter()
        .filter(|f| !f.front_matter.always_apply)
        .collect();

    for rule in optional_rules {
        let (path, content) = generate_skill_file_content(rule, project_root)?;
        skill_files.insert(path, content);
    }

    Ok(skill_files)
}

fn generate_skill_file_content(
    rule: &SourceFile,
    project_root: &Path,
) -> anyhow::Result<(PathBuf, String)> {
    let skill_folder_name = format!("{}{}", GENERATED_FILE_PREFIX, rule.base_file_name);
    let skill_file_path = project_root
        .join(CLAUDE_SKILLS_DIR)
        .join(&skill_folder_name)
        .join(SKILL_FILENAME);

    let description = if rule.front_matter.description.is_empty() {
        &rule.base_file_name
    } else {
        &rule.front_matter.description
    };

    let skill_name = sanitize_skill_name(description);

    let body_file_name = rule.get_body_file_name();
    let generated_path = generated_body_file_reference_path(&body_file_name);

    let skill_content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n@{}",
        skill_name,
        description,
        generated_path.display()
    );

    Ok((skill_file_path, skill_content))
}

pub fn check_skills_in_sync(
    source_files: &[SourceFile],
    project_root: &Path,
) -> anyhow::Result<bool> {
    let expected_skills = generate_skills_for_optional_rules(source_files, project_root)?;
    let skills_dir = project_root.join(CLAUDE_SKILLS_DIR);

    let actual_generated_skill_dirs: Vec<_> = if skills_dir.exists() {
        std::fs::read_dir(&skills_dir)?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.path().is_dir()
                    && entry
                        .file_name()
                        .to_str()
                        .is_some_and(|n| n.starts_with(GENERATED_FILE_PREFIX))
            })
            .collect()
    } else {
        vec![]
    };
    if expected_skills.is_empty() {
        return Ok(actual_generated_skill_dirs.is_empty());
    }
    if actual_generated_skill_dirs.is_empty() {
        return Ok(false);
    }
    if actual_generated_skill_dirs.len() != expected_skills.len() {
        return Ok(false);
    }
    for (expected_path, expected_content) in &expected_skills {
        if !expected_path.exists() {
            return Ok(false);
        }
        let actual_content = std::fs::read_to_string(expected_path)?;
        if actual_content != *expected_content {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source_file::FrontMatter;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_source_file(
        base_name: &str,
        description: &str,
        always_apply: bool,
        body: &str,
    ) -> SourceFile {
        SourceFile {
            front_matter: FrontMatter {
                description: description.to_string(),
                always_apply,
                file_matching_patterns: None,
            },
            body: body.to_string(),
            base_file_name: base_name.to_string(),
        }
    }

    fn create_skill_file(temp_dir: &Path, skill_name: &str, content: &str) -> PathBuf {
        let skill_dir = temp_dir.join(format!(".claude/skills/{skill_name}"));
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), content).unwrap();
        skill_dir
    }

    fn generate_skill_content(name: &str, description: &str, base_name: &str) -> String {
        format!(
            "---\nname: {name}\ndescription: {description}\n---\n\n@ai-rules/.generated-ai-rules/ai-rules-generated-{base_name}.md"
        )
    }

    #[test]
    fn test_sanitize_skill_name() {
        assert_eq!(sanitize_skill_name("Test Workflow"), "test-workflow");
        assert_eq!(sanitize_skill_name("My_Cool_Rule"), "my-cool-rule");
        assert_eq!(sanitize_skill_name("Rule with CAPS"), "rule-with-caps");
        assert_eq!(sanitize_skill_name("Rule123"), "rule123");
        assert_eq!(
            sanitize_skill_name("Rule   Multiple   Spaces"),
            "rule-multiple-spaces"
        );
        assert_eq!(
            sanitize_skill_name("Rule-with-hyphens"),
            "rule-with-hyphens"
        );
        assert_eq!(
            sanitize_skill_name("Rule_with_underscores"),
            "rule-with-underscores"
        );
        assert_eq!(sanitize_skill_name("Rule!@#$%Special"), "rule-special");
        assert_eq!(sanitize_skill_name("---leading-hyphens"), "leading-hyphens");
        assert_eq!(
            sanitize_skill_name("trailing-hyphens---"),
            "trailing-hyphens"
        );
        assert_eq!(
            sanitize_skill_name("multiple---hyphens"),
            "multiple-hyphens"
        );

        let long_name = "a".repeat(100);
        assert_eq!(sanitize_skill_name(&long_name).len(), 64);
    }

    #[test]
    fn test_generate_skill_file_content() {
        let temp_dir = TempDir::new().unwrap();
        let source_file =
            create_test_source_file("test-optional", "Test Workflow", false, "My workflow is 42");

        let (path, content) = generate_skill_file_content(&source_file, temp_dir.path()).unwrap();

        assert_eq!(
            path,
            temp_dir
                .path()
                .join(".claude/skills/ai-rules-generated-test-optional/SKILL.md")
        );
        assert!(content.contains("name: test-workflow"));
        assert!(content.contains("description: Test Workflow"));
        assert!(
            content.contains("@ai-rules/.generated-ai-rules/ai-rules-generated-test-optional.md")
        );
    }

    #[test]
    fn test_generate_skills_filters_optional_only() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file("optional", "Optional Rule", false, "Optional content"),
            create_test_source_file("required", "Required Rule", true, "Required content"),
        ];

        let skills = generate_skills_for_optional_rules(&source_files, temp_dir.path()).unwrap();

        assert_eq!(skills.len(), 1);
        assert!(skills.contains_key(
            &temp_dir
                .path()
                .join(".claude/skills/ai-rules-generated-optional/SKILL.md")
        ));
        assert!(!skills.contains_key(
            &temp_dir
                .path()
                .join(".claude/skills/ai-rules-generated-required/SKILL.md")
        ));
    }

    #[test]
    fn test_empty_description_uses_filename() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_test_source_file("fallback-name", "", false, "Content");

        let (path, content) = generate_skill_file_content(&source_file, temp_dir.path()).unwrap();

        assert_eq!(
            path,
            temp_dir
                .path()
                .join(".claude/skills/ai-rules-generated-fallback-name/SKILL.md")
        );
        assert!(content.contains("name: fallback-name"));
        assert!(content.contains("description: fallback-name"));
    }

    #[test]
    fn test_remove_generated_skills() {
        let temp_dir = TempDir::new().unwrap();

        let generated_skill = create_skill_file(
            temp_dir.path(),
            "ai-rules-generated-test",
            "generated content",
        );
        let user_skill = create_skill_file(temp_dir.path(), "my-custom-skill", "user content");

        remove_generated_skills(temp_dir.path()).unwrap();

        assert!(!generated_skill.exists());
        assert!(user_skill.exists());
        assert_eq!(
            std::fs::read_to_string(user_skill.join("SKILL.md")).unwrap(),
            "user content"
        );
    }

    #[test]
    fn test_check_skills_in_sync_no_optional_rules() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "required", "Required", true, "Content",
        )];

        let result = check_skills_in_sync(&source_files, temp_dir.path()).unwrap();
        assert!(result);

        create_skill_file(temp_dir.path(), "my-custom-skill", "custom");

        let result = check_skills_in_sync(&source_files, temp_dir.path()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_skills_in_sync_missing_skill_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "optional", "Optional", false, "Content",
        )];

        let result = check_skills_in_sync(&source_files, temp_dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_skills_in_sync_with_orphaned_skill() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "optional", "Optional", false, "Content",
        )];

        create_skill_file(
            temp_dir.path(),
            "ai-rules-generated-optional",
            &generate_skill_content("optional", "Optional", "optional"),
        );

        let result = check_skills_in_sync(&source_files, temp_dir.path()).unwrap();
        assert!(result);

        create_skill_file(temp_dir.path(), "my-custom-skill", "custom");

        let result = check_skills_in_sync(&source_files, temp_dir.path()).unwrap();
        assert!(result);

        create_skill_file(temp_dir.path(), "ai-rules-generated-orphaned", "orphaned");

        let result = check_skills_in_sync(&source_files, temp_dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_skills_in_sync_wrong_content() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![create_test_source_file(
            "optional", "Optional", false, "Content",
        )];

        create_skill_file(
            temp_dir.path(),
            "ai-rules-generated-optional",
            "---\nname: Wrong\ndescription: Wrong\n---\n\n@wrong.md",
        );

        let result = check_skills_in_sync(&source_files, temp_dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_skills_in_sync_different_folder_names() {
        let temp_dir = TempDir::new().unwrap();

        let source_files = vec![
            create_test_source_file("skill1", "Skill 1", false, "Content 1"),
            create_test_source_file("skill3", "Skill 3", false, "Content 3"),
        ];

        create_skill_file(
            temp_dir.path(),
            "ai-rules-generated-skill1",
            &generate_skill_content("skill-1", "Skill 1", "skill1"),
        );
        create_skill_file(
            temp_dir.path(),
            "ai-rules-generated-skill2",
            &generate_skill_content("skill-2", "Skill 2", "skill2"),
        );

        let result = check_skills_in_sync(&source_files, temp_dir.path()).unwrap();
        assert!(!result);
    }
}

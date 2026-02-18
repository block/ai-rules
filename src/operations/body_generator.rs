use crate::constants::{
    AI_RULE_SOURCE_DIR, GENERATED_RULE_BODY_DIR, INLINED_AGENTS_FILENAME, OPTIONAL_RULES_FILENAME,
};
use crate::models::SourceFile;
use crate::operations::optional_rules::generate_optional_rules_content;
use crate::utils::file_utils::ensure_trailing_newline;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn generate_body_contents(
    source_files: &[SourceFile],
    current_dir: &Path,
) -> HashMap<PathBuf, String> {
    let mut body_files = HashMap::new();

    if source_files.is_empty() {
        return body_files;
    }

    let generated_dir = generated_body_file_dir(current_dir);

    for source_file in source_files {
        let body_file_name = source_file.get_body_file_name();
        let file_path = generated_dir.join(body_file_name);
        body_files.insert(file_path, ensure_trailing_newline(source_file.body.clone()));
    }

    let optional_content = generate_optional_rules_content(source_files);
    if !optional_content.is_empty() {
        let optional_file_path = generated_dir.join(OPTIONAL_RULES_FILENAME);
        body_files.insert(optional_file_path, optional_content);
    }

    let inlined_content = generate_inlined_agents_content(source_files);
    if !inlined_content.is_empty() {
        let inlined_file_path = generated_dir.join(INLINED_AGENTS_FILENAME);
        body_files.insert(inlined_file_path, inlined_content);
    }

    body_files
}

pub fn generated_body_file_dir(current_dir: &Path) -> PathBuf {
    current_dir
        .join(AI_RULE_SOURCE_DIR)
        .join(GENERATED_RULE_BODY_DIR)
}

pub fn generated_body_file_reference_path(filename: &str) -> PathBuf {
    Path::new(AI_RULE_SOURCE_DIR)
        .join(GENERATED_RULE_BODY_DIR)
        .join(filename)
}

pub fn generate_required_rule_references(source_files: &[SourceFile]) -> String {
    let mut content = String::new();

    for source_file in source_files {
        if source_file.front_matter.always_apply {
            let body_file_name = source_file.get_body_file_name();
            let generated_path = generated_body_file_reference_path(&body_file_name);
            content.push_str(&format!("@{}\n", generated_path.display()));
        }
    }

    content
}

pub fn generate_all_rule_references(source_files: &[SourceFile]) -> String {
    let mut content = generate_required_rule_references(source_files);

    // Check if there are any optional rules and reference the optional.md file
    let has_optional_rules = source_files
        .iter()
        .any(|file| !file.front_matter.always_apply);
    if has_optional_rules {
        content.push('\n');
        let optional_path = generated_body_file_reference_path(OPTIONAL_RULES_FILENAME);
        content.push_str(&format!("@{}\n", optional_path.display()));
    }

    content
}

pub fn generate_inlined_agents_content(source_files: &[SourceFile]) -> String {
    let mut content = generate_inlined_required_content(source_files);

    let optional_content = generate_optional_rules_content(source_files);
    if !optional_content.is_empty() {
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&optional_content);
    }

    content
}

pub fn generate_inlined_required_content(source_files: &[SourceFile]) -> String {
    let mut parts: Vec<String> = Vec::new();

    for source_file in source_files {
        if source_file.front_matter.always_apply {
            let mut part = String::new();
            if !source_file.front_matter.description.is_empty() {
                part.push_str(&format!("# {}\n\n", source_file.front_matter.description));
            }
            part.push_str(&ensure_trailing_newline(source_file.body.clone()));
            parts.push(part);
        }
    }

    parts.join("\n")
}

pub fn inlined_agents_relative_path() -> PathBuf {
    Path::new(AI_RULE_SOURCE_DIR)
        .join(GENERATED_RULE_BODY_DIR)
        .join(INLINED_AGENTS_FILENAME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source_file::FrontMatter;

    fn create_test_source_file(
        base_file_name: &str,
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
            base_file_name: base_file_name.to_string(),
        }
    }

    #[test]
    fn test_generate_required_rule_references_only_always_apply() {
        let source_files = vec![
            create_test_source_file("always1", "Always", true, "Content"),
            create_test_source_file("optional1", "Optional", false, "Content"),
        ];

        let content = generate_required_rule_references(&source_files);

        assert!(content.contains("ai-rules-generated-always1.md"));
        assert!(!content.contains("ai-rules-generated-optional1.md"));
        assert!(!content.contains("ai-rules-generated-optional.md"));
    }

    #[test]
    fn test_generate_required_rule_references_empty_list() {
        let source_files: Vec<SourceFile> = vec![];

        let content = generate_required_rule_references(&source_files);

        assert_eq!(content, "");
    }

    #[test]
    fn test_generate_required_rule_references_no_required_rules() {
        let source_files = vec![
            create_test_source_file("optional1", "Optional", false, "Content"),
            create_test_source_file("optional2", "Optional", false, "Content"),
        ];

        let content = generate_required_rule_references(&source_files);

        assert_eq!(content, "");
    }

    #[test]
    fn test_generate_required_rule_references_multiple_required() {
        let source_files = vec![
            create_test_source_file("always1", "Always 1", true, "Content 1"),
            create_test_source_file("always2", "Always 2", true, "Content 2"),
            create_test_source_file("optional1", "Optional", false, "Content"),
        ];

        let content = generate_required_rule_references(&source_files);

        assert_eq!(
            content,
            "@ai-rules/.generated-ai-rules/ai-rules-generated-always1.md\n\
             @ai-rules/.generated-ai-rules/ai-rules-generated-always2.md\n"
        );
    }

    #[test]
    fn test_generate_all_rule_references_only_required() {
        let source_files = vec![
            create_test_source_file("always1", "Always", true, "Content"),
            create_test_source_file("always2", "Always 2", true, "Content 2"),
        ];

        let content = generate_all_rule_references(&source_files);

        assert_eq!(
            content,
            "@ai-rules/.generated-ai-rules/ai-rules-generated-always1.md\n\
             @ai-rules/.generated-ai-rules/ai-rules-generated-always2.md\n"
        );
    }

    #[test]
    fn test_generate_all_rule_references_only_optional() {
        let source_files = vec![
            create_test_source_file("optional1", "Optional", false, "Content"),
            create_test_source_file("optional2", "Optional 2", false, "Content 2"),
        ];

        let content = generate_all_rule_references(&source_files);

        assert_eq!(
            content,
            "\n@ai-rules/.generated-ai-rules/ai-rules-generated-optional.md\n"
        );
    }

    #[test]
    fn test_generate_all_rule_references_mixed() {
        let source_files = vec![
            create_test_source_file("always1", "Always", true, "Content"),
            create_test_source_file("optional1", "Optional", false, "Content"),
            create_test_source_file("always2", "Always 2", true, "Content 2"),
        ];

        let content = generate_all_rule_references(&source_files);

        assert_eq!(
            content,
            "@ai-rules/.generated-ai-rules/ai-rules-generated-always1.md\n\
             @ai-rules/.generated-ai-rules/ai-rules-generated-always2.md\n\
             \n\
             @ai-rules/.generated-ai-rules/ai-rules-generated-optional.md\n"
        );
    }

    #[test]
    fn test_generate_all_rule_references_empty() {
        let source_files: Vec<SourceFile> = vec![];

        let content = generate_all_rule_references(&source_files);

        assert_eq!(content, "");
    }
}

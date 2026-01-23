use crate::constants::{
    AGENTS_MD_AGENTS, AGENTS_MD_GROUP_NAME, AI_RULE_SOURCE_DIR, GENERATED_RULE_BODY_DIR,
};
use crate::models::SourceFile;
use crate::operations::optional_rules::{
    generate_optional_rules_content, optional_rules_filename_for_agent,
};
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

pub fn generate_all_rule_references_for_agent(
    source_files: &[SourceFile],
    agent_name: &str,
) -> String {
    let mut content = generate_required_rule_references(source_files);

    // Check if there are any optional rules and reference the optional.md file
    let has_optional_rules = source_files
        .iter()
        .any(|file| !file.front_matter.always_apply);
    if has_optional_rules {
        content.push('\n');
        let optional_filename = optional_rules_filename_for_agent(agent_name);
        let optional_path = generated_body_file_reference_path(&optional_filename);
        content.push_str(&format!("@{}\n", optional_path.display()));
    }

    content
}

pub fn generate_optional_rule_files_for_agents(
    source_files: &[SourceFile],
    current_dir: &Path,
    agents: &[String],
) -> HashMap<PathBuf, String> {
    let mut optional_files = HashMap::new();

    if source_files.is_empty() {
        return optional_files;
    }

    let generated_dir = generated_body_file_dir(current_dir);

    if agents
        .iter()
        .any(|agent| AGENTS_MD_AGENTS.iter().any(|name| name == &agent.as_str()))
    {
        let filtered_source_files = crate::models::source_file::filter_source_files_for_agent_group(
            source_files,
            &AGENTS_MD_AGENTS,
        );
        let optional_content = generate_optional_rules_content(&filtered_source_files);
        if !optional_content.is_empty() {
            let optional_filename = optional_rules_filename_for_agent(AGENTS_MD_GROUP_NAME);
            optional_files.insert(generated_dir.join(optional_filename), optional_content);
        }
    }

    for agent in agents {
        if AGENTS_MD_AGENTS.iter().any(|name| name == &agent.as_str()) {
            continue;
        }
        let filtered_source_files = crate::models::source_file::filter_source_files_for_agent(
            source_files,
            agent,
        );
        let optional_content = generate_optional_rules_content(&filtered_source_files);
        if optional_content.is_empty() {
            continue;
        }
        let optional_filename = optional_rules_filename_for_agent(agent);
        optional_files.insert(generated_dir.join(optional_filename), optional_content);
    }

    optional_files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source_file::FrontMatter;
    use tempfile::TempDir;

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
                allowed_agents: None,
                blocked_agents: None,
            },
            body: body.to_string(),
            base_file_name: base_file_name.to_string(),
        }
    }

    fn create_test_source_file_with_agents(
        base_file_name: &str,
        description: &str,
        always_apply: bool,
        allowed_agents: Option<Vec<&str>>,
        blocked_agents: Option<Vec<&str>>,
        body: &str,
    ) -> SourceFile {
        SourceFile {
            front_matter: FrontMatter {
                description: description.to_string(),
                always_apply,
                file_matching_patterns: None,
                allowed_agents: allowed_agents.map(|agents| {
                    agents.into_iter().map(|agent| agent.to_string()).collect()
                }),
                blocked_agents: blocked_agents.map(|agents| {
                    agents.into_iter().map(|agent| agent.to_string()).collect()
                }),
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
        assert!(!content.contains("ai-rules-generated-optional-claude.md"));
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

        let content = generate_all_rule_references_for_agent(&source_files, "claude");

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

        let content = generate_all_rule_references_for_agent(&source_files, "claude");

        assert_eq!(
            content,
            "\n@ai-rules/.generated-ai-rules/ai-rules-generated-optional-claude.md\n"
        );
    }

    #[test]
    fn test_generate_all_rule_references_mixed() {
        let source_files = vec![
            create_test_source_file("always1", "Always", true, "Content"),
            create_test_source_file("optional1", "Optional", false, "Content"),
            create_test_source_file("always2", "Always 2", true, "Content 2"),
        ];

        let content = generate_all_rule_references_for_agent(&source_files, "claude");

        assert_eq!(
            content,
            "@ai-rules/.generated-ai-rules/ai-rules-generated-always1.md\n\
             @ai-rules/.generated-ai-rules/ai-rules-generated-always2.md\n\
             \n\
             @ai-rules/.generated-ai-rules/ai-rules-generated-optional-claude.md\n"
        );
    }

    #[test]
    fn test_generate_all_rule_references_empty() {
        let source_files: Vec<SourceFile> = vec![];

        let content = generate_all_rule_references_for_agent(&source_files, "claude");

        assert_eq!(content, "");
    }

    #[test]
    fn test_generate_optional_rule_files_for_agents_filters() {
        let temp_dir = TempDir::new().unwrap();
        let source_files = vec![
            create_test_source_file_with_agents(
                "claude_only",
                "Claude only",
                false,
                Some(vec!["claude"]),
                None,
                "Optional",
            ),
            create_test_source_file_with_agents(
                "not_goose",
                "Everyone but goose",
                false,
                None,
                Some(vec!["goose"]),
                "Optional",
            ),
        ];
        let agents = vec!["claude".to_string(), "goose".to_string()];

        let files =
            generate_optional_rule_files_for_agents(&source_files, temp_dir.path(), &agents);
        let claude_optional_path = temp_dir
            .path()
            .join("ai-rules/.generated-ai-rules/ai-rules-generated-optional-claude.md");

        assert!(files.contains_key(&claude_optional_path));
        assert!(!files.keys().any(|path| {
            path.to_string_lossy()
                .contains("ai-rules-generated-optional-goose.md")
        }));
        assert!(!files.keys().any(|path| {
            path.to_string_lossy()
                .contains("ai-rules-generated-optional-agents-md.md")
        }));
        let content = files.get(&claude_optional_path).unwrap();
        assert!(content.contains("Claude only"));
        assert!(content.contains("Everyone but goose"));
    }
}

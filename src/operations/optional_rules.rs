use crate::constants::OPTIONAL_RULES_TEMPLATE;
use crate::models::SourceFile;
use crate::operations::body_generator::generated_body_file_reference_path;

/// Generates the optional rules content string for source files where `always_apply == false`.
///
/// This function filters the provided source files to find those marked as optional
/// (where `always_apply` is false) and formats them into a standardized optional rules
/// section.
///
/// # Arguments
///
/// * `source_files` - A slice of SourceFile objects to process
///
/// # Returns
///
/// Returns an empty string if there are no optional rules, otherwise returns a formatted
/// string with the header "# Optional Rules (use when relevant):\n\n" followed by each
/// optional rule formatted as "{description}: read this file {path}\n\n"
pub fn generate_optional_rules_content(source_files: &[SourceFile]) -> String {
    let optional_files: Vec<_> = source_files
        .iter()
        .filter(|file| !file.front_matter.always_apply)
        .collect();

    if optional_files.is_empty() {
        return String::new();
    }

    let main_template = OPTIONAL_RULES_TEMPLATE;

    let mut rule_entries = String::new();
    for source_file in optional_files {
        let body_file_name = source_file.get_body_file_name();
        let generated_path = generated_body_file_reference_path(&body_file_name);

        let entry = format!(
            "{}: {}",
            source_file.front_matter.description,
            generated_path.display()
        );

        rule_entries.push_str(&entry);
        rule_entries.push_str("\n\n");
    }

    main_template.replace("{{RULE_ENTRIES}}", &rule_entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source_file::FrontMatter;

    fn create_test_source_file(
        base_name: &str,
        description: &str,
        always_apply: bool,
        file_patterns: Vec<String>,
        body: &str,
    ) -> SourceFile {
        SourceFile {
            front_matter: FrontMatter {
                description: description.to_string(),
                always_apply,
                file_matching_patterns: Some(file_patterns),
            },
            body: body.to_string(),
            base_file_name: base_name.to_string(),
        }
    }

    fn generate_expected_content(rule_entries: &str) -> String {
        OPTIONAL_RULES_TEMPLATE.replace("{{RULE_ENTRIES}}", rule_entries)
    }

    #[test]
    fn test_generate_optional_rules_content_empty() {
        let source_files = vec![];
        let result = generate_optional_rules_content(&source_files);
        assert_eq!(result, "");
    }

    #[test]
    fn test_generate_optional_rules_content_no_optional_files() {
        let source_files = vec![
            create_test_source_file(
                "rule1",
                "Always apply rule 1",
                true,
                vec!["**/*.ts".to_string()],
                "Rule 1 body",
            ),
            create_test_source_file(
                "rule2",
                "Always apply rule 2",
                true,
                vec!["**/*.js".to_string()],
                "Rule 2 body",
            ),
        ];

        let result = generate_optional_rules_content(&source_files);
        assert_eq!(result, "");
    }

    #[test]
    fn test_generate_optional_rules_content_single_optional() {
        let source_files = vec![create_test_source_file(
            "optional_rule",
            "Optional testing rule",
            false,
            vec!["**/*.test.ts".to_string()],
            "Optional rule body",
        )];

        let result = generate_optional_rules_content(&source_files);
        let expected = generate_expected_content(
            "Optional testing rule: ai-rules/.generated-ai-rules/ai-rules-generated-optional_rule.md\n\n",
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_optional_rules_content_multiple_optional() {
        let source_files = vec![
            create_test_source_file(
                "optional1",
                "First optional rule",
                false,
                vec!["**/*.ts".to_string()],
                "Optional 1 body",
            ),
            create_test_source_file(
                "optional2",
                "Second optional rule",
                false,
                vec!["**/*.js".to_string()],
                "Optional 2 body",
            ),
        ];

        let result = generate_optional_rules_content(&source_files);
        let expected = generate_expected_content("First optional rule: ai-rules/.generated-ai-rules/ai-rules-generated-optional1.md\n\nSecond optional rule: ai-rules/.generated-ai-rules/ai-rules-generated-optional2.md\n\n");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_optional_rules_content_mixed_rules() {
        let source_files = vec![
            create_test_source_file(
                "always1",
                "Always apply rule",
                true,
                vec!["**/*.ts".to_string()],
                "Always body",
            ),
            create_test_source_file(
                "optional1",
                "Optional rule",
                false,
                vec!["**/*.js".to_string()],
                "Optional body",
            ),
            create_test_source_file(
                "always2",
                "Another always rule",
                true,
                vec!["**/*.rs".to_string()],
                "Always body 2",
            ),
            create_test_source_file(
                "optional2",
                "Another optional rule",
                false,
                vec!["**/*.py".to_string()],
                "Optional body 2",
            ),
        ];

        let result = generate_optional_rules_content(&source_files);
        let expected = generate_expected_content("Optional rule: ai-rules/.generated-ai-rules/ai-rules-generated-optional1.md\n\nAnother optional rule: ai-rules/.generated-ai-rules/ai-rules-generated-optional2.md\n\n");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_optional_rules_content_preserves_order() {
        let source_files = vec![
            create_test_source_file(
                "rule_c",
                "Rule C",
                false,
                vec!["**/*.c".to_string()],
                "C body",
            ),
            create_test_source_file(
                "rule_a",
                "Rule A",
                false,
                vec!["**/*.a".to_string()],
                "A body",
            ),
            create_test_source_file(
                "rule_b",
                "Rule B",
                false,
                vec!["**/*.b".to_string()],
                "B body",
            ),
        ];

        let result = generate_optional_rules_content(&source_files);

        // Check that the result contains 'Rule C, Rule A, Rule B content' together in the correct order
        assert!(
            result.contains("Rule C:") && result.contains("Rule A:") && result.contains("Rule B:"),
            "Result should contain all three rules"
        );

        // Verify all rules appear in the correct order with a single readable assertion
        let rule_c_pos = result.find("Rule C:").expect("Rule C should be present");
        let rule_a_pos = result.find("Rule A:").expect("Rule A should be present");
        let rule_b_pos = result.find("Rule B:").expect("Rule B should be present");

        assert!(
            rule_c_pos < rule_a_pos && rule_a_pos < rule_b_pos,
            "Rules should appear in order: Rule C, Rule A, Rule B. Found positions: Rule C at {rule_c_pos}, Rule A at {rule_a_pos}, Rule B at {rule_b_pos}"
        );
    }

    #[test]
    fn test_generate_optional_rules_content_special_characters_in_description() {
        let source_files = vec![create_test_source_file(
            "special",
            "Rule with special chars: & < > \" '",
            false,
            vec!["**/*.txt".to_string()],
            "Special body",
        )];

        let result = generate_optional_rules_content(&source_files);
        assert!(result.contains("Rule with special chars: & < > \" ':"));
    }

    #[test]
    fn test_generate_optional_rules_content_empty_description() {
        let source_files = vec![create_test_source_file(
            "empty_desc",
            "",
            false,
            vec!["**/*.txt".to_string()],
            "Body",
        )];

        let result = generate_optional_rules_content(&source_files);
        let expected = generate_expected_content(
            ": ai-rules/.generated-ai-rules/ai-rules-generated-empty_desc.md\n\n",
        );
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_optional_rules_content_long_base_name() {
        let source_files = vec![create_test_source_file(
            "very_long_base_name_for_testing_purposes_that_exceeds_normal_length",
            "Long name rule",
            false,
            vec!["**/*.txt".to_string()],
            "Body",
        )];

        let result = generate_optional_rules_content(&source_files);
        assert!(result.contains(
            "ai-rules/.generated-ai-rules/ai-rules-generated-very_long_base_name_for_testing_purposes_that_exceeds_normal_length.md"
        ));
    }
}

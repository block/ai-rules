#[cfg(test)]
pub mod helpers {
    use std::fs;
    use std::path::Path;

    pub fn create_file(base: &Path, path: &str, content: &str) {
        let full_path = base.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }

    pub fn assert_file_exists(base: &Path, path: &str) {
        assert!(base.join(path).exists(), "Expected {path} to exist");
    }

    pub fn assert_file_not_exists(base: &Path, path: &str) {
        assert!(!base.join(path).exists(), "Expected {path} to not exist");
    }

    pub fn assert_file_content(base: &Path, path: &str, expected_content: &str) {
        let full_path = base.join(path);
        assert!(full_path.exists(), "File {path} does not exist");
        let actual_content = std::fs::read_to_string(&full_path)
            .unwrap_or_else(|_| panic!("Failed to read file {path}"));
        assert_eq!(
            actual_content, expected_content,
            "Content mismatch in file {path}"
        );
    }

    pub fn assert_file_has_trailing_newline(base: &Path, path: &str) {
        let full_path = base.join(path);
        assert!(full_path.exists(), "File {path} does not exist");
        let content =
            std::fs::read(&full_path).unwrap_or_else(|_| panic!("Failed to read file {path}"));
        assert!(
            !content.is_empty() && content.last() == Some(&b'\n'),
            "File {path} does not end with a newline character"
        );
    }

    pub fn create_test_source_file(
        base_name: &str,
        description: &str,
        always_apply: bool,
        file_patterns: Vec<String>,
        body: &str,
    ) -> crate::models::source_file::SourceFile {
        use crate::models::source_file::{FrontMatter, SourceFile};
        SourceFile {
            base_file_name: base_name.to_string(),
            front_matter: FrontMatter {
                description: description.to_string(),
                always_apply,
                file_matching_patterns: Some(file_patterns),
                allowed_agents: None,
                blocked_agents: None,
            },
            body: body.to_string(),
        }
    }
}

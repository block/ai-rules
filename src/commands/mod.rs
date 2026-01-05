mod clean;
mod generate;
mod init;
mod list_agents;
mod status;

pub use clean::run_clean;
pub use generate::run_generate;
pub use init::run_init;
pub use list_agents::run_list_agents;
pub use status::run_status;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{InitArgs, ResolvedGenerateArgs, ResolvedStatusArgs};
    use crate::commands::status::check_project_status;
    use crate::constants::AGENTS_MD_FILENAME;
    use crate::utils::test_utils::helpers::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn verify_symlinks(
        project_path: &Path,
        expected_targets: &[(&str, &str)],
        expected_content: &str,
    ) {
        for (symlink_path, expected_target) in expected_targets {
            let full_path = project_path.join(symlink_path);
            assert!(
                full_path.is_symlink(),
                "Expected {symlink_path} to be a symlink"
            );

            // Verify symlink target
            let actual_target = std::fs::read_link(&full_path).unwrap();
            assert_eq!(
                actual_target,
                Path::new(expected_target),
                "Symlink {symlink_path} should point to {expected_target}, but points to {actual_target:?}"
            );

            // Verify content is accessible through symlink
            let content = std::fs::read_to_string(&full_path).unwrap();
            assert_eq!(
                content, expected_content,
                "Content mismatch for {symlink_path}"
            );
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn test_complete_workflow() {
        let nested_depth = 0;
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        let goose_bin_dir = TempDir::new().unwrap();
        let _path_guard = PathGuard::prepend(goose_bin_dir.path());
        write_failing_goose(goose_bin_dir.path());

        let init_result = run_init(project_path, InitArgs::default());
        assert!(init_result.is_ok());
        let starter_rule_path = "ai-rules/example.md";
        assert_file_exists(project_path, starter_rule_path);

        // Generate - should create generated files
        let generate_args = ResolvedGenerateArgs {
            agents: None,
            command_agents: None,
            gitignore: true,
            nested_depth,
        };
        let generate_result = run_generate(project_path, generate_args, false);
        if let Err(e) = &generate_result {
            panic!("Generate failed with error: {e}");
        }
        assert!(generate_result.is_ok());

        let generated_rule_path = "ai-rules/.generated-ai-rules/ai-rules-generated-example.md";
        assert_file_exists(project_path, generated_rule_path);
        assert_file_exists(project_path, "CLAUDE.md");
        let cursor_rule_path = ".cursor/rules/ai-rules-generated-example.mdc";
        assert_file_exists(project_path, cursor_rule_path);
        assert_file_exists(project_path, AGENTS_MD_FILENAME);
        assert_file_exists(project_path, ".gitignore");

        // Check status - should be in sync
        let status_args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth,
        };
        let status_result = check_project_status(project_path, status_args, false).unwrap();
        assert!(status_result.has_ai_rules);
        assert!(!status_result.body_files_out_of_sync);
        for in_sync in status_result.agent_statuses.values() {
            assert!(*in_sync, "All agents should be in sync after generation");
        }

        // Change one generated file - modify CLAUDE.md
        create_file(project_path, "CLAUDE.md", "modified content");

        // Check status again - should be out of sync
        let status_args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth,
        };
        let status_after_change = check_project_status(project_path, status_args, false).unwrap();
        assert!(status_after_change.has_ai_rules);
        assert!(!status_after_change.body_files_out_of_sync);

        assert!(
            !status_after_change.agent_statuses["claude"],
            "Claude should be out of sync"
        );
        assert!(
            status_after_change.agent_statuses["cursor"],
            "Cursor should still be in sync"
        );
        assert!(
            status_after_change.agent_statuses["goose"],
            "Goose should still be in sync"
        );

        // Clean - should remove all generated files
        let clean_result = run_clean(project_path, nested_depth, false);
        assert!(clean_result.is_ok());

        assert_file_not_exists(project_path, "ai-rules/.generated-ai-rules");
        assert_file_not_exists(project_path, "CLAUDE.md");
        assert_file_not_exists(project_path, ".cursor/rules");
        assert_file_not_exists(project_path, AGENTS_MD_FILENAME);

        assert_file_exists(project_path, starter_rule_path);
        assert_file_exists(project_path, ".gitignore"); // Gitignore remains
    }

    #[test]
    fn test_symlink_mode_all_agents() {
        let nested_depth = 0;
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();
        let symlink_content = "# Symlink content";

        create_file(project_path, "ai-rules/AGENTS.md", symlink_content);

        let generate_args = ResolvedGenerateArgs {
            agents: None,
            command_agents: None,
            gitignore: true,
            nested_depth,
        };
        let generate_result = run_generate(project_path, generate_args, false);
        assert!(generate_result.is_ok());

        // Verify all agents created symlinks pointing to the correct target
        let expected_targets = vec![
            ("CLAUDE.md", "ai-rules/AGENTS.md"),
            (AGENTS_MD_FILENAME, "ai-rules/AGENTS.md"),
        ];

        verify_symlinks(project_path, &expected_targets, symlink_content);

        // Verify no normal generated files exist
        assert_file_not_exists(project_path, "ai-rules/.generated-ai-rules");
        assert_file_not_exists(project_path, ".cursor/rules");
        assert_file_exists(project_path, ".gitignore");

        // Remove CLAUDE.md
        fs::remove_file(project_path.join("CLAUDE.md")).unwrap();

        // Check status again - should be out of sync
        let status_args = ResolvedStatusArgs {
            agents: None,
            command_agents: None,
            nested_depth,
        };
        let status_after_change = check_project_status(project_path, status_args, false).unwrap();
        assert!(status_after_change.has_ai_rules);
        assert!(!status_after_change.body_files_out_of_sync);

        assert!(
            !status_after_change.agent_statuses["claude"],
            "Claude should be out of sync"
        );
        assert!(
            status_after_change.agent_statuses["cursor"],
            "Cursor should still be in sync"
        );
    }

    struct PathGuard {
        original: Option<String>,
    }

    impl PathGuard {
        fn prepend(dir: &Path) -> Self {
            let original = std::env::var("PATH").ok();
            let mut new_path = dir.display().to_string();
            if let Some(ref orig) = original {
                new_path.push(':');
                new_path.push_str(orig);
            }
            std::env::set_var("PATH", &new_path);
            Self { original }
        }
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            if let Some(ref original) = self.original {
                std::env::set_var("PATH", original);
            } else {
                std::env::remove_var("PATH");
            }
        }
    }

    #[cfg(not(windows))]
    fn write_failing_goose(dir: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("goose");
        std::fs::write(&path, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

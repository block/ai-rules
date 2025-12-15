use crate::constants::GENERATED_FILE_PREFIX;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Legacy directory configurations for agents that migrated to AGENTS.md
/// Each entry is (agent_dir, optional_rules_subdir)
#[allow(dead_code)]
const LEGACY_AGENT_DIRS: &[(&str, Option<&str>)] = &[
    (".roo", Some("rules")),
    (".clinerules", None),
    (".kilocode", Some("rules")),
];

/// Cleans up legacy generated files from agents that have migrated to AGENTS.md.
/// Only removes files with the ai-rules-generated- prefix, then removes empty directories.
#[allow(dead_code)]
pub fn clean_legacy_agent_directories(current_dir: &Path) -> Result<()> {
    for (agent_dir, rules_subdir) in LEGACY_AGENT_DIRS {
        let rules_path = if let Some(subdir) = rules_subdir {
            current_dir.join(agent_dir).join(subdir)
        } else {
            current_dir.join(agent_dir)
        };

        if rules_path.exists() && rules_path.is_dir() {
            remove_generated_files_from_directory(&rules_path)?;
            remove_directory_if_empty(&rules_path)?;
        }

        // Try to remove the parent agent directory if it's now empty
        let agent_path = current_dir.join(agent_dir);
        if agent_path.exists() && agent_path.is_dir() {
            remove_directory_if_empty(&agent_path)?;
        }
    }

    Ok(())
}

/// Removes files with the ai-rules-generated- prefix from a directory
#[allow(dead_code)]
fn remove_generated_files_from_directory(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(GENERATED_FILE_PREFIX) {
                    fs::remove_file(&path)?;
                }
            }
        }
    }

    Ok(())
}

/// Removes a directory only if it's empty
#[allow(dead_code)]
fn remove_directory_if_empty(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    // Check if directory is empty
    let is_empty = fs::read_dir(dir)?.next().is_none();

    if is_empty {
        fs::remove_dir(dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_clean_legacy_removes_generated_files_only() {
        let temp_dir = TempDir::new().unwrap();

        // Create legacy roo files (with rules subdir)
        create_file(
            temp_dir.path(),
            ".roo/rules/ai-rules-generated-test.md",
            "generated",
        );
        create_file(temp_dir.path(), ".roo/rules/custom-rule.md", "user file");

        let result = clean_legacy_agent_directories(temp_dir.path());
        assert!(result.is_ok());

        // Generated file should be removed
        assert_file_not_exists(temp_dir.path(), ".roo/rules/ai-rules-generated-test.md");
        // User file should remain
        assert_file_exists(temp_dir.path(), ".roo/rules/custom-rule.md");
        // Directory should remain (not empty)
        assert!(temp_dir.path().join(".roo/rules").exists());
    }

    #[test]
    fn test_clean_legacy_removes_empty_directories() {
        let temp_dir = TempDir::new().unwrap();

        // Create legacy roo files (only generated)
        create_file(
            temp_dir.path(),
            ".roo/rules/ai-rules-generated-test.md",
            "generated",
        );

        let result = clean_legacy_agent_directories(temp_dir.path());
        assert!(result.is_ok());

        // Both rules dir and .roo dir should be removed (both empty)
        assert_file_not_exists(temp_dir.path(), ".roo/rules");
        assert_file_not_exists(temp_dir.path(), ".roo");
    }

    #[test]
    fn test_clean_legacy_preserves_non_empty_parent() {
        let temp_dir = TempDir::new().unwrap();

        // Create legacy roo files
        create_file(
            temp_dir.path(),
            ".roo/rules/ai-rules-generated-test.md",
            "generated",
        );
        create_file(temp_dir.path(), ".roo/mcp.json", "mcp config");

        let result = clean_legacy_agent_directories(temp_dir.path());
        assert!(result.is_ok());

        // Rules dir should be removed (empty after cleanup)
        assert_file_not_exists(temp_dir.path(), ".roo/rules");
        // .roo dir should remain (has mcp.json)
        assert!(temp_dir.path().join(".roo").exists());
        assert_file_exists(temp_dir.path(), ".roo/mcp.json");
    }

    #[test]
    fn test_clean_legacy_clinerules_no_subdir() {
        let temp_dir = TempDir::new().unwrap();

        // Create legacy cline files (no rules subdir)
        create_file(
            temp_dir.path(),
            ".clinerules/ai-rules-generated-test.md",
            "generated",
        );

        let result = clean_legacy_agent_directories(temp_dir.path());
        assert!(result.is_ok());

        // Directory should be removed
        assert_file_not_exists(temp_dir.path(), ".clinerules");
    }

    #[test]
    fn test_clean_legacy_kilocode() {
        let temp_dir = TempDir::new().unwrap();

        // Create legacy kilocode files
        create_file(
            temp_dir.path(),
            ".kilocode/rules/ai-rules-generated-test.md",
            "generated",
        );

        let result = clean_legacy_agent_directories(temp_dir.path());
        assert!(result.is_ok());

        // Both should be removed
        assert_file_not_exists(temp_dir.path(), ".kilocode/rules");
        assert_file_not_exists(temp_dir.path(), ".kilocode");
    }

    #[test]
    fn test_clean_legacy_nonexistent_dirs() {
        let temp_dir = TempDir::new().unwrap();

        // No legacy directories exist
        let result = clean_legacy_agent_directories(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_clean_legacy_multiple_generated_files() {
        let temp_dir = TempDir::new().unwrap();

        create_file(
            temp_dir.path(),
            ".roo/rules/ai-rules-generated-rule1.md",
            "rule1",
        );
        create_file(
            temp_dir.path(),
            ".roo/rules/ai-rules-generated-rule2.md",
            "rule2",
        );
        create_file(
            temp_dir.path(),
            ".roo/rules/ai-rules-generated-optional.md",
            "optional",
        );

        let result = clean_legacy_agent_directories(temp_dir.path());
        assert!(result.is_ok());

        assert_file_not_exists(temp_dir.path(), ".roo");
    }
}

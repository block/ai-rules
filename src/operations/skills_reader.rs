use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::constants::{AI_RULE_SOURCE_DIR, GENERATED_FILE_PREFIX, SKILLS_DIR, SKILL_FILENAME};
use crate::utils::file_utils::{calculate_relative_path, create_relative_symlink};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SkillFolder {
    pub name: String,
    pub relative_path: PathBuf,
    pub full_path: PathBuf,
}

/// Finds all valid skill folders in ai-rules/skills/ directory
#[allow(dead_code)]
pub fn find_skill_folders(current_dir: &Path) -> Result<Vec<SkillFolder>> {
    let skills_dir = current_dir.join(AI_RULE_SOURCE_DIR).join(SKILLS_DIR);

    // If the skills directory doesn't exist, return empty list
    if !skills_dir.exists() || !skills_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut skill_folders = Vec::new();

    for entry in fs::read_dir(&skills_dir)
        .with_context(|| format!("Failed to read skills directory: {}", skills_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        // Skip non-directories with a warning
        if !path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                eprintln!(
                    "Warning: Skipping '{}' in skills directory - not a directory",
                    name
                );
            }
            continue;
        }

        // Check if SKILL.md exists in this folder
        let skill_file = path.join(SKILL_FILENAME);
        if !skill_file.exists() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                eprintln!(
                    "Warning: Skipping '{}' - missing {} file",
                    name, SKILL_FILENAME
                );
            }
            continue;
        }

        // Get the folder name
        if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
            let relative_path = PathBuf::from(AI_RULE_SOURCE_DIR)
                .join(SKILLS_DIR)
                .join(folder_name);

            skill_folders.push(SkillFolder {
                name: folder_name.to_string(),
                relative_path,
                full_path: path,
            });
        }
    }

    Ok(skill_folders)
}

/// Creates symlinks for each skill folder in the target directory
#[allow(dead_code)]
pub fn create_skill_symlinks(current_dir: &Path, target_dir: &str) -> Result<Vec<PathBuf>> {
    let skill_folders = find_skill_folders(current_dir)?;

    if skill_folders.is_empty() {
        return Ok(Vec::new());
    }

    let mut created_symlinks = Vec::new();

    for skill in skill_folders {
        // Create symlink name with prefix: ai-rules-generated-<name>
        let symlink_name = format!("{}{}", GENERATED_FILE_PREFIX, skill.name);
        let from_path = PathBuf::from(target_dir).join(&symlink_name);

        // Calculate the relative path from symlink location to source
        let relative_source = calculate_relative_path(&from_path, &skill.relative_path);

        // Create the actual symlink
        let symlink_path = current_dir.join(&from_path);
        create_relative_symlink(&symlink_path, &relative_source)?;

        created_symlinks.push(symlink_path);
    }

    Ok(created_symlinks)
}

/// Removes generated skill symlinks from target directory
#[allow(dead_code)]
pub fn remove_generated_skill_symlinks(current_dir: &Path, target_dir: &str) -> Result<()> {
    let target_path = current_dir.join(target_dir);

    if !target_path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&target_path)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            // Remove any file/symlink that starts with our generated prefix.
            // Note: fs::remove_file only works on files and symlinks, not directories,
            // so this won't accidentally remove directories from experimental claude skills.
            if file_name.starts_with(GENERATED_FILE_PREFIX) {
                fs::remove_file(&path)
                    .with_context(|| format!("Failed to remove: {}", path.display()))?;
            }
        }
    }

    Ok(())
}

/// Checks if generated skill symlinks are in sync
#[allow(dead_code)]
pub fn check_skill_symlinks_in_sync(current_dir: &Path, target_dir: &str) -> Result<bool> {
    let skill_folders = find_skill_folders(current_dir)?;
    let target_path = current_dir.join(target_dir);

    // If no source skills exist, check that no generated symlinks exist
    if skill_folders.is_empty() {
        if !target_path.exists() {
            return Ok(true);
        }

        // Check for any orphaned generated symlinks
        for entry in fs::read_dir(&target_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with(GENERATED_FILE_PREFIX) && path.is_symlink() {
                    // Found an orphaned generated symlink
                    return Ok(false);
                }
            }
        }
        return Ok(true);
    }

    // Check each source skill has a corresponding symlink
    for skill in &skill_folders {
        let symlink_name = format!("{}{}", GENERATED_FILE_PREFIX, skill.name);
        let symlink_path = target_path.join(&symlink_name);

        // Check symlink exists
        if !symlink_path.is_symlink() {
            return Ok(false);
        }

        // Check symlink points to correct target
        let actual_target = fs::read_link(&symlink_path)?;
        let resolved_target = if actual_target.is_absolute() {
            actual_target
        } else {
            let symlink_parent = symlink_path.parent().unwrap_or(current_dir);
            symlink_parent.join(&actual_target)
        };

        let resolved_canonical = resolved_target
            .canonicalize()
            .unwrap_or(resolved_target.clone());
        let expected_canonical = skill
            .full_path
            .canonicalize()
            .unwrap_or(skill.full_path.clone());

        if resolved_canonical != expected_canonical {
            return Ok(false);
        }
    }

    // Check for orphaned generated symlinks
    if target_path.exists() {
        for entry in fs::read_dir(&target_path)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with(GENERATED_FILE_PREFIX) && path.is_symlink() {
                    // Extract the skill name from the symlink name
                    let skill_name = file_name.strip_prefix(GENERATED_FILE_PREFIX).unwrap_or("");

                    // Check if this skill exists in our source skills
                    let skill_exists = skill_folders.iter().any(|s| s.name == skill_name);

                    if !skill_exists {
                        // Orphaned symlink found
                        return Ok(false);
                    }
                }
            }
        }
    }

    Ok(true)
}

/// Returns gitignore patterns for generated skill symlinks
#[allow(dead_code)]
pub fn get_skill_gitignore_patterns(target_dir: &str) -> Vec<String> {
    vec![format!("{}/{}*", target_dir, GENERATED_FILE_PREFIX)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_skill_folder(temp_dir: &Path, skill_name: &str, content: &str) -> PathBuf {
        let skill_dir = temp_dir
            .join(AI_RULE_SOURCE_DIR)
            .join(SKILLS_DIR)
            .join(skill_name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join(SKILL_FILENAME), content).unwrap();
        skill_dir
    }

    #[test]
    fn test_find_skill_folders_empty_when_no_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_skill_folders(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_find_skill_folders_empty_when_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(SKILLS_DIR);
        fs::create_dir_all(&skills_dir).unwrap();

        let result = find_skill_folders(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_find_skill_folders_finds_valid_skills() {
        let temp_dir = TempDir::new().unwrap();

        create_skill_folder(temp_dir.path(), "my-skill", "skill content");
        create_skill_folder(temp_dir.path(), "another-skill", "more content");

        let result = find_skill_folders(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 2);

        let names: Vec<String> = result.iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"my-skill".to_string()));
        assert!(names.contains(&"another-skill".to_string()));
    }

    #[test]
    fn test_find_skill_folders_requires_skill_md() {
        let temp_dir = TempDir::new().unwrap();

        // Create a folder without SKILL.md
        let skill_dir = temp_dir
            .path()
            .join(AI_RULE_SOURCE_DIR)
            .join(SKILLS_DIR)
            .join("invalid-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("README.md"), "not a skill").unwrap();

        // Create a valid skill folder
        create_skill_folder(temp_dir.path(), "valid-skill", "skill content");

        let result = find_skill_folders(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "valid-skill");
    }

    #[test]
    fn test_find_skill_folders_ignores_files() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(SKILLS_DIR);
        fs::create_dir_all(&skills_dir).unwrap();

        // Create a file in the skills directory (should be ignored)
        fs::write(skills_dir.join("readme.md"), "not a skill").unwrap();

        // Create a valid skill folder
        create_skill_folder(temp_dir.path(), "valid-skill", "skill content");

        let result = find_skill_folders(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "valid-skill");
    }

    #[test]
    fn test_create_skill_symlinks() {
        let temp_dir = TempDir::new().unwrap();

        create_skill_folder(temp_dir.path(), "my-skill", "skill content");
        create_skill_folder(temp_dir.path(), "another-skill", "more content");

        let symlinks = create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();
        assert_eq!(symlinks.len(), 2);

        // Check symlinks exist
        let skill1_symlink = temp_dir
            .path()
            .join(".claude/skills")
            .join(format!("{}my-skill", GENERATED_FILE_PREFIX));
        let skill2_symlink = temp_dir
            .path()
            .join(".claude/skills")
            .join(format!("{}another-skill", GENERATED_FILE_PREFIX));

        assert!(skill1_symlink.is_symlink());
        assert!(skill2_symlink.is_symlink());

        // Check we can read through the symlinks
        let content = fs::read_to_string(skill1_symlink.join(SKILL_FILENAME)).unwrap();
        assert_eq!(content, "skill content");
    }

    #[test]
    fn test_create_skill_symlinks_no_skills() {
        let temp_dir = TempDir::new().unwrap();

        let symlinks = create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();
        assert_eq!(symlinks.len(), 0);
    }

    #[test]
    fn test_remove_skill_symlinks_preserves_user_skills() {
        let temp_dir = TempDir::new().unwrap();

        // Create source skills and generate symlinks
        create_skill_folder(temp_dir.path(), "my-skill", "skill content");
        create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();

        // Create a user's custom skill (not a symlink, a real folder)
        let user_skill = temp_dir
            .path()
            .join(".claude/skills")
            .join("user-custom-skill");
        fs::create_dir_all(&user_skill).unwrap();
        fs::write(user_skill.join(SKILL_FILENAME), "user content").unwrap();

        // Remove generated symlinks
        remove_generated_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();

        // Check generated symlink is gone
        let generated = temp_dir
            .path()
            .join(".claude/skills")
            .join(format!("{}my-skill", GENERATED_FILE_PREFIX));
        assert!(!generated.exists());

        // Check user skill still exists
        assert!(user_skill.exists());
        let content = fs::read_to_string(user_skill.join(SKILL_FILENAME)).unwrap();
        assert_eq!(content, "user content");
    }

    #[test]
    fn test_check_skill_symlinks_in_sync() {
        let temp_dir = TempDir::new().unwrap();

        // Create source skills
        create_skill_folder(temp_dir.path(), "my-skill", "skill content");

        // Before generating, should be out of sync
        let result = check_skill_symlinks_in_sync(temp_dir.path(), ".claude/skills").unwrap();
        assert!(!result);

        // Generate symlinks
        create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();

        // Now should be in sync
        let result = check_skill_symlinks_in_sync(temp_dir.path(), ".claude/skills").unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_skill_symlinks_detects_orphaned() {
        let temp_dir = TempDir::new().unwrap();

        // Create and generate skills
        create_skill_folder(temp_dir.path(), "my-skill", "skill content");
        create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();

        // Create an orphaned symlink manually
        let orphaned_path = temp_dir
            .path()
            .join(".claude/skills")
            .join(format!("{}orphaned-skill", GENERATED_FILE_PREFIX));
        let fake_target = temp_dir.path().join("fake");
        fs::create_dir_all(&fake_target).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&fake_target, &orphaned_path).unwrap();

        // Should be out of sync due to orphaned symlink
        let result = check_skill_symlinks_in_sync(temp_dir.path(), ".claude/skills").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_skill_symlinks_no_source_skills() {
        let temp_dir = TempDir::new().unwrap();

        // No source skills and no target directory - should be in sync
        let result = check_skill_symlinks_in_sync(temp_dir.path(), ".claude/skills").unwrap();
        assert!(result);
    }

    #[test]
    fn test_get_skill_gitignore_patterns() {
        let patterns = get_skill_gitignore_patterns(".claude/skills");
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0], ".claude/skills/ai-rules-generated-*");
    }

    #[test]
    fn test_skill_folder_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();

        // Create skill folders with various special characters
        create_skill_folder(temp_dir.path(), "my-skill", "dash skill");
        create_skill_folder(temp_dir.path(), "my_skill", "underscore skill");
        create_skill_folder(temp_dir.path(), "my.skill", "dot skill");

        let result = find_skill_folders(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 3);

        // Create symlinks and verify they work
        let symlinks = create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();
        assert_eq!(symlinks.len(), 3);

        // Verify we can read through all symlinks
        for symlink in symlinks {
            let content = fs::read_to_string(symlink.join(SKILL_FILENAME)).unwrap();
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn test_skill_folder_with_additional_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create skill folder with additional files
        let skill_dir = create_skill_folder(temp_dir.path(), "my-skill", "main skill content");
        fs::write(skill_dir.join("helper.md"), "helper content").unwrap();
        fs::create_dir_all(skill_dir.join("examples")).unwrap();
        fs::write(skill_dir.join("examples/example1.md"), "example content").unwrap();

        // Create symlink
        let symlinks = create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();
        assert_eq!(symlinks.len(), 1);

        let symlink = &symlinks[0];

        // Verify all files are accessible through the symlink
        assert_eq!(
            fs::read_to_string(symlink.join(SKILL_FILENAME)).unwrap(),
            "main skill content"
        );
        assert_eq!(
            fs::read_to_string(symlink.join("helper.md")).unwrap(),
            "helper content"
        );
        assert_eq!(
            fs::read_to_string(symlink.join("examples/example1.md")).unwrap(),
            "example content"
        );
    }

    #[test]
    fn test_skill_source_is_symlink() {
        let temp_dir = TempDir::new().unwrap();

        // Create a real skill folder somewhere else
        let actual_skill_dir = temp_dir.path().join("external-skills/shared-skill");
        fs::create_dir_all(&actual_skill_dir).unwrap();
        fs::write(actual_skill_dir.join(SKILL_FILENAME), "shared content").unwrap();

        // Create ai-rules/skills directory
        let skills_dir = temp_dir.path().join(AI_RULE_SOURCE_DIR).join(SKILLS_DIR);
        fs::create_dir_all(&skills_dir).unwrap();

        // Create a symlink in skills directory pointing to the external skill
        let symlink_source = skills_dir.join("shared-skill");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&actual_skill_dir, &symlink_source).unwrap();

        // Find should discover the skill through the symlink
        let result = find_skill_folders(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "shared-skill");

        // Generate symlinks
        let symlinks = create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();
        assert_eq!(symlinks.len(), 1);

        // Content should be accessible
        let content = fs::read_to_string(symlinks[0].join(SKILL_FILENAME)).unwrap();
        assert_eq!(content, "shared content");
    }

    #[test]
    fn test_broken_symlink_in_target_is_cleaned() {
        let temp_dir = TempDir::new().unwrap();

        // Create target directory with a broken symlink
        let target_dir = temp_dir.path().join(".claude/skills");
        fs::create_dir_all(&target_dir).unwrap();

        let broken_symlink = target_dir.join(format!("{}broken-skill", GENERATED_FILE_PREFIX));
        #[cfg(unix)]
        std::os::unix::fs::symlink("/nonexistent/path", &broken_symlink).unwrap();

        // Verify broken symlink exists
        assert!(broken_symlink.is_symlink());
        assert!(!broken_symlink.exists()); // exists() returns false for broken symlinks

        // Remove should clean up broken symlinks
        remove_generated_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();

        // Broken symlink should be removed
        assert!(!broken_symlink.is_symlink());
    }

    #[test]
    fn test_check_detects_broken_symlink() {
        let temp_dir = TempDir::new().unwrap();

        // Create a skill, generate symlinks, then delete the source
        create_skill_folder(temp_dir.path(), "my-skill", "content");
        create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();

        // Verify in sync
        assert!(check_skill_symlinks_in_sync(temp_dir.path(), ".claude/skills").unwrap());

        // Delete the source skill (but leave the symlink)
        fs::remove_dir_all(
            temp_dir
                .path()
                .join(AI_RULE_SOURCE_DIR)
                .join(SKILLS_DIR)
                .join("my-skill"),
        )
        .unwrap();

        // Should now be out of sync (orphaned symlink pointing to deleted source)
        assert!(!check_skill_symlinks_in_sync(temp_dir.path(), ".claude/skills").unwrap());
    }

    #[test]
    fn test_regenerate_overwrites_existing_symlink() {
        let temp_dir = TempDir::new().unwrap();

        // Create initial skill
        create_skill_folder(temp_dir.path(), "my-skill", "original content");
        create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();

        // Verify initial content
        let symlink_path = temp_dir
            .path()
            .join(".claude/skills")
            .join(format!("{}my-skill", GENERATED_FILE_PREFIX));
        assert_eq!(
            fs::read_to_string(symlink_path.join(SKILL_FILENAME)).unwrap(),
            "original content"
        );

        // Update skill content
        let skill_path = temp_dir
            .path()
            .join(AI_RULE_SOURCE_DIR)
            .join(SKILLS_DIR)
            .join("my-skill")
            .join(SKILL_FILENAME);
        fs::write(&skill_path, "updated content").unwrap();

        // Regenerate should work without error (symlink already exists)
        create_skill_symlinks(temp_dir.path(), ".claude/skills").unwrap();

        // Content should be updated (same symlink, but source changed)
        assert_eq!(
            fs::read_to_string(symlink_path.join(SKILL_FILENAME)).unwrap(),
            "updated content"
        );
    }
}

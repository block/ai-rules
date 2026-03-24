use crate::constants::{AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR};
use crate::operations::body_generator::inlined_agents_relative_path;
use anyhow::Result;

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

// Re-export DirectoryFilter so existing callers don't need to change their imports
pub use crate::utils::dir_filter::DirectoryFilter;
use crate::utils::dir_filter::TraversalDecision;

/// Ensures a string ends with a newline character.
/// This is a helper to maintain POSIX compliance for generated files.
pub fn ensure_trailing_newline(content: impl Into<String>) -> String {
    let content = content.into();
    if content.ends_with('\n') {
        content
    } else {
        format!("{content}\n")
    }
}

pub fn find_files_by_extension(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        let metadata = fs::metadata(&path)?;

        if metadata.is_file() && path.extension().is_some_and(|ext| ext == extension) {
            files.push(path);
        }
    }

    // Sort files alphabetically for deterministic output across filesystems
    files.sort();

    Ok(files)
}

pub fn create_relative_symlink(symlink_path: &Path, relative_target: &Path) -> Result<()> {
    if let Some(parent) = symlink_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    if symlink_path.exists() || symlink_path.is_symlink() {
        fs::remove_file(symlink_path)?;
    }

    unix_fs::symlink(relative_target, symlink_path)?;
    Ok(())
}

pub fn calculate_relative_path(from_path: &Path, target_relative_to_root: &Path) -> PathBuf {
    let slash_count = from_path.to_str().unwrap_or("").matches('/').count();
    let up_dirs = "../".repeat(slash_count);
    PathBuf::from(up_dirs + &target_relative_to_root.display().to_string())
}

pub fn create_symlink_to_agents_md(current_dir: &Path, output_path: &Path) -> Result<bool> {
    let source_full_path = current_dir
        .join(AI_RULE_SOURCE_DIR)
        .join(AGENTS_MD_FILENAME);

    if !source_full_path.exists() {
        return Ok(false);
    }

    let link = current_dir.join(output_path);
    let source_relative = PathBuf::from(AI_RULE_SOURCE_DIR).join(AGENTS_MD_FILENAME);
    let relative_source = calculate_relative_path(output_path, &source_relative);

    create_relative_symlink(&link, &relative_source)?;

    Ok(true)
}

pub fn check_agents_md_symlink(current_dir: &Path, symlink_path: &Path) -> Result<bool> {
    if !symlink_path.is_symlink() {
        return Ok(false);
    }

    let expected_target = current_dir
        .join(AI_RULE_SOURCE_DIR)
        .join(AGENTS_MD_FILENAME);
    let actual_target = fs::read_link(symlink_path)?;

    let resolved_target = if actual_target.is_absolute() {
        actual_target
    } else {
        // For relative paths, resolve from the symlink's parent directory
        let symlink_parent = symlink_path.parent().unwrap_or(current_dir);
        symlink_parent.join(&actual_target)
    };

    // Canonicalize both paths to handle ".." components properly
    let resolved_canonical = resolved_target.canonicalize().unwrap_or(resolved_target);
    let expected_canonical = expected_target
        .canonicalize()
        .unwrap_or_else(|_| expected_target.clone());

    Ok(resolved_canonical == expected_canonical && expected_target.exists())
}

pub fn create_symlink_to_inlined_file(current_dir: &Path, output_path: &Path) -> Result<bool> {
    let inlined_relative = inlined_agents_relative_path();
    let source_full_path = current_dir.join(&inlined_relative);

    if !source_full_path.exists() {
        return Ok(false);
    }

    let link = current_dir.join(output_path);
    let relative_source = calculate_relative_path(output_path, &inlined_relative);

    create_relative_symlink(&link, &relative_source)?;

    Ok(true)
}

pub fn check_inlined_file_symlink(current_dir: &Path, symlink_path: &Path) -> Result<bool> {
    if !symlink_path.is_symlink() {
        return Ok(false);
    }

    let inlined_relative = inlined_agents_relative_path();
    let expected_target = current_dir.join(&inlined_relative);
    let actual_target = fs::read_link(symlink_path)?;

    let resolved_target = if actual_target.is_absolute() {
        actual_target
    } else {
        let symlink_parent = symlink_path.parent().unwrap_or(current_dir);
        symlink_parent.join(&actual_target)
    };

    let resolved_canonical = resolved_target.canonicalize().unwrap_or(resolved_target);
    let expected_canonical = expected_target
        .canonicalize()
        .unwrap_or_else(|_| expected_target.clone());

    Ok(resolved_canonical == expected_canonical && expected_target.exists())
}

pub fn write_directory_files(files_to_write: &HashMap<PathBuf, String>) -> Result<()> {
    for (file_path, content) in files_to_write {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, content)?;
    }

    Ok(())
}

pub fn traverse_project_directories<F>(
    current_dir: &Path,
    max_depth: usize,
    current_depth: usize,
    filter: &DirectoryFilter,
    callback: &mut F,
) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    callback(current_dir)?;
    if current_depth >= max_depth {
        return Ok(());
    }

    traverse_children(
        current_dir,
        max_depth,
        current_depth,
        filter,
        false,
        callback,
    )
}

fn traverse_children<F>(
    current_dir: &Path,
    max_depth: usize,
    current_depth: usize,
    filter: &DirectoryFilter,
    parent_ignored: bool,
    callback: &mut F,
) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    // Collect and sort directories for deterministic traversal order
    let mut dirs: Vec<(PathBuf, TraversalDecision)> = Vec::new();
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            let decision = filter.traversal_decision(&path, dir_name, parent_ignored);
            if decision != TraversalDecision::Skip {
                dirs.push((path, decision));
            }
        }
    }

    // Sort directories alphabetically for consistent order
    dirs.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (dir, decision) in dirs {
        if current_depth + 1 >= max_depth {
            if decision == TraversalDecision::Enter {
                callback(&dir)?;
            }
            continue;
        }

        // Only clone/rebuild the filter when a child .gitignore exists
        let child_filter_owned;
        let effective_filter = if let Some(cf) = filter.with_child_gitignore(&dir) {
            child_filter_owned = cf;
            &child_filter_owned
        } else {
            filter
        };

        match decision {
            TraversalDecision::Enter => {
                callback(&dir)?;
                traverse_children(
                    &dir,
                    max_depth,
                    current_depth + 1,
                    effective_filter,
                    false,
                    callback,
                )?;
            }
            TraversalDecision::SkipCallbackButRecurse => {
                traverse_children(
                    &dir,
                    max_depth,
                    current_depth + 1,
                    effective_filter,
                    true,
                    callback,
                )?;
            }
            TraversalDecision::Skip => unreachable!(),
        }
    }

    Ok(())
}

pub fn check_directory_exact_match(
    dir: &Path,
    expected_files: &HashMap<PathBuf, String>,
) -> Result<bool> {
    if !dir.exists() {
        return Ok(false);
    }

    let actual_files: Vec<_> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .collect();

    if actual_files.len() != expected_files.len() {
        return Ok(false);
    }

    for (file_path, expected_content) in expected_files {
        if !file_path.exists() {
            return Ok(false);
        }
        let actual_content = fs::read_to_string(file_path)?;
        if actual_content != *expected_content {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn test_find_files_by_extension() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join("test1.txt"), "content1").unwrap();
        fs::write(temp_path.join("test2.txt"), "content2").unwrap();
        fs::write(temp_path.join("test3.rs"), "content3").unwrap();
        fs::write(temp_path.join("no_extension"), "content4").unwrap();

        let txt_files = find_files_by_extension(temp_path, "txt").unwrap();
        assert_eq!(txt_files.len(), 2);

        let rs_files = find_files_by_extension(temp_path, "rs").unwrap();
        assert_eq!(rs_files.len(), 1);

        let nonexistent_files = find_files_by_extension(temp_path, "xyz").unwrap();
        assert_eq!(nonexistent_files.len(), 0);
    }

    #[test]
    fn test_find_files_by_extension_returns_sorted_results() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create files in a deliberately non-alphabetical order to test sorting
        // These names are chosen to expose different filesystem ordering behaviors:
        // - Mixed case (README vs run-tests)
        // - Hyphens vs underscores
        // - Numbers
        fs::write(temp_path.join("run-tests-after-all-changes.md"), "content").unwrap();
        fs::write(temp_path.join("README.md"), "content").unwrap();
        fs::write(temp_path.join("check-the-diff.md"), "content").unwrap();
        fs::write(
            temp_path.join("read-the-inventory-basics-doc.md"),
            "content",
        )
        .unwrap();
        fs::write(temp_path.join("zebra.md"), "content").unwrap();
        fs::write(temp_path.join("apple.md"), "content").unwrap();
        fs::write(temp_path.join("01-first.md"), "content").unwrap();
        fs::write(temp_path.join("10-tenth.md"), "content").unwrap();
        fs::write(temp_path.join("02-second.md"), "content").unwrap();

        let md_files = find_files_by_extension(temp_path, "md").unwrap();
        assert_eq!(md_files.len(), 9);

        // Extract just the filenames for easier assertion
        let filenames: Vec<String> = md_files
            .iter()
            .filter_map(|p| p.file_name())
            .filter_map(|n| n.to_str())
            .map(|s| s.to_string())
            .collect();

        // Verify files are in alphabetical order
        let mut expected = filenames.clone();
        expected.sort();
        assert_eq!(
            filenames, expected,
            "Files should be returned in alphabetical order"
        );

        // Verify the specific order matches what we expect
        assert_eq!(filenames[0], "01-first.md");
        assert_eq!(filenames[1], "02-second.md");
        assert_eq!(filenames[2], "10-tenth.md");
        assert_eq!(filenames[3], "README.md");
        assert_eq!(filenames[4], "apple.md");
        assert_eq!(filenames[5], "check-the-diff.md");
        assert_eq!(filenames[6], "read-the-inventory-basics-doc.md");
        assert_eq!(filenames[7], "run-tests-after-all-changes.md");
        assert_eq!(filenames[8], "zebra.md");
    }

    #[test]
    fn test_write_directory_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let mut files_to_write = HashMap::new();
        files_to_write.insert(temp_path.join("file1.txt"), "content1".to_string());
        files_to_write.insert(temp_path.join("subdir/file2.txt"), "content2".to_string());

        write_directory_files(&files_to_write).unwrap();

        assert!(temp_path.join("file1.txt").exists());
        assert!(temp_path.join("subdir/file2.txt").exists());

        let content1 = fs::read_to_string(temp_path.join("file1.txt")).unwrap();
        assert_eq!(content1, "content1");

        let content2 = fs::read_to_string(temp_path.join("subdir/file2.txt")).unwrap();
        assert_eq!(content2, "content2");
    }

    #[test]
    fn test_check_directory_exact_match() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let mut expected_files = HashMap::new();
        expected_files.insert(temp_path.join("file1.txt"), "content1".to_string());
        expected_files.insert(temp_path.join("file2.txt"), "content2".to_string());

        fs::write(temp_path.join("file1.txt"), "content1").unwrap();
        fs::write(temp_path.join("file2.txt"), "content2").unwrap();

        assert!(check_directory_exact_match(temp_path, &expected_files).unwrap());

        fs::write(temp_path.join("file2.txt"), "different_content").unwrap();
        assert!(!check_directory_exact_match(temp_path, &expected_files).unwrap());

        fs::write(temp_path.join("extra_file.txt"), "extra").unwrap();
        assert!(!check_directory_exact_match(temp_path, &expected_files).unwrap());
    }

    #[test]
    fn test_traverse_project_directories() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join("src/utils")).unwrap();
        fs::create_dir_all(temp_path.join("target")).unwrap();
        fs::create_dir_all(temp_path.join(".git")).unwrap();
        fs::create_dir_all(temp_path.join("tests")).unwrap();

        let mut visited = Vec::new();
        let mut callback = |path: &Path| -> Result<()> {
            visited.push(path.to_path_buf());
            Ok(())
        };

        traverse_project_directories(temp_path, 2, 0, &DirectoryFilter::Hardcoded, &mut callback)
            .unwrap();

        assert!(visited.contains(&temp_path.to_path_buf()));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "tests"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "target"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == ".git"));
    }

    fn setup_test_directory_structure() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join("src/utils/deep")).unwrap();
        fs::create_dir_all(temp_path.join("tests/unit/helpers")).unwrap();
        fs::create_dir_all(temp_path.join("docs")).unwrap();

        temp_dir
    }

    fn traverse_and_collect(root_path: &Path, max_depth: usize) -> Vec<PathBuf> {
        traverse_and_collect_with_filter(root_path, max_depth, &DirectoryFilter::Hardcoded)
    }

    fn traverse_and_collect_with_filter(
        root_path: &Path,
        max_depth: usize,
        filter: &DirectoryFilter,
    ) -> Vec<PathBuf> {
        let mut visited = Vec::new();
        let mut callback = |path: &Path| -> Result<()> {
            visited.push(path.to_path_buf());
            Ok(())
        };
        traverse_project_directories(root_path, max_depth, 0, filter, &mut callback).unwrap();
        visited
    }

    #[test]
    fn test_traverse_depth_0_only_root_directory() {
        let temp_dir = setup_test_directory_structure();
        let visited = traverse_and_collect(temp_dir.path(), 0);

        assert_eq!(visited.len(), 1);
        assert!(visited.contains(&temp_dir.path().to_path_buf()));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "src"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "tests"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "docs"));
    }

    #[test]
    fn test_traverse_depth_1_includes_direct_children() {
        let temp_dir = setup_test_directory_structure();
        let visited = traverse_and_collect(temp_dir.path(), 1);

        assert!(visited.contains(&temp_dir.path().to_path_buf()));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "tests"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "docs"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "utils"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "unit"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "deep"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "helpers"));
    }

    #[test]
    fn test_traverse_depth_2_includes_grandchildren() {
        let temp_dir = setup_test_directory_structure();
        let visited = traverse_and_collect(temp_dir.path(), 2);

        assert!(visited.iter().any(|p| p.file_name().unwrap() == "utils"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "unit"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "deep"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "helpers"));
    }

    #[test]
    fn test_create_symlink_to_agents_md_success() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(AI_RULE_SOURCE_DIR)).unwrap();
        fs::write(
            temp_path.join(AI_RULE_SOURCE_DIR).join(AGENTS_MD_FILENAME),
            "# Test content",
        )
        .unwrap();

        let result = create_symlink_to_agents_md(temp_path, Path::new(AGENTS_MD_FILENAME)).unwrap();

        assert!(result);
        let symlink_path = temp_path.join(AGENTS_MD_FILENAME);
        assert!(symlink_path.is_symlink());

        let content = fs::read_to_string(&symlink_path).unwrap();
        assert_eq!(content, "# Test content");
    }

    #[test]
    fn test_create_symlink_to_agents_md_nested_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(AI_RULE_SOURCE_DIR)).unwrap();
        fs::write(
            temp_path.join(AI_RULE_SOURCE_DIR).join(AGENTS_MD_FILENAME),
            "# Nested test",
        )
        .unwrap();

        let nested_output = Path::new("nested/dir").join(AGENTS_MD_FILENAME);
        let result = create_symlink_to_agents_md(temp_path, &nested_output).unwrap();

        assert!(result);
        let symlink_path = temp_path.join(&nested_output);
        assert!(symlink_path.exists());
        assert!(symlink_path.is_symlink());

        let content = fs::read_to_string(&symlink_path).unwrap();
        assert_eq!(content, "# Nested test");
    }

    #[test]
    fn test_check_agents_md_symlink_not_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join("CLAUDE.md"), "regular file content").unwrap();

        let result = check_agents_md_symlink(temp_path, &temp_path.join("CLAUDE.md")).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_agents_md_symlink_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let result = check_agents_md_symlink(temp_path, &temp_path.join("CLAUDE.md")).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_check_agents_md_symlink_correct_target() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join("ai-rules")).unwrap();
        fs::write(temp_path.join("ai-rules/AGENTS.md"), "# Source content").unwrap();

        let result = create_symlink_to_agents_md(temp_path, Path::new("CLAUDE.md"));
        assert!(result.is_ok());

        let symlink_path = temp_path.join("CLAUDE.md");

        let result = check_agents_md_symlink(temp_path, &symlink_path).unwrap();
        assert!(result);
    }

    #[test]
    #[cfg(unix)]
    fn test_check_agents_md_symlink_wrong_target() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join("ai-rules")).unwrap();
        fs::write(temp_path.join("ai-rules/AGENTS.md"), "# Source content").unwrap();

        fs::write(temp_path.join("wrong-target.md"), "# Wrong content").unwrap();

        let symlink_path = temp_path.join("CLAUDE.md");
        symlink("wrong-target.md", &symlink_path).unwrap();

        let result = check_agents_md_symlink(temp_path, &symlink_path).unwrap();
        assert!(!result);
    }

    #[test]
    #[cfg(unix)]
    fn test_check_agents_md_symlink_missing_source() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let symlink_path = temp_path.join("CLAUDE.md");
        symlink("ai-rules/AGENTS.md", &symlink_path).unwrap();

        let result = check_agents_md_symlink(temp_path, &symlink_path).unwrap();
        assert!(!result);
    }

    #[test]
    #[cfg(unix)]
    fn test_find_files_by_extension_with_symlinks_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join("regular.md"), "regular file").unwrap();
        fs::write(temp_path.join("target.md"), "target file").unwrap();
        symlink(temp_path.join("target.md"), temp_path.join("link.md")).unwrap();

        let md_files = find_files_by_extension(temp_path, "md").unwrap();
        assert_eq!(md_files.len(), 3);

        let filenames: Vec<String> = md_files
            .iter()
            .filter_map(|p| p.file_name())
            .filter_map(|n| n.to_str())
            .map(|s| s.to_string())
            .collect();

        assert!(filenames.contains(&"regular.md".to_string()));
        assert!(filenames.contains(&"target.md".to_string()));
        assert!(filenames.contains(&"link.md".to_string()));
    }

    #[test]
    #[cfg(unix)]
    fn test_find_files_by_extension_with_broken_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join("regular.md"), "regular file").unwrap();
        symlink("nonexistent.md", temp_path.join("broken_link.md")).unwrap();

        let result = find_files_by_extension(temp_path, "md");
        assert!(result.is_err());
    }

    #[test]
    fn test_gitignore_filter_excludes_ignored_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join(".gitignore"), "target/\n").unwrap();
        fs::create_dir_all(temp_path.join("src")).unwrap();
        fs::create_dir_all(temp_path.join("target")).unwrap();
        fs::create_dir_all(temp_path.join("tests")).unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);
        let visited = traverse_and_collect_with_filter(temp_path, 1, &filter);

        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "tests"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "target"));
    }

    #[test]
    fn test_gitignore_filter_always_excludes_hidden() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // .gitignore that does NOT mention .git
        fs::write(temp_path.join(".gitignore"), "target/\n").unwrap();
        fs::create_dir_all(temp_path.join(".git")).unwrap();
        fs::create_dir_all(temp_path.join(".hidden")).unwrap();
        fs::create_dir_all(temp_path.join("src")).unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);
        let visited = traverse_and_collect_with_filter(temp_path, 1, &filter);

        assert!(!visited.iter().any(|p| p.file_name().unwrap() == ".git"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == ".hidden"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
    }

    #[test]
    fn test_gitignore_filter_always_excludes_ai_rules() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // .gitignore that does NOT mention ai-rules
        fs::write(temp_path.join(".gitignore"), "target/\n").unwrap();
        fs::create_dir_all(temp_path.join("ai-rules")).unwrap();
        fs::create_dir_all(temp_path.join("src")).unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);
        let visited = traverse_and_collect_with_filter(temp_path, 1, &filter);

        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "ai-rules"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
    }

    #[test]
    fn test_gitignore_filter_allows_non_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // .gitignore only ignores "dist/"
        fs::write(temp_path.join(".gitignore"), "dist/\n").unwrap();
        // "node_modules" is in the hardcoded list but NOT in .gitignore
        fs::create_dir_all(temp_path.join("node_modules")).unwrap();
        fs::create_dir_all(temp_path.join("vendor")).unwrap();
        fs::create_dir_all(temp_path.join("dist")).unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);
        let visited = traverse_and_collect_with_filter(temp_path, 1, &filter);

        // node_modules and vendor should be traversed (not in .gitignore)
        assert!(visited
            .iter()
            .any(|p| p.file_name().unwrap() == "node_modules"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "vendor"));
        // dist should be excluded (in .gitignore)
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "dist"));
    }

    #[test]
    fn test_fallback_to_hardcoded_when_no_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // No .gitignore file
        fs::create_dir_all(temp_path.join("src")).unwrap();
        fs::create_dir_all(temp_path.join("target")).unwrap();
        fs::create_dir_all(temp_path.join("node_modules")).unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);
        let visited = traverse_and_collect_with_filter(temp_path, 1, &filter);

        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "target"));
        assert!(!visited
            .iter()
            .any(|p| p.file_name().unwrap() == "node_modules"));
    }

    #[test]
    fn test_negation_pattern_build_with_important() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // build/ is ignored, but build/important/ is negated
        fs::write(temp_path.join(".gitignore"), "build/\n!build/important/\n").unwrap();
        fs::create_dir_all(temp_path.join("build/important")).unwrap();
        fs::create_dir_all(temp_path.join("build/other")).unwrap();
        fs::create_dir_all(temp_path.join("src")).unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);

        // build/ should be SkipCallbackButRecurse because it has a negated child
        assert_eq!(
            filter.traversal_decision(&temp_path.join("build"), "build", false),
            TraversalDecision::SkipCallbackButRecurse
        );

        // src/ should be Enter (not ignored)
        assert_eq!(
            filter.traversal_decision(&temp_path.join("src"), "src", false),
            TraversalDecision::Enter
        );

        // Full traversal: build/ should NOT appear in visited, but build/important/ SHOULD
        let visited = traverse_and_collect_with_filter(temp_path, 3, &filter);

        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
        assert!(
            visited
                .iter()
                .any(|p| p.file_name().unwrap() == "important"),
            "build/important/ should be visited due to negation pattern"
        );
        assert!(
            !visited.iter().any(|p| p == &temp_path.join("build")),
            "build/ itself should NOT be in visited (callback skipped)"
        );
        assert!(
            !visited.iter().any(|p| p.file_name().unwrap() == "other"),
            "build/other/ should not be visited (still ignored)"
        );
    }

    #[test]
    fn test_nested_gitignore_excludes_subdirectory_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Root .gitignore ignores nothing relevant
        fs::write(temp_path.join(".gitignore"), "target/\n").unwrap();

        // Create subdirectory with its own .gitignore
        fs::create_dir_all(temp_path.join("packages/frontend/dist")).unwrap();
        fs::create_dir_all(temp_path.join("packages/frontend/src")).unwrap();
        fs::write(temp_path.join("packages/frontend/.gitignore"), "dist/\n").unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);
        let visited = traverse_and_collect_with_filter(temp_path, 4, &filter);

        assert!(visited.iter().any(|p| p.file_name().unwrap() == "frontend"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
        assert!(
            !visited.iter().any(|p| p.file_name().unwrap() == "dist"),
            "dist/ should be excluded by nested .gitignore"
        );
    }

    #[test]
    fn test_nested_gitignore_with_negation() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Root .gitignore
        fs::write(temp_path.join(".gitignore"), "").unwrap();

        // Subdirectory with gitignore that has negation
        fs::create_dir_all(temp_path.join("app/generated/keep")).unwrap();
        fs::create_dir_all(temp_path.join("app/generated/throwaway")).unwrap();
        fs::create_dir_all(temp_path.join("app/src")).unwrap();
        fs::write(
            temp_path.join("app/.gitignore"),
            "generated/\n!generated/keep/\n",
        )
        .unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);
        let visited = traverse_and_collect_with_filter(temp_path, 4, &filter);

        assert!(visited.iter().any(|p| p.file_name().unwrap() == "app"));
        assert!(visited.iter().any(|p| p.file_name().unwrap() == "src"));
        assert!(
            visited.iter().any(|p| p.file_name().unwrap() == "keep"),
            "generated/keep/ should be visited due to negation in nested .gitignore"
        );
        assert!(
            !visited
                .iter()
                .any(|p| p == &temp_path.join("app/generated")),
            "generated/ itself should not be in visited (callback skipped)"
        );
        assert!(
            !visited
                .iter()
                .any(|p| p.file_name().unwrap() == "throwaway"),
            "generated/throwaway/ should not be visited (still ignored)"
        );
    }

    #[test]
    fn test_ignored_dir_without_negation_fully_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // build/ is ignored with NO negation patterns
        fs::write(temp_path.join(".gitignore"), "build/\n").unwrap();
        fs::create_dir_all(temp_path.join("build/sub")).unwrap();
        fs::create_dir_all(temp_path.join("src")).unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);

        // build/ should be fully skipped (no negation)
        assert_eq!(
            filter.traversal_decision(&temp_path.join("build"), "build", false),
            TraversalDecision::Skip
        );

        let visited = traverse_and_collect_with_filter(temp_path, 3, &filter);
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "build"));
        assert!(!visited.iter().any(|p| p.file_name().unwrap() == "sub"));
    }

    #[test]
    fn test_negation_at_max_depth_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // build/ ignored, build/important/ negated — important is at depth 2
        fs::write(temp_path.join(".gitignore"), "build/\n!build/important/\n").unwrap();
        fs::create_dir_all(temp_path.join("build/important")).unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);

        // max_depth=1: root(0) can recurse, but build/(1) is SkipCallbackButRecurse
        // at the depth boundary — no further recursion, so important/ is unreachable
        let visited = traverse_and_collect_with_filter(temp_path, 1, &filter);
        assert!(
            !visited
                .iter()
                .any(|p| p.file_name().unwrap() == "important"),
            "negated child beyond max_depth is not visited (depth limit takes precedence)"
        );

        // max_depth=2: build/(1) can recurse, important/(2) is Enter at the boundary
        let visited = traverse_and_collect_with_filter(temp_path, 2, &filter);
        assert!(
            visited
                .iter()
                .any(|p| p.file_name().unwrap() == "important"),
            "negated child within max_depth should be visited"
        );
    }
}

use crate::constants::{AGENT_SECTION_END, AGENT_SECTION_START, AGENTS_MD_FILENAME, AI_RULE_SOURCE_DIR, GENERATED_FILE_PREFIX};
use crate::operations::body_generator::inlined_agents_relative_path;
use anyhow::Result;

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

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

    if uses_section_merging(&link) {
        if link.is_symlink() {
            fs::remove_file(&link)?;
        }
        let inlined_content = fs::read_to_string(&source_full_path)?;
        let existing_content = if link.exists() {
            fs::read_to_string(&link)?
        } else {
            String::new()
        };
        let merged = merge_section_into_content(&existing_content, &inlined_content);
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&link, merged)?;
    } else {
        let relative_source = calculate_relative_path(output_path, &inlined_relative);
        create_relative_symlink(&link, &relative_source)?;
    }

    Ok(true)
}

pub fn check_inlined_file_symlink(current_dir: &Path, symlink_path: &Path) -> Result<bool> {
    let inlined_relative = inlined_agents_relative_path();
    let source_full_path = current_dir.join(&inlined_relative);

    if uses_section_merging(symlink_path) {
        if !source_full_path.exists() {
            if symlink_path.exists() {
                let content = fs::read_to_string(symlink_path)?;
                return Ok(extract_section_content(&content).is_none());
            }
            return Ok(true);
        }
        if !symlink_path.exists() {
            return Ok(false);
        }
        let expected = fs::read_to_string(&source_full_path)?;
        let actual = fs::read_to_string(symlink_path)?;
        return Ok(extract_section_content(&actual).as_deref() == Some(expected.as_str()));
    }

    if !symlink_path.is_symlink() {
        return Ok(false);
    }

    let expected_target = source_full_path;
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

pub fn uses_section_merging(file_path: &Path) -> bool {
    let is_md = file_path.extension().is_some_and(|ext| ext == "md");
    if !is_md {
        return false;
    }
    let path_str = file_path.to_string_lossy();
    !path_str.contains(GENERATED_FILE_PREFIX)
}

pub fn merge_section_into_content(existing_content: &str, section_content: &str) -> String {
    let wrapped = format!(
        "{}\n{}{}\n",
        AGENT_SECTION_START,
        ensure_trailing_newline(section_content),
        AGENT_SECTION_END
    );

    if let (Some(start), Some(end_start)) = (
        existing_content.find(AGENT_SECTION_START),
        existing_content.find(AGENT_SECTION_END),
    ) {
        if end_start <= start {
            return format!("{existing_content}\n{wrapped}");
        }
        let end = end_start + AGENT_SECTION_END.len();
        let end = if existing_content[end..].starts_with('\n') { end + 1 } else { end };
        format!("{}{}{}", &existing_content[..start], wrapped, &existing_content[end..])
    } else if existing_content.is_empty() {
        wrapped
    } else {
        let separator = if existing_content.ends_with("\n\n") {
            ""
        } else if existing_content.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        format!("{existing_content}{separator}{wrapped}")
    }
}

pub fn remove_section_from_content(content: &str) -> String {
    if let (Some(start), Some(end_start)) = (
        content.find(AGENT_SECTION_START),
        content.find(AGENT_SECTION_END),
    ) {
        if end_start <= start {
            return content.to_string();
        }
        let end = end_start + AGENT_SECTION_END.len();
        let end = if content[end..].starts_with('\n') { end + 1 } else { end };
        let effective_start = if content[..start].ends_with("\n\n") {
            start - 1
        } else {
            start
        };
        format!("{}{}", &content[..effective_start], &content[end..])
    } else {
        content.to_string()
    }
}

pub fn extract_section_content(content: &str) -> Option<String> {
    let start = content.find(AGENT_SECTION_START)?;
    let end_start = content.find(AGENT_SECTION_END)?;
    if end_start <= start {
        return None;
    }
    let inner_start = start + AGENT_SECTION_START.len();
    let inner_start = if content[inner_start..].starts_with('\n') {
        inner_start + 1
    } else {
        inner_start
    };
    Some(content[inner_start..end_start].to_string())
}

pub fn write_directory_files(files_to_write: &HashMap<PathBuf, String>) -> Result<()> {
    for (file_path, content) in files_to_write {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if uses_section_merging(file_path) {
            if file_path.is_symlink() {
                fs::remove_file(file_path)?;
            }
            let existing_content = if file_path.exists() {
                fs::read_to_string(file_path)?
            } else {
                String::new()
            };
            let merged = merge_section_into_content(&existing_content, content);
            fs::write(file_path, merged)?;
        } else {
            fs::write(file_path, content)?;
        }
    }

    Ok(())
}

pub fn traverse_project_directories<F>(
    current_dir: &Path,
    max_depth: usize,
    current_depth: usize,
    callback: &mut F,
) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    callback(current_dir)?;
    if current_depth >= max_depth {
        return Ok(());
    }

    // Collect and sort directories for deterministic traversal order
    let mut dirs = Vec::new();
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            if should_traverse_directory(dir_name) {
                dirs.push(path);
            }
        }
    }

    // Sort directories alphabetically for consistent order
    dirs.sort();

    for dir in dirs {
        traverse_project_directories(&dir, max_depth, current_depth + 1, callback)?;
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

const EXCLUDED_DIRECTORIES: &[&str] = &[
    "ai-rules",
    "target",
    "build",
    "dist",
    "out",
    "bin",
    "obj",
    "node_modules",
    "vendor",
    "packages",
    "__pycache__",
    ".pytest_cache",
    ".cache",
    ".vscode",
    ".idea",
    ".vs",
    "tmp",
    "temp",
    "logs",
];

fn should_traverse_directory(dir_name: &str) -> bool {
    !dir_name.starts_with('.') && !EXCLUDED_DIRECTORIES.contains(&dir_name)
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
    fn test_should_traverse_directory() {
        assert!(should_traverse_directory("src"));
        assert!(should_traverse_directory("utils"));
        assert!(!should_traverse_directory(".git"));
        assert!(!should_traverse_directory(".hidden"));
        assert!(!should_traverse_directory("target"));
        assert!(!should_traverse_directory("node_modules"));
        assert!(!should_traverse_directory("build"));
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

        traverse_project_directories(temp_path, 2, 0, &mut callback).unwrap();

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
        let mut visited = Vec::new();
        let mut callback = |path: &Path| -> Result<()> {
            visited.push(path.to_path_buf());
            Ok(())
        };
        traverse_project_directories(root_path, max_depth, 0, &mut callback).unwrap();
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
    fn test_uses_section_merging_true_for_md() {
        assert!(uses_section_merging(Path::new("CLAUDE.md")));
        assert!(uses_section_merging(Path::new("AGENTS.md")));
        assert!(uses_section_merging(Path::new("GEMINI.md")));
    }

    #[test]
    fn test_uses_section_merging_false_for_generated_md() {
        assert!(!uses_section_merging(Path::new(
            "ai-rules/.generated-ai-rules/ai-rules-generated-example.md"
        )));
        assert!(!uses_section_merging(Path::new(
            ".claude/skills/ai-rules-generated-foo/SKILL.md"
        )));
    }

    #[test]
    fn test_uses_section_merging_false_for_non_md() {
        assert!(!uses_section_merging(Path::new(".mcp.json")));
        assert!(!uses_section_merging(Path::new(".cursor/rules/rule.mdc")));
        assert!(!uses_section_merging(Path::new("firebender.json")));
    }

    #[test]
    fn test_merge_section_into_empty_content() {
        let result = merge_section_into_content("", "my content\n");
        assert_eq!(
            result,
            "<!-- ai-rules generated start -->\nmy content\n<!-- ai-rules generated end -->\n"
        );
    }

    #[test]
    fn test_merge_section_appends_to_existing_content() {
        let result = merge_section_into_content("# My Rules\n", "ai content\n");
        assert!(result.starts_with("# My Rules\n"));
        assert!(result.contains("<!-- ai-rules generated start -->\nai content\n<!-- ai-rules generated end -->\n"));
    }

    #[test]
    fn test_merge_section_replaces_existing_section() {
        let existing = "<!-- ai-rules generated start -->\nold content\n<!-- ai-rules generated end -->\n";
        let result = merge_section_into_content(existing, "new content\n");
        assert_eq!(
            result,
            "<!-- ai-rules generated start -->\nnew content\n<!-- ai-rules generated end -->\n"
        );
    }

    #[test]
    fn test_merge_section_replaces_section_preserving_surrounding_content() {
        let existing = "# Header\n\n<!-- ai-rules generated start -->\nold\n<!-- ai-rules generated end -->\n\n# Footer\n";
        let result = merge_section_into_content(existing, "new\n");
        assert!(result.contains("# Header\n"));
        assert!(result.contains("<!-- ai-rules generated start -->\nnew\n<!-- ai-rules generated end -->\n"));
        assert!(result.contains("# Footer\n"));
        assert!(!result.contains("old"));
    }

    #[test]
    fn test_merge_section_inverted_markers_appends() {
        let existing = "<!-- ai-rules generated end -->\nsome\n<!-- ai-rules generated start -->\n";
        let result = merge_section_into_content(existing, "new\n");
        assert!(result.contains(existing));
        assert!(result.contains("<!-- ai-rules generated start -->\nnew\n<!-- ai-rules generated end -->\n"));
    }

    #[test]
    fn test_remove_section_from_content_removes_section() {
        let content = "<!-- ai-rules generated start -->\ncontent\n<!-- ai-rules generated end -->\n";
        assert_eq!(remove_section_from_content(content), "");
    }

    #[test]
    fn test_remove_section_preserves_surrounding_content() {
        let content = "# Header\n\n<!-- ai-rules generated start -->\ncontent\n<!-- ai-rules generated end -->\n";
        let result = remove_section_from_content(content);
        assert_eq!(result, "# Header\n");
        assert!(!result.contains("ai-rules generated"));
    }

    #[test]
    fn test_remove_section_no_markers_returns_unchanged() {
        let content = "# Just user content\n";
        assert_eq!(remove_section_from_content(content), content);
    }

    #[test]
    fn test_remove_section_inverted_markers_returns_unchanged() {
        let content = "<!-- ai-rules generated end -->\nsome\n<!-- ai-rules generated start -->\n";
        assert_eq!(remove_section_from_content(content), content);
    }

    #[test]
    fn test_extract_section_content_returns_inner() {
        let content = "<!-- ai-rules generated start -->\ninner content\n<!-- ai-rules generated end -->\n";
        assert_eq!(
            extract_section_content(content),
            Some("inner content\n".to_string())
        );
    }

    #[test]
    fn test_extract_section_content_none_when_no_markers() {
        assert_eq!(extract_section_content("no markers here"), None);
    }

    #[test]
    fn test_extract_section_content_none_when_inverted() {
        let content = "<!-- ai-rules generated end -->\n<!-- ai-rules generated start -->\n";
        assert_eq!(extract_section_content(content), None);
    }

    #[test]
    fn test_extract_section_ignores_surrounding_content() {
        let content = "# Header\n\n<!-- ai-rules generated start -->\ninner\n<!-- ai-rules generated end -->\n\n# Footer\n";
        assert_eq!(
            extract_section_content(content),
            Some("inner\n".to_string())
        );
    }
}

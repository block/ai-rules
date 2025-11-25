use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_git_root(current_dir: &Path) -> Option<PathBuf> {
    let mut dir = current_dir;

    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

fn check_gitignore_in_dir(dir: &Path, patterns: &[String]) -> Option<PathBuf> {
    let gitignore_path = dir.join(".gitignore");
    if gitignore_path.exists() {
        if let Ok(content) = fs::read_to_string(&gitignore_path) {
            let has_pattern = content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .any(|line| patterns.contains(&line.to_string()));

            if has_pattern {
                return Some(gitignore_path);
            }
        }
    }
    None
}

pub fn check_gitignore_patterns_to_root(
    current_dir: &Path,
    patterns: &[String],
) -> Result<Vec<PathBuf>> {
    let mut found_ignores = Vec::new();

    let git_root = match find_git_root(current_dir) {
        Some(root) => root,
        None => return Ok(found_ignores),
    };

    let mut dir = current_dir;

    if let Some(gitignore_path) = check_gitignore_in_dir(dir, patterns) {
        found_ignores.push(gitignore_path);
    }

    while let Some(parent) = dir.parent() {
        if let Some(gitignore_path) = check_gitignore_in_dir(parent, patterns) {
            found_ignores.push(gitignore_path);
        }

        if parent == git_root {
            break;
        }
        dir = parent;
    }

    Ok(found_ignores)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_git_root_current_dir() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .git directory in current directory
        fs::create_dir_all(temp_path.join(".git")).unwrap();

        let result = find_git_root(temp_path);
        assert_eq!(result, Some(temp_path.to_path_buf()));
    }

    #[test]
    fn test_find_git_root_parent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(".git")).unwrap();

        let nested_path = temp_path.join("src/nested");
        fs::create_dir_all(&nested_path).unwrap();

        let result = find_git_root(&nested_path);
        assert_eq!(result, Some(temp_path.to_path_buf()));
    }

    #[test]
    fn test_find_git_root_no_git() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let result = find_git_root(temp_path);
        assert_eq!(result, None);
    }

    #[test]
    fn test_check_gitignore_in_dir_exact_match() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join(".gitignore"), "*.tmp\n**/.CLAUDE\n*.log\n").unwrap();

        let patterns = vec!["**/.CLAUDE".to_string(), "*.CLAUDE".to_string()];
        let result = check_gitignore_in_dir(temp_path, &patterns);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_path.join(".gitignore"));
    }

    #[test]
    fn test_check_gitignore_in_dir_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(
            temp_path.join(".gitignore"),
            "*.tmp\n*.log\nnode_modules/\n",
        )
        .unwrap();

        let patterns = vec!["**/.CLAUDE".to_string(), "*.CLAUDE".to_string()];
        let result = check_gitignore_in_dir(temp_path, &patterns);

        assert!(result.is_none());
    }

    #[test]
    fn test_check_gitignore_in_dir_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let patterns = vec!["**/.CLAUDE".to_string()];
        let result = check_gitignore_in_dir(temp_path, &patterns);

        assert!(result.is_none());
    }

    #[test]
    fn test_check_gitignore_patterns_to_root() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(".git")).unwrap();

        let nested_path = temp_path.join("src/nested");
        fs::create_dir_all(&nested_path).unwrap();

        fs::write(temp_path.join(".gitignore"), "*.tmp\n**/.CLAUDE\n*.log\n").unwrap();

        fs::write(temp_path.join("src/.gitignore"), "*.tmp\n*.cache\n").unwrap();

        let patterns = vec!["**/.CLAUDE".to_string(), "*.CLAUDE".to_string()];
        let result = check_gitignore_patterns_to_root(&nested_path, &patterns).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], temp_path.join(".gitignore"));
    }

    #[test]
    fn test_check_gitignore_patterns_to_root_no_git() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let patterns = vec!["**/.CLAUDE".to_string()];
        let result = check_gitignore_patterns_to_root(temp_path, &patterns).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_check_gitignore_patterns_multiple_matches() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join(".git")).unwrap();

        let nested_path = temp_path.join("src/nested");
        fs::create_dir_all(&nested_path).unwrap();

        fs::write(temp_path.join(".gitignore"), "*.CLAUDE\n").unwrap();

        fs::write(temp_path.join("src/.gitignore"), "**/.CLAUDE\n").unwrap();

        let patterns = vec!["**/.CLAUDE".to_string(), "*.CLAUDE".to_string()];
        let result = check_gitignore_patterns_to_root(&nested_path, &patterns).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&temp_path.join(".gitignore")));
        assert!(result.contains(&temp_path.join("src/.gitignore")));
    }
}

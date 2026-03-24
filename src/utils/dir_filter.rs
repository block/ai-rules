use crate::constants::AI_RULE_SOURCE_DIR;
use crate::utils::git_utils::find_git_root;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub(crate) enum TraversalDecision {
    /// Traverse the directory and invoke callback
    Enter,
    /// Don't invoke callback but recurse into children (parent ignored, children may be whitelisted)
    SkipCallbackButRecurse,
    /// Don't enter at all
    Skip,
}

pub enum DirectoryFilter {
    Gitignore {
        /// Chain of gitignore matchers, from root to most nested.
        /// Each is built with its own directory as root for correct pattern matching.
        gitignore_chain: Vec<Gitignore>,
        /// Project root for relative path computation
        root: PathBuf,
        /// Negation patterns: (base_dir, pattern) tuples extracted from all loaded files
        negation_patterns: Vec<(PathBuf, String)>,
    },
    Hardcoded,
}

impl DirectoryFilter {
    pub fn from_project_root(root: &Path) -> Self {
        // Try .gitignore at the given root first
        if let Some(filter) = Self::try_load_gitignore(root) {
            return filter;
        }

        // Try .gitignore at the git root
        if let Some(git_root) = find_git_root(root) {
            if git_root != root {
                if let Some(filter) = Self::try_load_gitignore(&git_root) {
                    return filter;
                }
            }
        }

        DirectoryFilter::Hardcoded
    }

    fn try_load_gitignore(root: &Path) -> Option<DirectoryFilter> {
        let (gitignore, negation_patterns) = build_gitignore_for_dir(root)?;
        Some(DirectoryFilter::Gitignore {
            gitignore_chain: vec![gitignore],
            root: root.to_path_buf(),
            negation_patterns,
        })
    }

    /// Check all gitignore matchers in the chain. Last non-None match wins
    /// (nested .gitignore files take precedence over parent ones).
    fn matched_in_chain(&self, dir_path: &Path) -> ignore::Match<()> {
        match self {
            DirectoryFilter::Gitignore {
                gitignore_chain, ..
            } => {
                let mut result = ignore::Match::None;
                for gi in gitignore_chain {
                    let m = gi.matched(dir_path, true);
                    if !m.is_none() {
                        // Map the match to strip the Glob reference
                        result = if m.is_ignore() {
                            ignore::Match::Ignore(())
                        } else {
                            ignore::Match::Whitelist(())
                        };
                    }
                }
                result
            }
            DirectoryFilter::Hardcoded => ignore::Match::None,
        }
    }

    pub(crate) fn traversal_decision(
        &self,
        dir_path: &Path,
        dir_name: &str,
        parent_ignored: bool,
    ) -> TraversalDecision {
        // Always exclude hidden directories and ai-rules directory
        if dir_name.starts_with('.') || dir_name == AI_RULE_SOURCE_DIR {
            return TraversalDecision::Skip;
        }

        match self {
            DirectoryFilter::Gitignore {
                negation_patterns, ..
            } => {
                let matched = self.matched_in_chain(dir_path);
                if matched.is_whitelist() {
                    TraversalDecision::Enter
                } else if matched.is_ignore() || parent_ignored {
                    // Ignored directly by a pattern, or inherited from an ignored parent
                    if has_negated_children(dir_path, negation_patterns) {
                        TraversalDecision::SkipCallbackButRecurse
                    } else {
                        TraversalDecision::Skip
                    }
                } else {
                    TraversalDecision::Enter
                }
            }
            DirectoryFilter::Hardcoded => {
                if should_traverse_directory(dir_name) {
                    TraversalDecision::Enter
                } else {
                    TraversalDecision::Skip
                }
            }
        }
    }

    /// If `dir` contains a `.gitignore`, return a new filter with it layered in.
    /// Returns `None` if no child `.gitignore` exists (caller should reuse `self`).
    pub(crate) fn with_child_gitignore(&self, dir: &Path) -> Option<DirectoryFilter> {
        match self {
            DirectoryFilter::Gitignore {
                gitignore_chain,
                root,
                negation_patterns,
            } => {
                let (child_gi, child_negations) = build_gitignore_for_dir(dir)?;

                let mut new_chain = gitignore_chain.clone();
                new_chain.push(child_gi);

                let mut new_negations = negation_patterns.clone();
                new_negations.extend(child_negations);

                Some(DirectoryFilter::Gitignore {
                    gitignore_chain: new_chain,
                    root: root.clone(),
                    negation_patterns: new_negations,
                })
            }
            DirectoryFilter::Hardcoded => None,
        }
    }
}

/// Read a `.gitignore` file once, building both a compiled `Gitignore` matcher
/// and a list of negation patterns. Returns `None` if the file doesn't exist or can't be parsed.
fn build_gitignore_for_dir(dir: &Path) -> Option<(Gitignore, Vec<(PathBuf, String)>)> {
    let gitignore_path = dir.join(".gitignore");
    let content = fs::read_to_string(&gitignore_path).ok()?;

    let mut builder = GitignoreBuilder::new(dir);
    let mut negation_patterns = Vec::new();

    for line in content.lines() {
        // Feed every line to the builder (it handles comments, blanks, etc.)
        if builder.add_line(Some(dir.to_path_buf()), line).is_err() {
            continue;
        }
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix('!') {
            if !rest.starts_with('#') && !rest.starts_with('!') {
                let pattern = rest.trim_start_matches('/');
                negation_patterns.push((dir.to_path_buf(), pattern.to_string()));
            }
        }
    }

    let gitignore = builder.build().ok()?;
    Some((gitignore, negation_patterns))
}

/// Check whether any literal negation pattern could match a child of the given directory.
/// Patterns containing glob metacharacters (*, ?, [) are not evaluated here;
/// they are handled by the compiled `Gitignore` matcher instead.
fn has_negated_children(dir_path: &Path, negation_patterns: &[(PathBuf, String)]) -> bool {
    negation_patterns.iter().any(|(base_dir, pattern)| {
        let negated_path = base_dir.join(pattern.trim_end_matches('/'));
        // Path::starts_with is component-aware, so "foobar" won't match under "foo"
        negated_path.starts_with(dir_path)
    })
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
    use tempfile::TempDir;

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
    fn test_traversal_decision_hidden_dirs_skip() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::write(temp_path.join(".gitignore"), "target/\n").unwrap();
        let filter = DirectoryFilter::from_project_root(temp_path);

        assert_eq!(
            filter.traversal_decision(&temp_path.join(".git"), ".git", false),
            TraversalDecision::Skip
        );
        assert_eq!(
            filter.traversal_decision(&temp_path.join("ai-rules"), "ai-rules", false),
            TraversalDecision::Skip
        );
        assert_eq!(
            filter.traversal_decision(&temp_path.join("src"), "src", false),
            TraversalDecision::Enter
        );
    }

    #[test]
    fn test_negation_pattern_traversal_decision() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // build/ is ignored, but build/important/ is negated
        fs::write(temp_path.join(".gitignore"), "build/\n!build/important/\n").unwrap();
        fs::create_dir_all(temp_path.join("build/important")).unwrap();

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
    }

    #[test]
    fn test_ignored_dir_without_negation_is_skip() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // build/ is ignored with NO negation patterns
        fs::write(temp_path.join(".gitignore"), "build/\n").unwrap();

        let filter = DirectoryFilter::from_project_root(temp_path);

        assert_eq!(
            filter.traversal_decision(&temp_path.join("build"), "build", false),
            TraversalDecision::Skip
        );
    }

    #[test]
    fn test_fallback_to_hardcoded_when_no_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // No .gitignore file
        let filter = DirectoryFilter::from_project_root(temp_path);
        assert!(matches!(filter, DirectoryFilter::Hardcoded));
    }
}

use crate::operations::source_reader::get_packages_dir;
use crate::utils::print_utils::print_success;
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

pub fn run_uninstall(current_dir: &Path, package_name: &str) -> Result<()> {
    // Validate package name to prevent path traversal
    if package_name.contains('/')
        || package_name.contains('\\')
        || package_name == ".."
        || package_name == "."
    {
        bail!(
            "Invalid package name '{}': must not contain path separators or dot-components",
            package_name
        );
    }

    let packages_dir = get_packages_dir(current_dir);
    let target_pkg_dir = packages_dir.join(package_name);

    if !target_pkg_dir.exists() && !target_pkg_dir.is_symlink() {
        bail!("Package '{}' is not installed.", package_name);
    }

    if target_pkg_dir.is_symlink() {
        fs::remove_file(&target_pkg_dir)?;
    } else {
        fs::remove_dir_all(&target_pkg_dir)?;
    }

    print_success(&format!(
        "Package '{}' uninstalled. Run 'ai-rules generate' to update generated files.",
        package_name
    ));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_uninstall_removes_package() {
        let temp_dir = TempDir::new().unwrap();

        // Create installed package
        create_file(
            temp_dir.path(),
            "ai-rules/packages/my-pkg/rule1.md",
            "---\ndescription: Rule\nalwaysApply: true\n---\nContent",
        );

        let result = run_uninstall(temp_dir.path(), "my-pkg");
        assert!(result.is_ok());

        assert_file_not_exists(temp_dir.path(), "ai-rules/packages/my-pkg");
    }

    #[test]
    fn test_uninstall_removes_symlinked_package() {
        let temp_dir = TempDir::new().unwrap();

        // Create a symlink target
        let source = temp_dir.path().join("source-pkg");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("rule1.md"), "content").unwrap();

        // Create the packages dir and symlink
        let packages_dir = temp_dir.path().join("ai-rules/packages");
        fs::create_dir_all(&packages_dir).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, packages_dir.join("my-pkg")).unwrap();

        let result = run_uninstall(temp_dir.path(), "my-pkg");
        assert!(result.is_ok());

        assert_file_not_exists(temp_dir.path(), "ai-rules/packages/my-pkg");
        // Source should still exist
        assert!(source.exists());
    }

    #[test]
    fn test_uninstall_rejects_path_traversal() {
        let temp_dir = TempDir::new().unwrap();

        for bad_name in &["../outside", "foo/bar", "..\\outside", "..", "."] {
            let result = run_uninstall(temp_dir.path(), bad_name);
            assert!(
                result.is_err(),
                "Expected error for name '{}' but got Ok",
                bad_name
            );
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Invalid package name"),
                "Expected 'Invalid package name' error for '{}', got: {}",
                bad_name,
                err
            );
        }
    }

    #[test]
    fn test_uninstall_not_installed() {
        let temp_dir = TempDir::new().unwrap();

        let result = run_uninstall(temp_dir.path(), "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not installed"));
    }
}

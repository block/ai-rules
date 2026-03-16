use crate::agents::AgentToolRegistry;
use crate::cli::InstallArgs;
use crate::constants::AI_RULE_SOURCE_DIR;
use crate::operations::source_reader::get_packages_dir;
use crate::operations::GenerationResult;
use crate::utils::print_utils::print_success;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use super::generate::generate_files;

pub fn run_install(current_dir: &Path, args: InstallArgs) -> Result<()> {
    let source_path = fs::canonicalize(&args.path)
        .with_context(|| format!("Source path not found: {}", args.path))?;

    // Find ai-rules/ content in the source
    let source_ai_rules_dir = source_path.join(AI_RULE_SOURCE_DIR);
    if !source_ai_rules_dir.exists() || !source_ai_rules_dir.is_dir() {
        bail!(
            "No ai-rules/ directory found in '{}'. The source must contain an ai-rules/ directory.",
            source_path.display()
        );
    }

    // Determine package name
    let package_name = args.name.unwrap_or_else(|| {
        source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    });

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
    let target_pkg_dir = packages_dir.join(&package_name);

    // Check if already installed
    if target_pkg_dir.exists() || target_pkg_dir.is_symlink() {
        if args.force {
            if target_pkg_dir.is_symlink() {
                fs::remove_file(&target_pkg_dir)?;
            } else {
                fs::remove_dir_all(&target_pkg_dir)?;
            }
        } else {
            bail!(
                "Package '{}' is already installed. Use --force to reinstall.",
                package_name
            );
        }
    }

    // Create packages directory
    fs::create_dir_all(&packages_dir).with_context(|| {
        format!(
            "Failed to create packages directory: {}",
            packages_dir.display()
        )
    })?;

    // Install: symlink or copy
    if args.link {
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source_ai_rules_dir, &target_pkg_dir).with_context(|| {
            format!(
                "Failed to create symlink from {} to {}",
                target_pkg_dir.display(),
                source_ai_rules_dir.display()
            )
        })?;

        #[cfg(not(unix))]
        bail!("--link is only supported on Unix systems");

        println!(
            "Linked package '{}' -> {}",
            package_name,
            source_ai_rules_dir.display()
        );
    } else {
        copy_dir_recursive(&source_ai_rules_dir, &target_pkg_dir)?;
        println!(
            "Installed package '{}' from {}",
            package_name,
            source_path.display()
        );
    }

    // Determine which agents to generate for
    let use_claude_skills = false; // Install doesn't use claude skills mode
    let registry = AgentToolRegistry::new(use_claude_skills);

    let agents = if let Some(specified) = args.agents {
        specified
    } else {
        let detected = registry.detect_agents(current_dir);
        if detected.is_empty() {
            println!("No agents detected. Use --agents to specify which agents to install for.");
            return Ok(());
        }
        println!("Detected agents: {}", detected.join(", "));
        detected
    };

    // Run generate in no-clobber mode
    let mut generation_result = GenerationResult::default();
    generate_files(
        current_dir,
        &agents,
        &agents,
        &registry,
        &mut generation_result,
        true,
    )?;

    generation_result.display(current_dir);
    print_success(&format!(
        "Package '{}' installed successfully",
        package_name
    ));

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in
        fs::read_dir(src).with_context(|| format!("Failed to read directory: {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "Failed to copy {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::helpers::*;
    use tempfile::TempDir;

    fn create_package_source(temp_dir: &Path, name: &str) -> std::path::PathBuf {
        let pkg_dir = temp_dir.join(name);
        let ai_rules_dir = pkg_dir.join("ai-rules");
        std::fs::create_dir_all(&ai_rules_dir).unwrap();
        pkg_dir
    }

    #[test]
    fn test_install_copies_package() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        // Create source package
        let source = create_package_source(temp_dir.path(), "my-package");
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Test rule\nalwaysApply: true\n---\nRule content",
        );

        // Create a .claude dir so claude is detected
        fs::create_dir_all(target.join(".claude")).unwrap();

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: false,
            force: false,
        };

        let result = run_install(&target, args);
        assert!(result.is_ok(), "Install failed: {:?}", result.err());

        // Package files should be copied
        assert_file_exists(&target, "ai-rules/packages/my-package/rule1.md");

        // Generated files should exist
        assert_file_exists(
            &target,
            "ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md",
        );
    }

    #[test]
    fn test_install_with_link() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        // Create source package
        let source = create_package_source(temp_dir.path(), "linked-pkg");
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Linked rule\nalwaysApply: true\n---\nLinked content",
        );

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: true,
            force: false,
        };

        let result = run_install(&target, args);
        assert!(result.is_ok(), "Install failed: {:?}", result.err());

        // Package dir should be a symlink
        let pkg_dir = target.join("ai-rules/packages/linked-pkg");
        assert!(pkg_dir.is_symlink(), "Package dir should be a symlink");
    }

    #[test]
    fn test_install_no_clobber_preserves_existing_claude_md() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        // Create a hand-written CLAUDE.md
        create_file(&target, "CLAUDE.md", "My hand-written rules");

        // Create source package
        let source = create_package_source(temp_dir.path(), "my-pkg");
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Pkg rule\nalwaysApply: true\n---\nPackage content",
        );

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: false,
            force: false,
        };

        let result = run_install(&target, args);
        assert!(result.is_ok());

        // CLAUDE.md should preserve hand-written content AND have reference appended
        let content = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
        assert!(
            content.contains("My hand-written rules"),
            "Should preserve original content"
        );
        assert!(
            content.contains("@ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md"),
            "Should append reference"
        );

        // Body files should be generated
        assert_file_exists(
            &target,
            "ai-rules/.generated-ai-rules/ai-rules-generated-rule1.md",
        );
    }

    #[test]
    fn test_install_already_installed_without_force() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let source = create_package_source(temp_dir.path(), "my-pkg");
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Rule\nalwaysApply: true\n---\nContent",
        );

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: false,
            force: false,
        };

        // First install succeeds
        run_install(&target, args.clone()).unwrap();

        // Second install fails without --force
        let result = run_install(&target, args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already installed"));
    }

    #[test]
    fn test_install_force_reinstall() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let source = create_package_source(temp_dir.path(), "my-pkg");
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Rule v1\nalwaysApply: true\n---\nVersion 1",
        );

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: false,
            force: false,
        };
        run_install(&target, args).unwrap();

        // Update source
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Rule v2\nalwaysApply: true\n---\nVersion 2",
        );

        let args_force = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: false,
            force: true,
        };
        run_install(&target, args_force).unwrap();

        // Should have updated content
        let content = fs::read_to_string(target.join("ai-rules/packages/my-pkg/rule1.md")).unwrap();
        assert!(content.contains("Version 2"));
    }

    #[test]
    fn test_install_no_ai_rules_dir_in_source() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        let source = temp_dir.path().join("empty-source");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(&source).unwrap();

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: false,
            force: false,
        };

        let result = run_install(&target, args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No ai-rules/ directory"));
    }

    #[test]
    fn test_install_custom_name() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let source = create_package_source(temp_dir.path(), "original-name");
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Rule\nalwaysApply: true\n---\nContent",
        );

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: Some("custom-name".to_string()),
            link: false,
            force: false,
        };

        run_install(&target, args).unwrap();

        assert_file_exists(&target, "ai-rules/packages/custom-name/rule1.md");
        assert_file_not_exists(&target, "ai-rules/packages/original-name/rule1.md");
    }

    #[test]
    fn test_install_with_skills() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let source = create_package_source(temp_dir.path(), "skills-pkg");
        create_file(
            &source,
            "ai-rules/skills/my-skill/SKILL.md",
            "---\nname: my-skill\ndescription: A skill\n---\nSkill content",
        );

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: false,
            force: false,
        };

        let result = run_install(&target, args);
        assert!(result.is_ok(), "Install failed: {:?}", result.err());

        // Skill should be installed
        assert_file_exists(
            &target,
            "ai-rules/packages/skills-pkg/skills/my-skill/SKILL.md",
        );

        // Skill symlink should be created
        let symlink_path = target.join(".claude/skills/ai-rules-generated-my-skill");
        assert!(symlink_path.exists(), "Skill symlink should exist");
        assert!(symlink_path.is_symlink(), "Should be a symlink");
    }

    #[test]
    fn test_install_with_commands() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let source = create_package_source(temp_dir.path(), "cmds-pkg");
        create_file(
            &source,
            "ai-rules/commands/review.md",
            "---\ndescription: Review command\n---\nReview instructions",
        );

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: Some(vec!["claude".to_string()]),
            name: None,
            link: false,
            force: false,
        };

        let result = run_install(&target, args);
        assert!(result.is_ok(), "Install failed: {:?}", result.err());

        // Command symlink should be created
        let symlink_path = target.join(".claude/commands/ai-rules/review.md");
        assert!(symlink_path.is_symlink(), "Command symlink should exist");
    }

    #[test]
    fn test_install_rejects_path_traversal_in_name() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let source = create_package_source(temp_dir.path(), "my-pkg");
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Rule\nalwaysApply: true\n---\nContent",
        );

        for bad_name in &["../outside", "foo/bar", "..\\outside", "..", "."] {
            let args = InstallArgs {
                path: source.to_string_lossy().to_string(),
                agents: Some(vec!["claude".to_string()]),
                name: Some(bad_name.to_string()),
                link: false,
                force: false,
            };

            let result = run_install(&target, args);
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
    fn test_install_auto_detect_agents() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&target).unwrap();

        // Create .claude dir so claude is detected
        fs::create_dir_all(target.join(".claude")).unwrap();
        // Create .cursor dir so cursor is detected
        fs::create_dir_all(target.join(".cursor")).unwrap();

        let source = create_package_source(temp_dir.path(), "my-pkg");
        create_file(
            &source,
            "ai-rules/rule1.md",
            "---\ndescription: Test rule\nalwaysApply: true\n---\nRule content",
        );

        let args = InstallArgs {
            path: source.to_string_lossy().to_string(),
            agents: None, // auto-detect
            name: None,
            link: false,
            force: false,
        };

        let result = run_install(&target, args);
        assert!(result.is_ok(), "Install failed: {:?}", result.err());

        // Cursor rules should be generated (detected)
        assert_file_exists(&target, ".cursor/rules/ai-rules-generated-rule1.mdc");
    }
}

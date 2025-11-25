---
description: "Base repository guidelines"
alwaysApply: true
---

# Repository Guidelines

This is a Rust-based CLI tool called `ai-rules` that manages AI rules across different AI coding agents. The project uses Cargo for dependency management and Hermit for environment tooling.

## Project Structure & Module Organization

The codebase follows a modular Rust structure:

- `src/main.rs` - Entry point that delegates to CLI runner
- `src/cli/` - Command-line argument parsing with clap
- `src/commands/` - Command implementations (init, generate, list_agents, status, clean)
- `src/operations/` - Core business logic (body generation, gitignore updates, source reading)
- `src/agents/` - Agent-specific configurations and handlers
- `src/models/` - Data structures and types
- `src/utils/` - Utility functions (file, git, goose, print, prompt)
- `src/templates/` - Embedded template files for initialization
- `scripts/` - Shell scripts for CI tasks (clippy-check.sh, clippy-fix.sh, release.sh)
- `target/` - Build artifacts (gitignored)

## Build, Test, and Development Commands

All commands require Hermit environment activation first:

```bash
source bin/activate-hermit
```

**Build:**
- `cargo build` - Debug build
- `cargo build --release` - Optimized release build

**Test:**
- `cargo test` - Run all tests
- `cargo test --verbose` - Run tests with detailed output
- `cargo test <test_name>` - Run specific test

**Linting & Formatting:**
- `cargo fmt` - Format code
- `cargo fmt --check` - Check formatting without modifying
- `cargo clippy` - Run lints
- `./scripts/clippy-check.sh` - Run clippy with strict warnings (used in CI)
- `./scripts/clippy-fix.sh` - Auto-fix clippy issues

**Security:**
- `cargo audit` - Check for security vulnerabilities in dependencies

**Run the tool:**
- `cargo run -- <args>` - Run with arguments (e.g., `cargo run -- init`)

## Coding Style & Naming Conventions

**Indentation:** 4 spaces (standard Rust convention)

**Naming:**
- `snake_case` for functions, variables, modules, and files
- `PascalCase` for types, structs, enums, and traits
- `SCREAMING_SNAKE_CASE` for constants

**Code Organization:**
- Each module has a `mod.rs` that exports public items
- Tests are included in the same file using `#[cfg(test)]` modules
- Use `anyhow::Result` for error handling with context
- Prefer explicit error messages with `.with_context()`

**Dependencies:**
- Keep dependencies minimal (currently: anyhow, clap, serde, serde_json, serde_yaml, colored, cliclack, regex, which)
- Use feature flags to minimize compilation (e.g., `clap` with `derive` feature)

## Testing Guidelines

**Framework:** Built-in Rust testing with `tempfile` for temporary directories

**Test Organization:**
- Unit tests live in `#[cfg(test)]` modules at the bottom of each source file
- Test functions use `#[test]` attribute
- Use descriptive test names: `test_<function>_<scenario>` (e.g., `test_load_config_no_file`)

**Running Tests:**
- `cargo test` - All tests
- `cargo test <module>` - Tests in specific module
- `cargo test <test_name>` - Specific test

**Test Patterns:**
- Use `tempfile::TempDir` for filesystem tests
- Use `unwrap()` in tests (it's acceptable to panic on test failures)
- Test both success and error cases
- Include edge cases (empty inputs, invalid data, missing files)

**Coverage:** No explicit coverage requirement, but aim for comprehensive test coverage of core logic

## Commit & Pull Request Guidelines

**Commit Message Format:**
Based on recent history, commits follow a descriptive style:
- Use imperative mood: "Add feature" not "Added feature"
- Include PR numbers when merging: "removed internal info (#5)"
- Be specific and descriptive
- Examples from history:
  - "added ci pipeline (#6)"
  - "only run goose when it is installed (#4)"
  - "Invert gitignore default behavior (#3)"

**PR Requirements:**
- Link to related issues/PRs in commit messages
- Include PR number in merge commits
- Ensure CI passes (format check, clippy, tests, audit, release build)
- Keep changes focused and atomic

**Branch Strategy:**
- Main branch: `main`
- CI runs on pushes to `main` and all pull requests

## CI/CD Pipeline

The project uses GitHub Actions with the following checks:

1. **Format Check** - `cargo fmt --check`
2. **Clippy Lints** - `./scripts/clippy-check.sh` (strict mode with `-D warnings`)
3. **Tests** - `cargo test --verbose`
4. **Security Audit** - `cargo audit` (continue-on-error: true)
5. **Test Release Build** - `cargo build --release --verbose`

All jobs run on `ubuntu-latest` and require Hermit activation via `source bin/activate-hermit`

## Hermit Environment

This project uses Hermit for reproducible tooling. The `bin/` directory contains Hermit-managed tools.

**Activation:**
```bash
source bin/activate-hermit
```

This ensures consistent versions of Rust, Cargo, and other tools across all environments.

## Configuration

The tool supports configuration via `ai-rules/ai-rules-config.yaml`:
- `agents` - List of agent names to generate for
- `gitignore` - Whether to update .gitignore
- `nested_depth` - Directory traversal depth
- `use_claude_skills` - Enable Claude-specific features

## Key Constants

Defined in `src/constants.rs`:
- `AI_RULE_SOURCE_DIR` - "ai-rules/"
- `AI_RULE_CONFIG_FILENAME` - "ai-rules-config.yaml"
- `GENERATED_RULE_BODY_DIR` - ".generated-ai-rules/"
- `OPTIONAL_RULES_FILENAME` - "ai-rules-generated-optional.md"

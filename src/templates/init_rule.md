You are initializing this repository for AI-assisted development.

🎯 Goal
Generate a rule file - a guide that helps developers and AI agents understand how to work with this codebase effectively.

**Requirements:**
- Include YAML frontmatter with description and alwaysApply flag
- Content title: "Repository Guidelines" (H1 heading)
- Content length: 200–400 words
- Adapt sections as needed: add if relevant, omit if not applicable

📤 Output format (strict)
The file must start with YAML frontmatter, followed by the markdown content:

---
description: "Base respository guidelines"
alwaysApply: true
---

# Repository Guidelines

[Your content here...]

No extra commentary before or after the file content.

🔎 Project Discovery Process (scan systematically)

Step 1: **Explore structure and documentation**
  - List root directory contents
  - Identify major subdirectories (src/, tests/, apps/, packages/, etc.)
  - Read README.md and CONTRIBUTING.md if present (full contents)

Step 2: **Read build/dependency files completely**
  - Find: package.json, Cargo.toml, go.mod, pyproject.toml, Makefile, 
    Justfile, Taskfile, build.gradle, pom.xml, composer.json, mix.exs, etc.
  - Read the FULL contents of these files:
    - package.json: entire "scripts" section and key dependencies
    - Makefile/Justfile/Taskfile: ALL target/task definitions
    - Other manifest files: scripts, dependencies, configuration sections

Step 3: **Read CI/automation files completely**
  - Read full contents of .github/workflows/*, .gitlab-ci.yml, .buildkite/*.yml
  - Extract ALL commands from build, test, lint, and deploy jobs
  - These commands are the authoritative source of truth

Step 4: **Sample source code extensively** (minimum 5-8 files)
  - Read multiple files from different modules/packages/directories
  - Include both main source code and test files
  - Analyze consistently across files:
    - Indentation (tabs vs spaces, 2 vs 4)
    - Naming conventions (camelCase, snake_case, PascalCase)
    - Import/module patterns
    - Code organization patterns

Step 5: **Check commit history for conventions**
  - Review recent commit messages (last 15-20 commits)
  - Use `git log --oneline -20` or examine commit history
  - Identify patterns: Conventional Commits, ticket references, format standards

Step 6: **Read .gitignore completely**
  - Read full .gitignore file
  - Identify all generated/build directories (dist/, build/, gen/, coverage/, etc.)

⚠️ Critical Rules
1. **Only describe what you ACTUALLY find** - Do not invent or assume
2. **Be specific to THIS repo** - No generic boilerplate advice
3. **When uncertain, say so** - Add "TODO: Verify [detail]" rather than guessing
4. **For custom projects**: Look for wrapper commands in Makefile/Justfile/Taskfile (install, dev, build, test, lint). Use CI commands as the authoritative source.

🧱 Recommended Sections
Use these sections as a guide. Adapt headings and content based on what you discover.

**## Project Structure & Module Organization**
Outline the project structure - where source code, tests, and assets are located.
Include real directory paths found in this repository (e.g., src/**, tests/**, apps/**).

**## Build, Test, and Development Commands**
List key commands for building, testing, and running locally.
Explain what each command does (e.g., `npm test` runs unit tests).

**## Coding Style & Naming Conventions**
Specify indentation rules (tabs vs spaces, 2 vs 4).
Document language-specific style preferences and naming patterns (camelCase, snake_case, PascalCase).
Include any formatting or linting tools actually used (prettier, eslint, rustfmt, etc.).

**## Testing Guidelines**
Identify testing frameworks used (jest, pytest, cargo test, go test, etc.).
Explain how to run tests (all tests, single test, with coverage).
State coverage requirements or test naming conventions if present.

**## Commit & Pull Request Guidelines**
Summarize commit message conventions (check Git history for patterns).
Outline PR requirements: descriptions, linked issues, screenshots, review process.
Note if Conventional Commits or other standards are used.

**(Optional sections)** - Add if relevant:
- Security & Configuration Tips
- Architecture Overview
- Deployment Process

✍️ Writing Style
- Maintain a short, direct, and welcoming tone
- Provide concrete examples with real paths/commands from THIS repository
- Use bullet points or numbered lists for subsections to improve readability
- Format commands and paths with backticks (e.g., `npm test`, `src/**`)
# Source Rule File Format

Rule files are markdown files (`.md`) stored in your `ai-rules/` directory. There are two modes of operation: Standard Mode and Symlink Mode.

## Standard Mode

Use Standard Mode when you want fine-grained control over rules with YAML frontmatter.

### Format

```markdown
---
description: Context description for when to apply this rule
alwaysApply: true/false
fileMatching: "**/*.ext"
---

# Rule Content

Your markdown content here...
```

### Frontmatter Fields

All fields are optional:

| Field | Description | Default |
|-------|-------------|---------|
| `description` | Context description that helps agents understand when to apply this rule if `alwaysApply` is `false` | - |
| `alwaysApply` | `true` = referenced directly in agent rule files; `false` = included as optional rules based on context | `true` |
| `fileMatching` | Glob patterns for which files this rule applies to (e.g., `"**/*.ts"`, `"src/**/*.py"`). Currently supported in Cursor. | - |

If frontmatter is omitted entirely, the file is treated as a regular markdown rule with default settings (`alwaysApply: true`).

### How Standard Mode Works

In Standard Mode, `ai-rules generate` produces a single inlined file at `ai-rules/.generated-ai-rules/ai-rules-generated-AGENTS.md` that contains all rule content concatenated together. Each rule is preceded by a `# Description` heading (from its frontmatter `description` field) for structure.

Most agent output files (e.g., `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`) are created as **symlinks** pointing to this inlined file. This ensures every agent reads the same complete content without needing `@` file expansion support.

Cursor and Firebender generate their own formats (`.mdc` files and JSON respectively) and do not use the inlined file.

## Symlink Mode

Use Symlink Mode for simple setups where all agents share the same rules.

### Requirements

- Must be named `AGENTS.md`
- Must be the only file in the `ai-rules/` directory (commands/ and skills/ subdirectories are allowed)
- Must not start with `---` (no YAML frontmatter)

### Format

```markdown
# Project Rules

- Use TypeScript for all new code
- Write comprehensive tests
- Follow conventional commits
- Prefer explicit types over `any`
```

### How It Works

In Symlink Mode, `ai-rules generate` creates symlinks pointing to `ai-rules/AGENTS.md` for supported agents instead of generating separate files. This keeps all your rules in one place.

See [Supported Agents](agents.md) for details on which agents support symlink mode.

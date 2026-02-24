# Migration to agents.md standard

This guide describes how to migrate from the ai-rules–managed layout to the [agents.md](https://agents.md/) standard layout using `ai-rules migrate`. The migration is **one-way**: after migrating, the project no longer uses `ai-rules generate`, `ai-rules clean`, or `ai-rules status` for that content.

## Prerequisites

- Your project has an `ai-rules/` directory (created by `ai-rules init` or manually).
- **Back up** the `ai-rules/` directory if you want to keep the original layout; migration cannot be automatically reverted.

## Command options

| Option | Description |
|--------|-------------|
| `--nested-depth <N>` | Maximum nested directory depth to traverse (0 = current directory only). Same precedence as other commands: CLI overrides config file. |
| `--dry-run` | Print what would be done without writing or deleting. **Recommended first step.** |
| `--force` | Skip the confirmation prompt and run migration. |

## Confirmation behavior

- **With `--dry-run`**: No confirmation; the command only prints which directories would be migrated and what actions would be performed.
- **With `--force`**: No confirmation; migration runs immediately.
- **Without `--dry-run` or `--force`**: The command lists how many project(s) would be migrated, shows a warning that the change cannot be undone, and prompts: *"Proceed with migration? (y/N)"*. If you answer no, nothing is changed.

**Recommendation:** Run `ai-rules migrate --dry-run` first to see the list of directories and actions, then run `ai-rules migrate` (and confirm) or `ai-rules migrate --force` to perform the migration.

## What is written where

| Outcome | Description |
|--------|-------------|
| **Root `AGENTS.md`** | A single self-contained markdown file at the project root. In **symlink mode** (single `ai-rules/AGENTS.md` with no YAML frontmatter), its content is a copy of that file. In **standard mode** (multiple `.md` rules with frontmatter), content is the inlined combination of all rules (no `@` file references). |
| **`.agents/skills/`** | The directory `ai-rules/skills/` is **moved** here. The whole tree is relocated; nothing is left under `ai-rules/`. |
| **`.agents/commands/`** | The directory `ai-rules/commands/` is **moved** here. Same as skills. |
| **Other `ai-rules/` subdirs** | Any other non-generated directories under `ai-rules/` (e.g. custom dirs) are moved into `.agents/` with the same name so `ai-rules/` can be fully removed. |
| **Root `.mcp.json`** | If `ai-rules/mcp.json` existed, it is moved to the project root as `.mcp.json` so tools that read MCP from root (e.g. Claude Code) can use it. |

## What is removed

- **Generated files and symlinks**: All outputs previously created by `ai-rules generate` are removed (e.g. `CLAUDE.md`, `GEMINI.md`, `.cursor/rules/*.mdc`, `firebender.json`, command/skill symlinks in `.claude`, `.cursor`, `.agents`, etc.), using the same logic as `ai-rules clean`.
- **`ai-rules/` directory**: After moving content out and running the equivalent of clean, the entire `ai-rules/` directory is deleted.

## .gitignore behavior

The block between `# AI Rules - Generated Files` and `# End AI Rules` is removed from the project’s `.gitignore`. No new entries are added for `.agents/`; the new layout is intended to be committed.

## What is not migrated

- **`ai-rules-config.yaml`** and **`ai-rules/firebender-overlay.json`** are not copied; they become obsolete. If you use Firebender or other tools that relied on generated paths, reconfigure them to point at root `AGENTS.md` if desired.
- **Cursor `.cursor/rules/*.mdc`** and **Firebender `firebender.json`** are not regenerated from the new layout; those tools would need to be pointed at root `AGENTS.md` manually if you want to use them with the new layout.

## Nested / monorepo usage

Migration runs per directory. Each directory that has its own `ai-rules/` gets its own root `AGENTS.md` and `.agents/` at that directory (e.g. `frontend/AGENTS.md`, `frontend/.agents/`). Use `--nested-depth` to control how many levels are traversed (same as `generate` and `clean`).

## After migration

- The project no longer uses ai-rules for that content. To use ai-rules again you would need to recreate `ai-rules/` and run `generate` (not automated).
- Tools that read `AGENTS.md` / `AGENTS(S).md` and `.agents/` (e.g. [agents.md](https://agents.md/)) can use the new layout directly.

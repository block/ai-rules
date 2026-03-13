# Configuration

You can set default values for commonly used options in a configuration file. This is especially useful in team environments or when you have consistent preferences across projects.

## Configuration File

Create `ai-rules/ai-rules-config.yaml` in the `ai-rules` directory:

```yaml
agents: [claude, cursor, cline] # Generate rules only for these agents
command_agents: [claude, amp]   # Generate commands for these agents (defaults to agents list if not specified)
nested_depth: 2 # Search 2 levels deep for ai-rules/ folders
gitignore: true # Ignore the generated rules in git
include_dirs: [packages] # Override excluded directories (see below)
```

## Include Dirs

When traversing nested directories (`nested_depth > 0`), certain directories are excluded by default to avoid scanning build artifacts and dependency folders: `target`, `build`, `dist`, `out`, `bin`, `obj`, `node_modules`, `vendor`, `packages`, `__pycache__`, `.pytest_cache`, `.cache`, `.vscode`, `.idea`, `.vs`, `tmp`, `temp`, `logs`.

The `include_dirs` option lets you opt specific excluded directories back in. This is useful for monorepos that use `packages/` (yarn workspaces, Lerna, Turborepo) or `vendor/` directories containing nested projects with their own `ai-rules/` folders.

**Config file:**
```yaml
nested_depth: 3
include_dirs: [packages, vendor]
```

**CLI flag:**
```bash
ai-rules generate --nested-depth 3 --include-dirs packages,vendor
```

The CLI flag takes precedence over the config file value.

## Configuration Precedence

Options are resolved in the following order (highest to lowest priority):

1. **CLI options** - `--agents`, `--nested-depth`, `--gitignore`, `--include-dirs`
2. **Config file** - `ai-rules/ai-rules-config.yaml` (at current working directory)
3. **Default values** - All agents, depth 0, generated files are NOT git ignored

## Experimental Options

### Claude Code Skills Mode

```yaml
use_claude_skills: true  # Default: false
```

Experimental toggle to test Claude Code's skills feature. When enabled, rules with `alwaysApply: false` are generated as separate skills in `.claude/skills/` instead of being included in `CLAUDE.md`. This allows Claude Code to selectively apply optional rules based on context.

See [Commands and Skills](commands-and-skills.md) for more details on skills.

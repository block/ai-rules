# Configuration

You can set default values for commonly used options in a configuration file. This is especially useful in team environments or when you have consistent preferences across projects.

## Configuration File

Create `ai-rules/ai-rules-config.yaml` in the `ai-rules` directory:

```yaml
agents: [claude, cursor, cline] # Generate rules only for these agents
command_agents: [claude, amp]   # Generate commands for these agents (defaults to agents list if not specified)
nested_depth: 2 # Search 2 levels deep for ai-rules/ folders
gitignore: true # Ignore the generated rules in git
```

## Default Agents via `.env`

You can set a default agent list in `ai-rules/.env` using the `AI_RULES_AGENTS` key:

```sh
# ai-rules/.env
AI_RULES_AGENTS=claude,cursor,gemini
```

This is useful when you want a personal default that differs from the team config (or when there's no config file at all). Unrecognised agent names in any list are skipped with a warning.

## Configuration Precedence

Options are resolved in the following order (highest to lowest priority):

1. **CLI options** - `--agents`, `--nested-depth`, `--no-gitignore`
2. **Config file** - `ai-rules/ai-rules-config.yaml` (at current working directory)
3. **`.env` file** - `AI_RULES_AGENTS` in `ai-rules/.env`
4. **Default values** - All agents, depth 0, generated files are NOT git ignored

## Experimental Options

### Claude Code Skills Mode

```yaml
use_claude_skills: true  # Default: false
```

Experimental toggle to test Claude Code's skills feature. When enabled, rules with `alwaysApply: false` are generated as separate skills in `.claude/skills/` instead of being included in `CLAUDE.md`. This allows Claude Code to selectively apply optional rules based on context.

See [Commands and Skills](commands-and-skills.md) for more details on skills.

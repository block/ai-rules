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

## Configuration Precedence

Options are resolved in the following order (highest to lowest priority):

1. **CLI options** - `--agents`, `--nested-depth`, `--no-gitignore`
2. **Config file** - `ai-rules/ai-rules-config.yaml` (at current working directory)
3. **Default values** - All agents, depth 0, generated files are NOT git ignored

## Experimental Options

### Claude Code Skills Mode

```yaml
use_claude_skills: true  # Default: false
```

Experimental toggle to test Claude Code's skills feature. When enabled, rules with `alwaysApply: false` are generated as separate skills in `.claude/skills/` instead of being included in `CLAUDE.md`. This allows Claude Code to selectively apply optional rules based on context.

See [Commands and Skills](commands-and-skills.md) for more details on skills.

# Commands and Skills

## Custom Commands

Custom commands (also called "slash commands") allow you to define reusable prompts that can be invoked by name in supported AI agents.

### Setup

Create markdown files in `ai-rules/commands/`:

```
ai-rules/
└── commands/
    └── commit.md
```

### Frontmatter Fields

Command files support optional YAML frontmatter:

| Field | Description | Agent Support |
|-------|-------------|---------------|
| `allowed-tools` | Tool restrictions for the command | Claude only |
| `description` | Human-readable description of what the command does | All |
| `model` | Specific model to use for this command | Claude only |

### Agent Behavior

| Agent | Output Location | Frontmatter |
|-------|-----------------|-------------|
| **AMP** | `.agents/commands/{name}-ai-rules.md` | Stripped |
| **Claude Code** | `.claude/commands/ai-rules/*.md` | Preserved |
| **Cursor** | `.cursor/commands/ai-rules/*.md` | Stripped |
| **Firebender** | `firebender.json` (commands array) | Stripped |

### Documentation

- [Claude Code Slash Commands](https://code.claude.com/docs/en/slash-commands)
- [Cursor Commands](https://cursor.com/docs/agent/chat/commands)
- [Firebender Commands](https://docs.firebender.com/context/commands)

---

## Claude Code Skills

Claude Code supports optional rules through [skills](https://docs.claude.com/en/docs/claude-code/skills). This requires `use_claude_skills: true` in your config.

When enabled and a source rule has `alwaysApply: false`, the tool generates:
- **CLAUDE.md** - References required rules (`alwaysApply: true`) only
- **.claude/skills/{rule-name}/SKILL.md** - Individual skill files for optional rules

### Example

Source file `ai-rules/testing.md`:

```markdown
---
description: React Testing Library best practices
alwaysApply: false
---
# Testing Rules
- Prefer user-centric queries (getByRole, getByLabelText)
- Avoid implementation details (testId, class names)
- Test behavior, not implementation
```

Generates:
- `CLAUDE.md` - Contains only required rules
- `.claude/skills/testing/SKILL.md` - Skill that Claude can invoke when working with test files

Skills use the `description` field as the skill name for better discoverability.

---

## User-Defined Skills

You can define custom skill folders that are symlinked to supported agents' skill directories during generation.

### Setup

Create skill folders in `ai-rules/skills/<skill-name>/` with a `SKILL.md` file:

```
ai-rules/
└── skills/
    └── debugging/
        └── SKILL.md
```

### SKILL.md Format

```markdown
---
name: debugging
description: Debugging guidelines and best practices
---

# Debugging Guidelines

Your skill content here...
```

### Generated Symlinks

When you run `ai-rules generate`, symlinks are created:

| Agent | Symlink Location |
|-------|------------------|
| AMP | `.agents/skills/ai-rules-generated-debugging` -> `../../ai-rules/skills/debugging` |
| Claude | `.claude/skills/ai-rules-generated-debugging` -> `../../ai-rules/skills/debugging` |
| Codex | `.codex/skills/ai-rules-generated-debugging` -> `../../ai-rules/skills/debugging` |
| Cursor | `.cursor/skills/ai-rules-generated-debugging` -> `../../ai-rules/skills/debugging` |

Skill folders without a `SKILL.md` file are skipped with a warning.

# Supported AI Coding Agents

## Agent Compatibility Table

| Agent | Standard Mode | Symlink Mode | MCP Support | Notes |
|-------|---------------|--------------|-------------|-------|
| **AMP** | `AGENTS.md` | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | |
| **Claude Code** | `CLAUDE.md` (+ `.claude/skills/` if configured) | `CLAUDE.md` -> `ai-rules/AGENTS.md` | `.mcp.json` | Skills support via `use_claude_skills` config |
| **Cline** | `.clinerules/*.md` | `.clinerules/AGENTS.md` -> `../ai-rules/AGENTS.md` | - | |
| **Codex** | `AGENTS.md` | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | |
| **Copilot** | `AGENTS.md` | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | |
| **Cursor** | `.cursor/rules/*.mdc` | `AGENTS.md` -> `ai-rules/AGENTS.md` | `.cursor/mcp.json` | Symlink mode: only project root level |
| **Firebender** | `firebender.json` | `firebender.json` (references `ai-rules/AGENTS.md`) | Embedded in `firebender.json` | Supports overlay files |
| **Gemini** | `GEMINI.md` | `GEMINI.md` -> `ai-rules/AGENTS.md` | Embedded in `.gemini/settings.json` | |
| **Goose** | `AGENTS.md` | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | |
| **JetBrains AI Assistant** | `.aiassistant/rules/*.md` | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | Plain markdown, no frontmatter |
| **JetBrains Junie** | `AGENTS.md` | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | Native AGENTS.md support |
| **Kilocode** | `.kilocode/rules/*.md` | `.kilocode/rules/AGENTS.md` -> `../../ai-rules/AGENTS.md` | - | |
| **Roo** | `.roo/rules/*.md` | `.roo/rules/AGENTS.md` -> `../../ai-rules/AGENTS.md` | `.roo/mcp.json` | |

## Firebender Overlay Support

Firebender supports overlay configuration files. To customize your configuration:

1. Create `ai-rules/firebender-overlay.json` in the same parent directory as your generated `firebender.json`
2. Any values defined in the overlay file will be merged into the base configuration, with overlay values taking precedence

**MCP Integration:** If you have `ai-rules/mcp.json`, the MCP servers are merged into `firebender.json` first, then the overlay is applied. This allows you to override MCP configurations in the overlay if needed.

# Supported AI Coding Agents

## Agent Compatibility Table

| Agent | Standard Mode | Symlink Mode | MCP Support | Notes |
|-------|---------------|--------------|-------------|-------|
| **AMP** | `AGENTS.md` -> inlined file | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | |
| **Claude Code** | `CLAUDE.md` -> inlined file | `CLAUDE.md` -> `ai-rules/AGENTS.md` | `.mcp.json` | |
| **Cline** | `.clinerules/AGENTS.md` -> inlined file | `.clinerules/AGENTS.md` -> `../ai-rules/AGENTS.md` | - | |
| **Codex** | `AGENTS.md` -> inlined file | `AGENTS.md` -> `ai-rules/AGENTS.md` | `.codex/config.toml` | Optional `ai-rules/codex-config.toml` overlay supported |
| **Copilot** | `AGENTS.md` -> inlined file | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | |
| **Cursor** | `.cursor/rules/*.mdc` | `AGENTS.md` -> `ai-rules/AGENTS.md` | `.cursor/mcp.json` | Symlink mode: only project root level |
| **Firebender** | `AGENTS.md` -> inlined file | `AGENTS.md` -> `ai-rules/AGENTS.md` | `firebender.json` | Commands live in `.firebender/`; skills reuse `.agents/skills`; overlay supported |
| **Gemini** | `GEMINI.md` -> inlined file | `GEMINI.md` -> `ai-rules/AGENTS.md` | Embedded in `.gemini/settings.json` | |
| **Goose** | `AGENTS.md` -> inlined file | `AGENTS.md` -> `ai-rules/AGENTS.md` | - | |
| **Kilocode** | `.kilocode/rules/AGENTS.md` -> inlined file | `.kilocode/rules/AGENTS.md` -> `../../ai-rules/AGENTS.md` | - | |
| **Roo** | `.roo/rules/AGENTS.md` -> inlined file | `.roo/rules/AGENTS.md` -> `../../ai-rules/AGENTS.md` | `.roo/mcp.json` | |

## Firebender Overlay Support

Firebender supports overlay configuration files. To customize your configuration:

1. Create `ai-rules/firebender-overlay.json` in the same parent directory as your generated `firebender.json`
2. Any values defined in the overlay file will be merged into the base configuration, with overlay values taking precedence

`firebender.json` is generated as supplemental config when `ai-rules/mcp.json` and/or `ai-rules/firebender-overlay.json` exists.

**MCP Integration:** If you have `ai-rules/mcp.json`, the MCP servers are merged into `firebender.json` first, then the overlay is applied. This allows you to override MCP configurations in the overlay if needed.

## Codex Overlay Support

Codex supports overlay configuration files. To customize your configuration:

1. Create `ai-rules/codex-config.toml` in the same parent directory as your generated `.codex/config.toml`
2. Any values defined in the overlay file will be merged into the generated Codex configuration, with overlay values taking precedence

`.codex/config.toml` is generated when `ai-rules/mcp.json` and/or `ai-rules/codex-config.toml` exists.

**MCP Integration:** If you have `ai-rules/mcp.json`, the MCP servers are converted into Codex `[mcp_servers]` TOML first, then the overlay is applied. This allows you to override MCP configurations in the overlay if needed.

# Project Structure Examples

## Standard Mode

For projects using YAML frontmatter in rule files:

```
monorepo/
├── ai-rules/                     # Global rule files
│   ├── .generated-ai-rules/      # Processed files (generated)
│   ├── commands/                 # Custom commands
│   │   └── commit.md
│   ├── skills/                   # User-defined skills
│   │   └── debugging/
│   │       └── SKILL.md
│   ├── general.md                # Repository-wide rules
│   ├── ai-rules-config.yaml      # Configuration
│   └── mcp.json                  # MCP server configuration
│
├── frontend/                     # Frontend application
│   ├── ai-rules/                 # Frontend-specific rules
│   │   ├── .generated-ai-rules/
│   │   ├── react.md
│   │   └── styling.md
│   ├── CLAUDE.md                 # Generated
│   ├── AGENTS.md                 # Generated
│   ├── .cursor/rules/            # Generated (*.mdc)
│   └── src/
│
├── backend/                      # Backend services
│   ├── ai-rules/
│   │   ├── .generated-ai-rules/
│   │   ├── api.md
│   │   └── database.md
│   ├── CLAUDE.md
│   ├── AGENTS.md
│   └── api/
│
├── CLAUDE.md                     # Root rules (generated)
├── AGENTS.md                     # Root rules (generated)
├── .cursor/rules/                # Root Cursor rules (generated)
├── .clinerules/                  # Root Cline rules (generated)
├── .mcp.json                     # Root MCP config (generated)
└── firebender.json               # Root Firebender config (generated)
```

## Symlink Mode

For simple projects with a single `AGENTS.md` file:

```
project/
├── ai-rules/
│   ├── AGENTS.md                 # Source file (your rules)
│   ├── commands/                 # Custom commands (optional)
│   │   └── commit.md
│   ├── skills/                   # User-defined skills (optional)
│   │   └── debugging/
│   │       └── SKILL.md
│   └── mcp.json                  # MCP config (optional)
│
├── CLAUDE.md                     # Symlink -> ai-rules/AGENTS.md
├── GEMINI.md                     # Symlink -> ai-rules/AGENTS.md
├── AGENTS.md                     # Symlink -> ai-rules/AGENTS.md
├── firebender.json               # References ai-rules/AGENTS.md
├── .clinerules/
│   └── AGENTS.md                 # Symlink -> ../ai-rules/AGENTS.md
├── .cursor/
│   ├── commands/ai-rules/        # Commands (generated)
│   └── mcp.json                  # MCP config (if mcp.json exists)
├── .kilocode/rules/
│   └── AGENTS.md                 # Symlink -> ../../ai-rules/AGENTS.md
├── .roo/
│   ├── rules/
│   │   └── AGENTS.md             # Symlink -> ../../ai-rules/AGENTS.md
│   └── mcp.json                  # MCP config (if mcp.json exists)
└── .mcp.json                     # Claude MCP config (if mcp.json exists)
```

# MCP Configuration

The AI Rules Tool supports generating Model Context Protocol (MCP) configurations for compatible AI coding agents. MCP enables AI agents to connect to external tools and services.

## Setup

Create `ai-rules/mcp.json` with your MCP server configurations:

```json
{
  "mcpServers": {
    "server-name": {
      "command": "executable-command",
      "args": ["arg1", "arg2"],
      "env": {
        "API_KEY": "${MY_API_KEY}"
      }
    },
    "remote-server-name": {
      "type": "http",
      "url": "https://api.example.com/mcp"
    }
  }
}
```

## Environment Variable Substitution

Any `${VAR_NAME}` placeholder in `mcp.json` is substituted with the corresponding environment variable at generation time. This applies to all string values in the config — server commands, args, env vars, URLs, and headers.

**Shell environment variables take priority.** If `MY_API_KEY` is exported in your shell, that value is used. If not, the tool falls back to the `.env` file (see below).

If a variable is not set anywhere, the placeholder is left as-is and a warning is printed.

### Using a `.env` file

Create `ai-rules/.env` to store secrets locally without setting shell variables:

```sh
# ai-rules/.env
MY_API_KEY=sk-abc123
FIGMA_TOKEN=figd_xyz
DATABASE_URL=postgresql://user:pass@host/db
```

Supported syntax:
- `KEY=value` — basic assignment
- `KEY="value with spaces"` or `KEY='value'` — surrounding quotes are stripped
- `# comment` — lines starting with `#` are ignored
- Values containing `=` are handled correctly (`KEY=a=b` parses as `a=b`)

**Add `ai-rules/.env` to your `.gitignore`** — it contains secrets and should not be committed.

## Generation

Run `ai-rules generate` to automatically create agent-specific MCP configurations.

## Supported Agents

See the [Supported Agents](agents.md) table for which agents support MCP and their generated file locations:

### Project mode (`ai-rules generate`)

| Agent | MCP File Location |
|-------|-------------------|
| Claude Code | `.mcp.json` |
| Cursor | `.cursor/mcp.json` |
| Firebender | Embedded in `firebender.json` |
| Gemini | Embedded in `.gemini/settings.json` |
| Roo | `.roo/mcp.json` |

### Global mode (`ai-rules generate --global`)

| Agent | MCP File Location |
|-------|-------------------|
| Claude Code | `~/.claude.json` (merged into `mcpServers`) |
| Cursor | `~/.cursor/mcp.json` |
| Gemini | `~/.gemini/settings.json` (merged into `mcpServers`) |

Generated servers are prefixed with `air-` so they can be identified and cleaned up without affecting manually configured servers.

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
        "ENV_VAR": "${use_environment_variable}"
      }
    },
    "remote-server-name": {
      "type": "http",
      "url": "https://api.example.com/mcp"
    }
  }
}
```

## Generation

Run `ai-rules generate` to automatically create agent-specific MCP configurations.

## Supported Agents

See the [Supported Agents](agents.md) table for which agents support MCP and their generated file locations:

| Agent | MCP File Location |
|-------|-------------------|
| Claude Code | `.mcp.json` |
| Cursor | `.cursor/mcp.json` |
| Firebender | Embedded in `firebender.json` |
| Gemini | Embedded in `.gemini/settings.json` |
| Roo | `.roo/mcp.json` |

# AI Rules Tool

CLI tool to manage AI rules across different AI coding agents. Standardize and distribute your coding guidelines across AMP, Claude, Cline, Codex, Copilot, Cursor, Firebender, Gemini, Goose, Kilocode, and Roo.

## Features

- **Multi-Agent Support** - Generate rules for 11 AI coding agents from a single source
- **Sync Management** - Track and maintain consistency across all generated rule files
- **Selective Generation** - Generate rules for specific agents only
- **Non-Destructive Output** - Manages a section within existing markdown files, preserving your own content
- **MCP Support** - Generate Model Context Protocol configurations for compatible agents

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/block/ai-rules/main/scripts/install.sh | bash
```

Installs to `~/.local/bin/ai-rules`. Verify with `ai-rules --version`.

<details>
<summary>More installation options</summary>

**Specific version:**
```bash
curl -fsSL https://raw.githubusercontent.com/block/ai-rules/main/scripts/install.sh | VERSION=v1.0.0 bash
```

**Custom directory:**
```bash
curl -fsSL https://raw.githubusercontent.com/block/ai-rules/main/scripts/install.sh | INSTALL_DIR=/usr/local/bin bash
```
</details>

## Quick Start

1. **Initialize** your AI rules directory:
   ```bash
   ai-rules init
   ```
   Creates an `ai-rules/` directory with example rule files. Rule files are markdown files containing coding guidelines, conventions, and instructions that get distributed to your AI coding agents.

2. **Edit your rules** in `ai-rules/*.md` files to define your project's coding standards

3. **Generate** agent-specific files:
   ```bash
   ai-rules generate                            # All agents
   ai-rules generate --agents claude,cursor     # Specific agents
   ai-rules generate --global                   # Generate to home directory (~/.claude/CLAUDE.md etc.)
   ```
   Creates or updates `CLAUDE.md`, `.cursor/rules/*.mdc`, `AGENTS.md`, etc. For markdown files, ai-rules manages a dedicated section — your own content in the same file is preserved.

4. **Check status** to ensure everything is in sync:
   ```bash
   ai-rules status
   ```

## Commands

| Command | Description |
|---------|-------------|
| `ai-rules init` | Initialize AI rules in the current directory |
| `ai-rules generate` | Generate rules for AI coding agents |
| `ai-rules status` | Show sync status of AI rules |
| `ai-rules clean` | Remove all generated files |
| `ai-rules list-agents` | List all supported agents |

### Common Options

```bash
ai-rules generate --agents claude,cursor    # Generate for specific agents
ai-rules generate --nested-depth 2          # Process subdirectories
ai-rules generate --gitignore               # Add generated files to .gitignore
ai-rules generate --global                  # Generate to home directory paths
```

You can also set a default agent list in `ai-rules/.env` without passing `--agents` every time:

```sh
# ai-rules/.env
AI_RULES_AGENTS=claude,cursor,gemini
```

## Configuration

Create `ai-rules/ai-rules-config.yaml` to set defaults:

```yaml
agents: [claude, cursor, cline]
nested_depth: 2
gitignore: true
```

See [Configuration Guide](docs/configuration.md) for all options.

## Supported Agents

AMP, Claude Code, Cline, Codex, Copilot, Cursor, Firebender, Gemini, Goose, Kilocode, Roo

See [Supported Agents](docs/agents.md) for detailed compatibility information.

## Documentation

- [Configuration](docs/configuration.md) - Config file, `.env` defaults, and precedence
- [Rule Format](docs/rule-format.md) - Standard mode and symlink mode formats
- [Supported Agents](docs/agents.md) - Agent compatibility and generated files
- [MCP Configuration](docs/mcp.md) - Model Context Protocol setup
- [Commands and Skills](docs/commands-and-skills.md) - Custom commands and skills
- [Project Structure](docs/project-structure.md) - Example project layouts

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

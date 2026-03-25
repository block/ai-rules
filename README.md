# AI Rules Tool

CLI tool to manage AI rules across different AI coding agents. Standardize and distribute your coding guidelines across AMP, Claude, Cline, Codex, Copilot, Cursor, Firebender, Gemini, Goose, Kilocode, and Roo.

## Features

- **Multi-Agent Support** - Generate rules for 11 AI coding agents from a single source
- **Sync Management** - Track and maintain consistency across all generated rule files
- **Selective Generation** - Generate rules for specific agents only
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
   ```
   Creates `CLAUDE.md`, `.cursor/rules/*.mdc`, `AGENTS.md`, etc.

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
| `ai-rules migrate` | Migrate from ai-rules/ layout to agents.md standard (one-way) |
| `ai-rules list-agents` | List all supported agents |

### Common Options

```bash
ai-rules generate --agents claude,cursor    # Generate for specific agents
ai-rules generate --nested-depth 2          # Process subdirectories
ai-rules generate --gitignore               # Add generated files to .gitignore
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

## Migration to agents.md standard

You can migrate from the ai-rules–managed layout to the [agents.md](https://agents.md/) standard: a single `AGENTS.md` at project root, skills in `.agents/skills/`, and commands in `.agents/commands/`. Run `ai-rules migrate` to write root `AGENTS.md`, move `ai-rules/skills` and `ai-rules/commands` into `.agents/`, remove generated files, and purge the `ai-rules/` directory. **This is one-way**; after migrating, the project no longer uses ai-rules generate/clean/status for that content. Use `--dry-run` first to see what would be done, then run without it (and confirm) or with `--force` to migrate. See [Migration guide](docs/migration.md) for details.

## Documentation

- [Configuration](docs/configuration.md) - Config file options and precedence
- [Rule Format](docs/rule-format.md) - Standard mode and symlink mode formats
- [Supported Agents](docs/agents.md) - Agent compatibility and generated files
- [MCP Configuration](docs/mcp.md) - Model Context Protocol setup
- [Commands and Skills](docs/commands-and-skills.md) - Custom commands and skills
- [Project Structure](docs/project-structure.md) - Example project layouts
- [Migration to agents.md](docs/migration.md) - One-way migration from ai-rules/ layout

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

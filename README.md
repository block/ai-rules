# AI Rules Tool

CLI tool to manage AI rules across different AI coding agents. Standardize and distribute your coding guidelines and preferences across various development environments including AMP, Claude, Cline, Codex, Copilot, Cursor, Firebender, Gemini, Goose, Kilocode, and Roo.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Commands](#commands)
  - [`ai-rules init`](#ai-rules-init)
  - [`ai-rules generate [--agents <agent1,agent2>] [--gitignore] [--nested-depth <depth>]`](#ai-rules-generate---agents-agent1agent2--gitignore---nested-depth-depth)
  - [`ai-rules status [--agents <agent1,agent2>] [--nested-depth <depth>]`](#ai-rules-status---agents-agent1agent2---nested-depth-depth)
  - [`ai-rules clean [--nested-depth <depth>]`](#ai-rules-clean---nested-depth-depth)
  - [`ai-rules list-agents`](#ai-rules-list-agents)
- [Source Rule File Format (.md)](#source-rule-file-format-md)
  - [Standard Mode Format](#standard-mode-format)
  - [Symlink Mode Format](#symlink-mode-format)
- [MCP Configuration](#mcp-configuration)
- [Supported AI Coding Agents](#supported-ai-coding-agents)
  - [Firebender Overlay Support](#firebender-overlay-support)
  - [Custom Commands Support](#custom-commands-support)
  - [Claude Code Skills Support](#claude-code-skills-support)
  - [User-Defined Skills](#user-defined-skills)
- [Project Structure](#project-structure)
  - [Standard Mode](#standard-mode)
  - [Symlink Mode](#symlink-mode)
- [Development](#development)

## Features

- 🤖 **Multi-Agent Support** - Generate rules for AI coding agents including AMP, Claude, Cline, Codex, Copilot, Cursor, Firebender, Gemini, Goose, Kilocode, and Roo
- 🔄 **Sync Management** - Track and maintain consistency across all generated rule files
- 🧹 **Easy Cleanup** - Remove generated files when needed
- 🎯 **Selective Generation** - Generate rules for specific agents only

## Installation

### Quick Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/block/ai-rules/main/scripts/install.sh | bash
```

Installs to `~/.local/bin/ai-rules`. Verify with `ai-rules --version`.

### Install Specific Version

```bash
curl -fsSL https://raw.githubusercontent.com/block/ai-rules/main/scripts/install.sh | VERSION=v1.0.0 bash
```

### Custom Install Directory

```bash
curl -fsSL https://raw.githubusercontent.com/block/ai-rules/main/scripts/install.sh | INSTALL_DIR=/usr/local/bin bash
```

## Quick Start

1. **Set up your AI rules directory** 
   ```bash
   ai-rules init
   ```
   This creates an `ai-rules/` directory with an example rule file. The command uses a built-in [Goose recipe](https://block.github.io/goose/docs/guides/recipes/) to generate a context-aware rule file automatically. If Goose is not available, it falls back to creating a basic example template.

2. **Add/Edit your rules** in `ai-rules/example.md` or create your own rule files

3. **Configure for rule generation**(Optional) in `ai-rules/ai-rules-config.yaml` (see [Configuration](#configuration))

4. **Generate** rules
   ```bash
   ai-rules generate
   ```
   This creates agent-specific files like `CLAUDE.md`, `.cursor/rules/*.mdc`, `.clinerules/*.md`, `AGENTS.md`, `firebender.json`, etc.

5. **Check status** to ensure everything is in sync:
   ```bash
   ai-rules status
   ```

## Configuration

You can set default values for commonly used options in a configuration file. This is especially useful in team environments or when you have consistent preferences across projects.

### Configuration File

Create `ai-rules/ai-rules-config.yaml` in the `ai-rules` directory. Example:

```yaml
agents: [claude, cursor, cline] # Generate rules only for these agents
command_agents: [claude, amp]   # Generate commands for these agents (defaults to agents list if not specified)
nested_depth: 2 # Search 2 levels deep for ai-rules/ folders
gitignore: true # Ignore the generated rules in git
```

### Configuration Precedence

Options are resolved in the following order (highest to lowest priority):

1. **CLI options** - `--agents`, `--nested-depth`, `--no-gitignore`
2. **Config file** - `ai-rules/ai-rules-config.yaml` (at current working directory)
3. **Default values** - All agents, depth 0, generated files are NOT git ignored

### Experimental Options

**Claude Code Skills Mode (Testing)**

```yaml
use_claude_skills: true  # Default: false
```

Experimental toggle to test Claude Code's skills feature. When enabled, rules with `alwaysApply: false` are generated as separate skills in `.claude/skills/` instead of being included in `CLAUDE.md`. This allows Claude Code to selectively apply optional rules based on context.

## Commands

### `ai-rules init`
Initialize AI rules in the current directory. Uses Goose recipes to generate context-aware rule files:
- **Custom recipe** (`ai-rules/custom-init/recipe.yaml` at git root): Run your own initialization workflow with custom parameters
- **Built-in recipe** ([Recipe](src/templates/init_default_recipe.yaml)): Generate a single rule file based on your project context
- **Fallback**: Creates a basic example template if Goose is not available

**Options:**
- `--params <key=value>` - Pass custom parameters to recipes (can be specified multiple times)
- `--force` - Skip confirmation prompts and assume yes. Automatically bypasses the "Run Goose to initialize another rule file?" prompt when rules already exist. For custom recipes, passes `force=true` as a parameter.

**Examples:**
```bash
# Basic initialization
ai-rules init

# Pass parameters to custom recipes (e.g., service name, team ownership)
ai-rules init --params service=payments --params owner=checkout

# Force initialization without confirmation prompts
ai-rules init --force
```
### `ai-rules generate [--agents <agent1,agent2>] [--gitignore] [--nested-depth <depth>]`
Generate rules for AI coding agents from your `ai-rules/*.md` source files.

**Options:**
- `--agents` - Comma-separated list of specific agents (e.g., `--agents claude,cursor`). Can be set in config.
- `--gitignore` - Add generated file patterns to .gitignore. Can be set in config.
- `--no-gitignore` - (Deprecated: use `--gitignore` instead) Skip updating .gitignore with generated file patterns.
- `--nested-depth` - Maximum directory depth to scan for `ai-rules/` folders (default: 0). Can be set in config.
  - `0` - Only process current directory
  - `1` - Process current directory and immediate subdirectories
  - `2` - Process up to 2 levels deep, etc.

See [Configuration](#configuration) section for setting defaults.

**Generates:**

**Standard Mode** (when `ai-rules/*.md` files have YAML frontmatter):
- Rules files for AI coding agents (see [Supported AI Coding Agents](#supported-ai-coding-agents))
- `ai-rules/.generated-ai-rules/` - Directory with extracted rule content files, referenced by coding agent rule files
- Skill symlinks from `ai-rules/skills/` to agent skill directories
- Updates `.gitignore` with generated files (only if `--gitignore` is specified)

**Symlink Mode** (when only `AGENTS.md` exists in `ai-rules/` without frontmatter):
- Symlinks pointing directly to `ai-rules/AGENTS.md` for supported agents
- Skill symlinks from `ai-rules/skills/` to agent skill directories
- No `ai-rules/.generated-ai-rules/` directory created
- Updates `.gitignore` with symlink files (only if `--gitignore` is specified)

**Examples:**
```bash
ai-rules generate
ai-rules generate --agents claude,cursor
ai-rules generate --nested-depth 2
ai-rules generate --agents claude,cursor --nested-depth 1
```

### `ai-rules status [--agents <agent1,agent2>] [--nested-depth <depth>]`
Show the current sync status of AI rules

**Options:**
- `--agents` - Comma-separated list of specific agents to check (e.g., `--agents claude,cursor`). Can be set in config.
- `--nested-depth` - Maximum directory depth to check for rules in `ai-rules/` folders (default: 0). Can be set in config.

See [Configuration](#configuration) section for setting defaults.

**Examples:**
```bash
ai-rules status
ai-rules status --nested-depth 1

# Output:
# ✅ Claude: In sync
# ❌ Cursor: Out of sync
# ✅ Goose: In sync
```

**Exit Codes:** Returns `0` if in sync, `1` if out of sync, or `2` if no rules found. Use this in build scripts to ensure generated rules stay in sync.

### `ai-rules clean [--nested-depth <depth>]`
Remove all generated AI rule files while preserving source files in `ai-rules/`.

**Options:**
- `--nested-depth` - Maximum directory depth to clean generated files (default: 0). Can be set in config.

See [Configuration](#configuration) section for setting defaults.

**Examples:**
```bash
ai-rules clean
ai-rules clean --nested-depth 2
```

### `ai-rules list-agents`
List all supported AI coding agents that rules can be generated for.

**Example:**
```bash
ai-rules list-agents

# Output:
# Supported agents:
#   • amp
#   • claude
#   • cline
#   • codex
#   ....
```


## Source Rule File Format (.md)

### Standard Mode Format

Rule files use a markdown format with optional YAML frontmatter:

```markdown
---
description: Context description for when to apply this rule
alwaysApply: true/false
fileMatching: "**/*.ext"
---

# Rule Content

Your markdown content here...
```

**Frontmatter Fields (all optional):**
- `description` - Context description that helps agents understand when to apply this rule if `alwaysApply` is `false`.
- `alwaysApply` - Controls when this rule is applied:
  - `true` - Referenced directly in coding agent rule files
  - `false` - Included in the coding agent rule files as optional rules based on context
  - Default: `true` (if not specified)
- `fileMatching` - Glob patterns for which files this rule applies to (e.g., `"**/*.ts"`, `"src/**/*.py"`). Currently supported in Cursor.

**Note:** If frontmatter is omitted entirely, the file is treated as a regular markdown rule with default settings (`alwaysApply: true`).

## MCP Configuration

The AI Rules Tool supports generating Model Context Protocol (MCP) configurations for compatible AI coding agents. MCP enables AI agents to connect to external tools and services.

### Setup

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

Run `ai-rules generate` to automatically create agent-specific MCP configurations. See the [Supported AI Coding Agents](#supported-ai-coding-agents) table for which agents support MCP and their generated file locations.

### Symlink Mode Format

For symlink mode, use a single `AGENTS.md` file with pure markdown (no YAML frontmatter):

```markdown
# Project Rules

- Use TypeScript for all new code
- Write comprehensive tests
- Follow conventional commits
- Prefer explicit types over `any`
```

**Requirements:**
- Must be named `AGENTS.md` 
- Must be the only file in the `ai-rules/` directory
- Must not start with `---` (no YAML frontmatter)
- Content is used directly by all supported agents via symlinks

## Supported AI Coding Agents

| AI Coding Agent | Standard Mode | Symlink Mode | MCP Support | Notes |
|------|-------------|-------------|-------------|-------|
| **AMP** | `AGENTS.md` | ✅ `AGENTS.md` → `ai-rules/AGENTS.md` | ❌ | |
| **Claude Code** | `CLAUDE.md` (+ `.claude/skills/` if configured) | ✅ `CLAUDE.md` → `ai-rules/AGENTS.md` | ✅ `.mcp.json` | Skills support via `use_claude_skills` config |
| **Cline** | `.clinerules/*.md` | ✅ `.clinerules/AGENTS.md` → `../ai-rules/AGENTS.md` | ❌ | |
| **Codex** | `AGENTS.md` | ✅ `AGENTS.md` → `ai-rules/AGENTS.md` | ❌ | |
| **Copilot** | `AGENTS.md` | ✅ `AGENTS.md` → `ai-rules/AGENTS.md` | ❌ | |
| **Cursor** | `.cursor/rules/*.mdc` | ✅ `AGENTS.md` → `ai-rules/AGENTS.md` | ✅ `.cursor/mcp.json` | Symlink mode: only project root level |
| **Firebender** | `firebender.json` | ✅ `firebender.json` (references `ai-rules/AGENTS.md`) | ✅ Embedded in `firebender.json` | Supports overlay files |
| **Gemini** | `GEMINI.md` | ✅ `GEMINI.md` → `ai-rules/AGENTS.md` | ✅ Embedded in `.gemini/settings.json` | |
| **Goose** | `AGENTS.md` | ✅ `AGENTS.md` → `ai-rules/AGENTS.md` | ❌ | |
| **Kilocode** | `.kilocode/rules/*.md` | ✅ `.kilocode/rules/AGENTS.md` → `../../ai-rules/AGENTS.md` | ❌ | |
| **Roo** | `.roo/rules/*.md` | ✅ `.roo/rules/AGENTS.md` → `../../ai-rules/AGENTS.md` | ✅ `.roo/mcp.json` | |

### Firebender Overlay Support

Firebender supports overlay configuration files. To customize your configuration, create a `firebender-overlay.json` file inside the `ai-rules/` directory, located in the same parent directory as your generated `firebender.json` file. Any values defined in the overlay file will be merged into the base configuration, with overlay values taking precedence.

**MCP Integration:** If you have `ai-rules/mcp.json`, the MCP servers are merged into `firebender.json` first, then the overlay is applied. This allows you to override MCP configurations in the overlay if needed.

### Custom Commands Support

Custom commands (also called "slash commands") allow you to define reusable prompts that can be invoked by name in supported AI agents. Commands are defined as markdown files in `ai-rules/commands/` and are generated to agent-specific locations.

**Frontmatter Fields:**

Command files support optional YAML frontmatter. The following fields are currently supported:

- `allowed-tools` - Tool restrictions for the command (Claude-specific)
- `description` - Human-readable description of what the command does
- `model` - Specific model to use for this command (Claude-specific)

**Agent Behavior:**

| Agent | Output Location | Frontmatter Handling |
|-------|----------------|---------------------|
| **AMP** | `.agents/commands/{name}-ai-rules.md` | ❌ Stripped - AMP doesn't use YAML frontmatter |
| **Claude Code** | `.claude/commands/ai-rules/*.md` | ✅ Preserved - Claude uses frontmatter for tool restrictions and model selection |
| **Cursor** | `.cursor/commands/ai-rules/*.md` | ❌ Stripped - Cursor doesn't use YAML frontmatter |
| **Firebender** | `firebender.json` (commands array) | ❌ Stripped - Command paths embedded in JSON config |

**Documentation:**
- [Claude Code Slash Commands](https://code.claude.com/docs/en/slash-commands)
- [Cursor Commands](https://cursor.com/docs/agent/chat/commands)
- [Firebender Commands](https://docs.firebender.com/context/commands)

### Claude Code Skills Support

Claude Code supports optional rules through [skills](https://docs.claude.com/en/docs/claude-code/skills) (requires `use_claude_skills: true` in config). When enabled and a source rule has `alwaysApply: false`, the tool generates:
- **CLAUDE.md** - References required rules (`alwaysApply: true`) only
- **.claude/skills/{rule-name}/SKILL.md** - Individual skill files for optional rules

This allows Claude to selectively apply rules based on file context and user preferences. See [Experimental Options](#experimental-options) for configuration.

**Example:**
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

**Generates:**
- `CLAUDE.md` - Contains only required rules
- `.claude/skills/testing/SKILL.md` - Skill that Claude can invoke when working with test files

**Note:** Skills use the `description` field as the skill name for better discoverability. The directory name uses the source file's base filename.

### User-Defined Skills

You can define custom skill folders that are symlinked to supported agents' skill directories during generation. This allows you to create reusable skills that contain multiple files or complex structures.

**Setup:**

Create skill folders in `ai-rules/skills/<skill-name>/` with a `SKILL.md` file:

```
ai-rules/
├── skills/
│   └── debugging/
│       └── SKILL.md    # Required - defines the skill
```

**SKILL.md Format:**
```markdown
---
name: debugging
description: Debugging guidelines and best practices
---

# Debugging Guidelines

Your skill content here...
```

When you run `ai-rules generate`, symlinks are created in the agent's skills directory:
- `.agents/skills/ai-rules-generated-debugging` → `../../ai-rules/skills/debugging` (AMP)
- `.claude/skills/ai-rules-generated-debugging` → `../../ai-rules/skills/debugging` (Claude)
- `.codex/skills/ai-rules-generated-debugging` → `../../ai-rules/skills/debugging` (Codex)

**Supported Agents:** AMP, Claude Code, Codex

**Note:** Skill folders without a `SKILL.md` file are skipped with a warning.

## Project Structure

### Standard Mode

```
monorepo/
├── ai-rules/              # Global rule files
│   ├── .generated-ai-rules/  # Root processed files
│   ├── commands/         # Custom commands (slash commands)
│   │   └── commit.md     # Example command
│   ├── skills/           # User-defined skills (symlinked to agents)
│   │   └── debugging/    # Example skill folder
│   │       └── SKILL.md  # Skill definition
│   ├── general.md        # Repository-wide rules
│   └── mcp.json          # MCP server configuration
├── frontend/              # Frontend application
│   ├── ai-rules/          # Frontend-specific rules
│   │   ├── .generated-ai-rules/  # Frontend processed files
│   │   ├── react.md      # React component rules
│   │   └── styling.md    # CSS/styling rules
│   ├── CLAUDE.md          # Frontend Claude rules
│   ├── .agents/commands/  # Frontend AMP commands ({name}-ai-rules.md)
│   ├── .claude/skills/    # Frontend Claude skills (requires use_claude_skills: true)
│   ├── .claude/commands/ai-rules/  # Frontend Claude commands (*.md)
│   ├── .clinerules/       # Frontend Cline rules (*.md files)
│   ├── .cursor/rules/     # Frontend Cursor rules (*.mdc files)
│   ├── .cursor/commands/ai-rules/  # Frontend Cursor commands (*.md)
│   ├── GEMINI.md          # Frontend Gemini rules
│   ├── AGENTS.md       # Frontend Goose/AMP/Codex/Copilot rules
│   ├── .kilocode/rules/   # Frontend Kilocode rules (*.md files)
│   ├── .roo/rules/        # Frontend Roo rules (*.md files)
│   └── src/
├── backend/               # Backend services
│   ├── ai-rules/          # Backend-specific rules
│   │   ├── .generated-ai-rules/  # Backend processed files
│   │   ├── api.md        # API development rules
│   │   └── database.md   # Database schema rules
│   ├── CLAUDE.md          # Backend Claude rules
│   ├── .agents/commands/  # Backend AMP commands ({name}-ai-rules.md)
│   ├── .claude/skills/    # Backend Claude skills (requires use_claude_skills: true)
│   ├── .claude/commands/ai-rules/  # Backend Claude commands (*.md)
│   ├── .clinerules/       # Backend Cline rules (*.md files)
│   ├── .cursor/rules/     # Backend Cursor rules (*.mdc files)
│   ├── .cursor/commands/ai-rules/  # Backend Cursor commands (*.md)
│   ├── GEMINI.md          # Backend Gemini rules
│   ├── AGENTS.md       # Backend Goose/AMP/Codex/Copilot rules
│   ├── .kilocode/rules/   # Backend Kilocode rules (*.md files)
│   ├── .roo/rules/        # Backend Roo rules (*.md files)
│   └── api/
├── CLAUDE.md             # Root Claude rules
├── .agents/commands/     # Root AMP commands ({name}-ai-rules.md)
├── .claude/skills/       # Root Claude skills (requires use_claude_skills: true)
├── .claude/commands/ai-rules/  # Root Claude commands (*.md)
├── .clinerules/          # Root Cline rules (*.md files)
├── .cursor/rules/        # Root Cursor rules (*.mdc files)
├── .cursor/commands/ai-rules/  # Root Cursor commands (*.md)
├── .cursor/mcp.json      # Root Cursor MCP config
├── firebender.json       # Root Firebender rules + MCP
├── firebender-overlay.json # Root Firebender overlay
├── GEMINI.md             # Root Gemini rules
├── AGENTS.md          # Root Goose/AMP/Codex/Copilot rules
├── .kilocode/rules/      # Root Kilocode rules (*.md files)
├── .roo/rules/           # Root Roo rules (*.md files)
├── .roo/mcp.json         # Root Roo MCP config
└── .mcp.json             # Root Claude Code MCP config
```

### Symlink Mode

```
project/
├── ai-rules/
│   ├── AGENTS.md          # Source file
│   ├── commands/          # Custom commands (optional)
│   │   └── commit.md      # Example command
│   ├── skills/            # User-defined skills (optional)
│   │   └── debugging/     # Example skill folder
│   │       └── SKILL.md   # Skill definition
│   └── mcp.json           # MCP config (optional)
├── CLAUDE.md              # Symlink → ai-rules/AGENTS.md
├── GEMINI.md              # Symlink → ai-rules/AGENTS.md
├── AGENTS.md              # Symlink → ai-rules/AGENTS.md
├── firebender.json        # References ai-rules/AGENTS.md + commands
├── .agents/
│   └── commands/          # AMP commands ({name}-ai-rules.md)
├── .claude/
│   └── commands/ai-rules/ # Claude commands (*.md)
├── .clinerules/
│   └── AGENTS.md          # Symlink → ../ai-rules/AGENTS.md
├── .cursor/
│   ├── commands/ai-rules/ # Cursor commands (*.md)
│   └── mcp.json           # Cursor MCP config (if mcp.json exists)
├── .kilocode/rules/
│   └── AGENTS.md          # Symlink → ../../ai-rules/AGENTS.md
├── .roo/
│   ├── rules/
│   │   └── AGENTS.md      # Symlink → ../../ai-rules/AGENTS.md
│   └── mcp.json           # Roo MCP config (if mcp.json exists)
└── .mcp.json              # Claude Code MCP config (if mcp.json exists)
```

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

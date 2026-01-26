pub const MD_EXTENSION: &str = "md";
pub const AI_RULE_SOURCE_DIR: &str = "ai-rules";
pub const GENERATED_RULE_BODY_DIR: &str = ".generated-ai-rules";
pub const AGENTS_MD_FILENAME: &str = "AGENTS.md";
pub const AGENTS_MD_GROUP_NAME: &str = "agents-md";
pub const AGENTS_MD_AGENTS: [&str; 7] = [
    "amp", "cline", "codex", "copilot", "goose", "kilocode", "roo",
];
pub const AI_RULE_CONFIG_FILENAME: &str = "ai-rules-config.yaml";
pub const GENERATED_FILE_PREFIX: &str = "ai-rules-generated-";
pub const GENERATED_COMMAND_SUFFIX: &str = "ai-rules";

pub const CLAUDE_SKILLS_DIR: &str = ".claude/skills";
#[allow(dead_code)]
pub const CODEX_SKILLS_DIR: &str = ".codex/skills";
#[allow(dead_code)]
pub const AMP_SKILLS_DIR: &str = ".agents/skills";
pub const CURSOR_SKILLS_DIR: &str = ".cursor/skills";
pub const FIREBENDER_SKILLS_DIR: &str = ".firebender/skills";
pub const SKILL_FILENAME: &str = "SKILL.md";
pub const SKILLS_DIR: &str = "skills";

pub const FIREBENDER_JSON: &str = "firebender.json";
pub const FIREBENDER_OVERLAY_JSON: &str = "firebender-overlay.json";
pub const FIREBENDER_USE_CURSOR_RULES_FIELD: &str = "useCursorRules";

pub const MCP_JSON: &str = "mcp.json";
pub const CLAUDE_MCP_JSON: &str = ".mcp.json";
pub const MCP_SERVERS_FIELD: &str = "mcpServers";

pub const COMMANDS_DIR: &str = "commands";
pub const CLAUDE_COMMANDS_DIR: &str = ".claude/commands";
pub const CLAUDE_COMMANDS_SUBDIR: &str = "ai-rules";
pub const CURSOR_COMMANDS_DIR: &str = ".cursor/commands";
pub const CURSOR_COMMANDS_SUBDIR: &str = "ai-rules";
pub const AMP_COMMANDS_DIR: &str = ".agents/commands";

// Embedded template content (compile-time inclusion)
pub const OPTIONAL_RULES_TEMPLATE: &str = include_str!("templates/optional_rules.md");

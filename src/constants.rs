pub const MD_EXTENSION: &str = "md";
pub const AI_RULE_SOURCE_DIR: &str = "ai-rules";
pub const GENERATED_RULE_BODY_DIR: &str = ".generated-ai-rules";
pub const OPTIONAL_RULES_FILENAME: &str = "ai-rules-generated-optional.md";
pub const AGENTS_MD_FILENAME: &str = "AGENTS.md";
pub const AI_RULE_CONFIG_FILENAME: &str = "ai-rules-config.yaml";
pub const GENERATED_FILE_PREFIX: &str = "ai-rules-generated-";

pub const CLAUDE_SKILLS_DIR: &str = ".claude/skills";
pub const SKILL_FILENAME: &str = "SKILL.md";

pub const FIREBENDER_JSON: &str = "firebender.json";
pub const FIREBENDER_OVERLAY_JSON: &str = "firebender-overlay.json";
pub const FIREBENDER_USE_CURSOR_RULES_FIELD: &str = "useCursorRules";

pub const MCP_JSON: &str = "mcp.json";
pub const CLAUDE_MCP_JSON: &str = ".mcp.json";
pub const MCP_SERVERS_FIELD: &str = "mcpServers";

#[allow(dead_code)]
pub const COMMANDS_DIR: &str = "commands";
#[allow(dead_code)]
pub const CLAUDE_COMMANDS_DIR: &str = ".claude/commands";
#[allow(dead_code)]
pub const CURSOR_COMMANDS_DIR: &str = ".cursor/commands";
pub const AMP_COMMANDS_DIR: &str = ".agents/commands";
#[allow(dead_code)]
pub const FIREBENDER_COMMANDS_FIELD: &str = "commands";

// Embedded template content (compile-time inclusion)
pub const OPTIONAL_RULES_TEMPLATE: &str = include_str!("templates/optional_rules.md");

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ai-rules",
    about = format!("AI Rules Tool - {}", super::SUMMARY),
    version
)]
pub struct Cli {
    #[arg(long)]
    pub summary: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Args, Default, Clone)]
pub struct InitArgs {
    #[arg(long = "params", value_name = "key=value")]
    pub params: Vec<String>,
    #[arg(long, help = "Skip confirmation prompts and assume yes")]
    pub force: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize AI rules in the current directory
    Init(InitArgs),
    /// Generate AI rules for tools
    Generate(GenerateArgs),
    /// Show status of AI rules (i.e. if they are in sync)
    Status(StatusArgs),
    /// Clean up generated files
    Clean(CleanArgs),
    /// List all supported coding agents
    ListAgents,
}

#[derive(Args)]
#[command(after_help = "Examples:
  ai-rules generate                           # Generate using config file settings (or all default values if no config file)
  ai-rules generate --agents claude,cursor    # Generate for specific agents only
  ai-rules generate --agents claude,cursor --nested-depth 5        # Specific agents in nested directories

Configuration Precedence (highest to lowest):
  1. CLI options (--agents, --nested-depth, --gitignore)
  2. Config file: ai-rules/ai-rules-config.yaml
  3. Default values (all agents, depth 0, generated files are NOT git ignored)

💡 Tip: Run 'ai-rules status' first to check sync status")]
pub struct GenerateArgs {
    #[arg(
        long,
        value_delimiter = ',',
        help = "Comma-separated list of agents to generate rules for"
    )]
    pub agents: Option<Vec<String>>,
    #[arg(long, help = "Add generated file patterns to .gitignore")]
    pub gitignore: bool,
    #[arg(
        long,
        help = "DEPRECATED: Use --gitignore instead. Skip updating .gitignore with generated file patterns"
    )]
    pub no_gitignore: bool,
    #[arg(
        long,
        help = "Maximum nested directory depth to traverse (0 = current directory only)"
    )]
    pub nested_depth: Option<usize>,
    #[arg(
        long,
        help = "Do not follow symlinks when discovering markdown files (symlinks are followed by default)"
    )]
    pub no_follow_symlinks: bool,
}

#[derive(Args)]
pub struct NestedDepthArgs {
    #[arg(
        long,
        help = "Maximum nested directory depth to traverse (0 = current directory only)"
    )]
    pub nested_depth: Option<usize>,
}

#[derive(Args)]
#[command(after_help = "Examples:
  ai-rules status                             # Check status using config file settings (or default values if no config file)
  ai-rules status --agents claude,cursor     # Check status for specific agents only
  ai-rules status --nested-depth 2           # Check status in nested directories

Configuration Precedence (highest to lowest):
  1. CLI options (--agents, --nested-depth)
  2. Config file: ai-rules/ai-rules-config.yaml
  3. Default values (all agents, depth 0)")]
pub struct StatusArgs {
    #[arg(
        long,
        value_delimiter = ',',
        help = "Comma-separated list of agents to check status for"
    )]
    pub agents: Option<Vec<String>>,
    #[command(flatten)]
    pub nested_depth_args: NestedDepthArgs,
}

#[derive(Args)]
#[command(after_help = "Examples:
  ai-rules clean                              # Clean using config file settings (or default values if no config file)
  ai-rules clean --nested-depth 2            # Clean generated files in nested directories

Configuration Precedence (highest to lowest):
  1. CLI options (--nested-depth)
  2. Config file: ai-rules/ai-rules-config.yaml
  3. Default values (depth 0)")]
pub struct CleanArgs {
    #[command(flatten)]
    pub nested_depth_args: NestedDepthArgs,
}

#[derive(Debug, Clone)]
pub struct ResolvedGenerateArgs {
    pub agents: Option<Vec<String>>,
    pub command_agents: Option<Vec<String>>,
    pub gitignore: bool,
    pub nested_depth: usize,
    pub follow_symlinks: bool,
}

#[derive(Debug)]
pub struct ResolvedStatusArgs {
    pub agents: Option<Vec<String>>,
    pub command_agents: Option<Vec<String>>,
    pub nested_depth: usize,
}

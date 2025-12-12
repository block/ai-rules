use super::args::{
    GenerateArgs, NestedDepthArgs, ResolvedGenerateArgs, ResolvedStatusArgs, StatusArgs,
};
use crate::config;

fn resolve_agents(
    agents: Option<Vec<String>>,
    config: Option<&config::Config>,
) -> Option<Vec<String>> {
    agents.or_else(|| config?.agents.clone())
}

fn resolve_nested_depth(
    nested_depth: Option<usize>,
    config: Option<&config::Config>,
) -> Option<usize> {
    nested_depth.or_else(|| config?.nested_depth)
}

impl GenerateArgs {
    pub fn with_config(self, config: Option<&config::Config>) -> ResolvedGenerateArgs {
        let agents = resolve_agents(self.agents, config);
        let nested_depth = resolve_nested_depth(self.nested_depth, config);

        // Handle gitignore resolution with backward compatibility
        // Priority: CLI flags > Config file > Default (false)
        let gitignore = if self.gitignore {
            // New --gitignore flag was set
            true
        } else if self.no_gitignore {
            // Deprecated --no-gitignore flag was set, invert it
            false
        } else if let Some(config) = config {
            // Check config file - new field takes precedence
            if let Some(gitignore_value) = config.gitignore {
                gitignore_value
            } else if let Some(no_gitignore_value) = config.no_gitignore {
                // Deprecated config field, invert it
                !no_gitignore_value
            } else {
                // No config set, use default
                false
            }
        } else {
            // No CLI flags and no config, use default
            false
        };

        let auto_update_gitignore = config.and_then(|c| c.auto_update_gitignore).unwrap_or(true);

        ResolvedGenerateArgs {
            agents,
            gitignore,
            nested_depth: nested_depth.unwrap_or(0),
            auto_update_gitignore,
        }
    }
}

impl StatusArgs {
    pub fn with_config(self, config: Option<&config::Config>) -> ResolvedStatusArgs {
        let agents = resolve_agents(self.agents, config);
        let nested_depth = self.nested_depth_args.with_config(config);
        ResolvedStatusArgs {
            agents,
            nested_depth,
        }
    }
}

impl NestedDepthArgs {
    pub fn with_config(self, config: Option<&config::Config>) -> usize {
        let nested_depth = resolve_nested_depth(self.nested_depth, config);
        nested_depth.unwrap_or(0)
    }
}

#[cfg(test)]
use super::args::*;
#[cfg(test)]
use crate::config;

#[test]
fn test_generate_args_with_config_cli_priority() {
    let config = config::Config {
        agents: Some(vec!["cursor".to_string()]),
        command_agents: None,
        gitignore: Some(false),
        no_gitignore: None,
        nested_depth: Some(5),
        use_claude_skills: None,
    };

    let args = GenerateArgs {
        agents: Some(vec!["claude".to_string()]),
        gitignore: true,
        no_gitignore: false,
        nested_depth: Some(2),
    };

    let resolved = args.with_config(Some(&config));

    assert_eq!(resolved.agents, Some(vec!["claude".to_string()]));
    assert!(resolved.gitignore);
    assert_eq!(resolved.nested_depth, 2);
}

#[test]
fn test_generate_args_with_config_uses_config_when_cli_missing() {
    let config = config::Config {
        agents: Some(vec!["cursor".to_string()]),
        command_agents: None,
        gitignore: Some(true),
        no_gitignore: None,
        nested_depth: Some(3),
        use_claude_skills: None,
    };

    let args = GenerateArgs {
        agents: None,
        gitignore: false,
        no_gitignore: false,
        nested_depth: None,
    };

    let resolved = args.with_config(Some(&config));

    assert_eq!(resolved.agents, Some(vec!["cursor".to_string()]));
    assert!(resolved.gitignore);
    assert_eq!(resolved.nested_depth, 3);
}

#[test]
fn test_generate_args_with_config_defaults() {
    let args = GenerateArgs {
        agents: None,
        gitignore: false,
        no_gitignore: false,
        nested_depth: None,
    };

    let resolved = args.with_config(None);

    assert_eq!(resolved.agents, None);
    assert!(!resolved.gitignore);
    assert_eq!(resolved.nested_depth, 0);
}

#[test]
fn test_generate_args_with_config_partial_config() {
    let config = config::Config {
        agents: Some(vec!["claude".to_string()]),
        command_agents: None,
        gitignore: None,
        no_gitignore: None,
        nested_depth: None,
        use_claude_skills: None,
    };

    let args = GenerateArgs {
        agents: None,
        gitignore: false,
        no_gitignore: false,
        nested_depth: None,
    };

    let resolved = args.with_config(Some(&config));

    assert_eq!(resolved.agents, Some(vec!["claude".to_string()]));
    assert!(!resolved.gitignore);
    assert_eq!(resolved.nested_depth, 0);
}

#[test]
fn test_nested_depth_args_with_config() {
    let config = config::Config {
        agents: None,
        command_agents: None,
        gitignore: None,
        no_gitignore: None,
        nested_depth: Some(4),
        use_claude_skills: None,
    };

    let args_with_cli = NestedDepthArgs {
        nested_depth: Some(1),
    };
    assert_eq!(args_with_cli.with_config(Some(&config)), 1);

    let args_without_cli = NestedDepthArgs { nested_depth: None };
    assert_eq!(args_without_cli.with_config(Some(&config)), 4);

    let args_no_config = NestedDepthArgs { nested_depth: None };
    assert_eq!(args_no_config.with_config(None), 0);
}

#[test]
fn test_nested_depth_explicit_zero_overrides_config() {
    let config = config::Config {
        agents: None,
        command_agents: None,
        gitignore: None,
        no_gitignore: None,
        nested_depth: Some(5),
        use_claude_skills: None,
    };

    let args = NestedDepthArgs {
        nested_depth: Some(0),
    };

    assert_eq!(args.with_config(Some(&config)), 0);
}

#[test]
fn test_status_args_with_config_cli_priority() {
    let config = config::Config {
        agents: Some(vec!["cursor".to_string()]),
        command_agents: None,
        gitignore: None,
        no_gitignore: None,
        nested_depth: Some(5),
        use_claude_skills: None,
    };

    let args = StatusArgs {
        agents: Some(vec!["claude".to_string()]),
        nested_depth_args: NestedDepthArgs {
            nested_depth: Some(2),
        },
    };

    let resolved = args.with_config(Some(&config));

    assert_eq!(resolved.agents, Some(vec!["claude".to_string()]));
    assert_eq!(resolved.nested_depth, 2);
}

#[test]
fn test_status_args_with_config_uses_config_when_cli_missing() {
    let config = config::Config {
        agents: Some(vec!["cursor".to_string()]),
        command_agents: None,
        gitignore: None,
        no_gitignore: None,
        nested_depth: Some(3),
        use_claude_skills: None,
    };

    let args = StatusArgs {
        agents: None,
        nested_depth_args: NestedDepthArgs { nested_depth: None },
    };

    let resolved = args.with_config(Some(&config));

    assert_eq!(resolved.agents, Some(vec!["cursor".to_string()]));
    assert_eq!(resolved.nested_depth, 3);
}

#[test]
fn test_status_args_with_config_defaults() {
    let args = StatusArgs {
        agents: None,
        nested_depth_args: NestedDepthArgs { nested_depth: None },
    };

    let resolved = args.with_config(None);

    assert_eq!(resolved.agents, None);
    assert_eq!(resolved.nested_depth, 0);
}

#[test]
fn test_generate_args_backward_compat_no_gitignore_config() {
    let config = config::Config {
        agents: None,
        command_agents: None,
        gitignore: None,
        no_gitignore: Some(true),
        nested_depth: None,
        use_claude_skills: None,
    };

    let args = GenerateArgs {
        agents: None,
        gitignore: false,
        no_gitignore: false,
        nested_depth: None,
    };

    let resolved = args.with_config(Some(&config));

    assert!(!resolved.gitignore);
}

#[test]
fn test_generate_args_backward_compat_no_gitignore_cli() {
    let config = config::Config {
        agents: None,
        command_agents: None,
        gitignore: Some(true),
        no_gitignore: None,
        nested_depth: None,
        use_claude_skills: None,
    };

    let args = GenerateArgs {
        agents: None,
        gitignore: false,
        no_gitignore: true,
        nested_depth: None,
    };

    let resolved = args.with_config(Some(&config));

    assert!(!resolved.gitignore);
}

#[test]
fn test_generate_args_new_gitignore_flag_overrides_old() {
    let config = config::Config {
        agents: None,
        command_agents: None,
        gitignore: None,
        no_gitignore: None,
        nested_depth: None,
        use_claude_skills: None,
    };

    let args = GenerateArgs {
        agents: None,
        gitignore: true,
        no_gitignore: true,
        nested_depth: None,
    };

    let resolved = args.with_config(Some(&config));

    assert!(resolved.gitignore);
}

use crate::agents::AgentToolRegistry;

pub fn run_list_agents(use_claude_skills: bool, use_cursor_rules: bool) -> anyhow::Result<()> {
    let registry = AgentToolRegistry::new(use_claude_skills, use_cursor_rules);
    let mut agent_names = registry.get_all_tool_names();
    agent_names.sort();

    println!("Supported agents:");
    for agent_name in agent_names {
        println!("  • {agent_name}");
    }

    Ok(())
}

use crate::agents::AgentToolRegistry;

pub fn run_list_agents() -> anyhow::Result<()> {
    let registry = AgentToolRegistry::new();
    let mut agent_names = registry.get_all_tool_names();
    agent_names.sort();

    println!("Supported agents:");
    for agent_name in agent_names {
        println!("  • {agent_name}");
    }

    Ok(())
}

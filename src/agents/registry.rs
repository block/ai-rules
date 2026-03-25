use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::{
    amp::AmpGenerator, claude::ClaudeGenerator, codex::CodexGenerator, cursor::CursorGenerator,
    firebender::FirebenderGenerator, gemini::GeminiGenerator, roo::RooGenerator,
    single_file_based::SingleFileBasedGenerator,
};
use crate::constants::{
    AGENTS_MD_FILENAME, AMP_GLOBAL_OUTPUT_FILE, CLAUDE_GLOBAL_OUTPUT_FILE, CLAUDE_OUTPUT_FILE,
    CODEX_GLOBAL_OUTPUT_FILE, FIREBENDER_GLOBAL_JSON, FIREBENDER_JSON, GEMINI_GLOBAL_OUTPUT_FILE,
    GEMINI_OUTPUT_FILE,
};
use std::collections::HashMap;

pub struct AgentToolRegistry {
    tools: HashMap<String, Box<dyn AgentRuleGenerator>>,
}

impl AgentToolRegistry {
    pub fn new(use_claude_skills: bool) -> Self {
        Self::create(false, use_claude_skills)
    }

    pub fn new_global(use_claude_skills: bool) -> Self {
        Self::create(true, use_claude_skills)
    }

    fn create(global: bool, use_claude_skills: bool) -> Self {
        let mut tools: HashMap<String, Box<dyn AgentRuleGenerator>> = HashMap::new();

        let generators: Vec<Box<dyn AgentRuleGenerator>> = vec![
            Box::new(ClaudeGenerator::new(
                "claude",
                if global { CLAUDE_GLOBAL_OUTPUT_FILE } else { CLAUDE_OUTPUT_FILE },
                use_claude_skills,
                global,
            )),
            Box::new(SingleFileBasedGenerator::new("cline", AGENTS_MD_FILENAME)),
            Box::new(CursorGenerator),
            Box::new(FirebenderGenerator::new(
                if global { FIREBENDER_GLOBAL_JSON } else { FIREBENDER_JSON },
            )),
            Box::new(SingleFileBasedGenerator::new("goose", AGENTS_MD_FILENAME)),
            Box::new(AmpGenerator::new(
                if global { AMP_GLOBAL_OUTPUT_FILE } else { AGENTS_MD_FILENAME },
            )),
            Box::new(CodexGenerator::new(
                if global { CODEX_GLOBAL_OUTPUT_FILE } else { AGENTS_MD_FILENAME },
            )),
            Box::new(SingleFileBasedGenerator::new("copilot", AGENTS_MD_FILENAME)),
            Box::new(GeminiGenerator::new(
                if global { GEMINI_GLOBAL_OUTPUT_FILE } else { GEMINI_OUTPUT_FILE },
            )),
            Box::new(SingleFileBasedGenerator::new(
                "kilocode",
                AGENTS_MD_FILENAME,
            )),
            Box::new(RooGenerator::new()),
        ];

        for generator in generators {
            let name = generator.name().to_string();
            tools.insert(name, generator);
        }

        Self { tools }
    }

    pub fn get_tool(&self, name: &str) -> Option<&dyn AgentRuleGenerator> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn get_all_tool_names(&self) -> Vec<String> {
        self.tools.keys().map(|s| s.to_string()).collect()
    }

    pub fn filter_valid_agents(&self, agents: Vec<String>) -> Vec<String> {
        agents
            .into_iter()
            .filter(|agent| {
                if self.get_tool(agent).is_none() {
                    eprintln!("Warning: unrecognised agent '{agent}', skipping");
                    false
                } else {
                    true
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::CLAUDE_MCP_JSON;

    #[test]
    fn test_new_global_uses_global_output_paths() {
        let registry = AgentToolRegistry::new_global(false);

        let cases = [
            ("claude", CLAUDE_GLOBAL_OUTPUT_FILE),
            ("gemini", GEMINI_GLOBAL_OUTPUT_FILE),
            ("codex", CODEX_GLOBAL_OUTPUT_FILE),
            ("amp", AMP_GLOBAL_OUTPUT_FILE),
            ("firebender", FIREBENDER_GLOBAL_JSON),
        ];

        for (agent, expected_path) in cases {
            let tool = registry.get_tool(agent).unwrap();
            let patterns = tool.gitignore_patterns();
            assert!(
                patterns.iter().any(|p| p == expected_path),
                "{agent} global registry should use {expected_path}"
            );
        }
    }

    #[test]
    fn test_new_global_claude_uses_global_mcp_generator() {
        let registry = AgentToolRegistry::new_global(false);
        let tool = registry.get_tool("claude").unwrap();
        let patterns = tool.mcp_generator().unwrap().mcp_gitignore_patterns();
        assert!(patterns.is_empty(), "global claude MCP should not produce gitignore patterns");
    }

    #[test]
    fn test_new_project_claude_uses_dot_mcp_json() {
        let registry = AgentToolRegistry::new(false);
        let tool = registry.get_tool("claude").unwrap();
        let patterns = tool.mcp_generator().unwrap().mcp_gitignore_patterns();
        assert!(
            patterns.iter().any(|p| p == CLAUDE_MCP_JSON),
            "project claude MCP should use {CLAUDE_MCP_JSON}"
        );
    }

    #[test]
    fn test_filter_valid_agents_empty_input() {
        let registry = AgentToolRegistry::new(false);
        let result = registry.filter_valid_agents(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_valid_agents_keeps_known_agents() {
        let registry = AgentToolRegistry::new(false);
        let result = registry.filter_valid_agents(vec!["claude".to_string(), "cursor".to_string()]);
        assert_eq!(result, vec!["claude".to_string(), "cursor".to_string()]);
    }

    #[test]
    fn test_filter_valid_agents_removes_unknown_agents() {
        let registry = AgentToolRegistry::new(false);
        let result =
            registry.filter_valid_agents(vec!["claude".to_string(), "nonexistent".to_string()]);
        assert_eq!(result, vec!["claude".to_string()]);
    }

    #[test]
    fn test_filter_valid_agents_returns_empty_for_all_unknown() {
        let registry = AgentToolRegistry::new(false);
        let result = registry.filter_valid_agents(vec!["unknown1".to_string(), "unknown2".to_string()]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_new_uses_project_output_paths() {
        let registry = AgentToolRegistry::new(false);

        let cases = [
            ("claude", CLAUDE_OUTPUT_FILE),
            ("gemini", GEMINI_OUTPUT_FILE),
            ("codex", AGENTS_MD_FILENAME),
            ("amp", AGENTS_MD_FILENAME),
            ("firebender", FIREBENDER_JSON),
        ];

        for (agent, expected_path) in cases {
            let tool = registry.get_tool(agent).unwrap();
            let patterns = tool.gitignore_patterns();
            assert!(
                patterns.iter().any(|p| p == expected_path),
                "{agent} default registry should use {expected_path}"
            );
        }
    }
}

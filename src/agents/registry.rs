use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::{
    amp::AmpGenerator, claude::ClaudeGenerator, codex::CodexGenerator, cursor::CursorGenerator,
    firebender::FirebenderGenerator, gemini::GeminiGenerator, roo::RooGenerator,
    single_file_based::SingleFileBasedGenerator,
};
use crate::constants::{AGENTS_MD_FILENAME, CLAUDE_GLOBAL_OUTPUT_FILE, CLAUDE_OUTPUT_FILE};
use std::collections::HashMap;

pub struct AgentToolRegistry {
    tools: HashMap<String, Box<dyn AgentRuleGenerator>>,
}

impl AgentToolRegistry {
    pub fn new(use_claude_skills: bool) -> Self {
        Self::create(CLAUDE_OUTPUT_FILE, use_claude_skills)
    }

    pub fn new_global(use_claude_skills: bool) -> Self {
        Self::create(CLAUDE_GLOBAL_OUTPUT_FILE, use_claude_skills)
    }

    fn create(claude_output_filename: &str, use_claude_skills: bool) -> Self {
        let mut tools: HashMap<String, Box<dyn AgentRuleGenerator>> = HashMap::new();

        let generators: Vec<Box<dyn AgentRuleGenerator>> = vec![
            Box::new(ClaudeGenerator::new(
                "claude",
                claude_output_filename,
                use_claude_skills,
            )),
            Box::new(SingleFileBasedGenerator::new("cline", AGENTS_MD_FILENAME)),
            Box::new(CursorGenerator),
            Box::new(FirebenderGenerator),
            Box::new(SingleFileBasedGenerator::new("goose", AGENTS_MD_FILENAME)),
            Box::new(AmpGenerator),
            Box::new(CodexGenerator::new()),
            Box::new(SingleFileBasedGenerator::new("copilot", AGENTS_MD_FILENAME)),
            Box::new(GeminiGenerator),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_global_uses_dot_claude_output_path() {
        let registry = AgentToolRegistry::new_global(false);
        let tool = registry.get_tool("claude").unwrap();
        let patterns = tool.gitignore_patterns();

        assert!(
            patterns.iter().any(|p| p == CLAUDE_GLOBAL_OUTPUT_FILE),
            "global registry should use {CLAUDE_GLOBAL_OUTPUT_FILE} as claude output path"
        );
        assert!(
            !patterns.iter().any(|p| p == CLAUDE_OUTPUT_FILE),
            "global registry should not use root {CLAUDE_OUTPUT_FILE}"
        );
    }

    #[test]
    fn test_new_uses_root_claude_output_path() {
        let registry = AgentToolRegistry::new(false);
        let tool = registry.get_tool("claude").unwrap();
        let patterns = tool.gitignore_patterns();

        assert!(
            patterns.iter().any(|p| p == CLAUDE_OUTPUT_FILE),
            "default registry should use root {CLAUDE_OUTPUT_FILE} as claude output path"
        );
    }
}

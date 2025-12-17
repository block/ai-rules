use crate::agents::rule_generator::AgentRuleGenerator;
use crate::agents::{
    amp::AmpGenerator, claude::ClaudeGenerator, codex::CodexGenerator, cursor::CursorGenerator,
    firebender::FirebenderGenerator, gemini::GeminiGenerator, roo::RooGenerator,
    single_file_based::SingleFileBasedGenerator,
};
use crate::constants::AGENTS_MD_FILENAME;
use std::collections::HashMap;

pub struct AgentToolRegistry {
    tools: HashMap<String, Box<dyn AgentRuleGenerator>>,
}

impl AgentToolRegistry {
    pub fn new(use_claude_skills: bool) -> Self {
        let mut tools: HashMap<String, Box<dyn AgentRuleGenerator>> = HashMap::new();

        // Claude now always uses ClaudeGenerator with skills_mode parameter
        let claude_generator: Box<dyn AgentRuleGenerator> = Box::new(ClaudeGenerator::new(
            "claude",
            "CLAUDE.md",
            use_claude_skills,
        ));

        let generators: Vec<Box<dyn AgentRuleGenerator>> = vec![
            claude_generator,
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

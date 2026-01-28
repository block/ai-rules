pub mod body_generator;
pub mod claude_skills;
pub mod cleaner;
pub mod command_reader;
pub mod generation_result;
pub mod gitignore_updater;
pub mod legacy_cleaner;
pub mod mcp_reader;
pub mod optional_rules;
pub mod skills_reader;
pub mod source_reader;

pub use body_generator::{
    generate_all_rule_references_for_agent, generate_body_contents,
    generate_optional_rule_files_for_agents, generate_required_rule_references,
};
pub use cleaner::clean_generated_files;
#[allow(unused_imports)]
pub use command_reader::{find_command_files, CommandFile};
pub use generation_result::GenerationResult;
pub use gitignore_updater::{remove_gitignore_section, update_project_gitignore};
#[allow(unused_imports)]
pub use legacy_cleaner::clean_legacy_agent_directories;
#[allow(unused_imports)]
pub use skills_reader::{
    check_skill_symlinks_in_sync, create_skill_symlinks, find_skill_folders,
    get_skill_gitignore_patterns, remove_generated_skill_symlinks, SkillFolder,
};
pub use source_reader::find_source_files;

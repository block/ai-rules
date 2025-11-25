use anyhow::Result;
use cliclack::{confirm, input};

pub fn prompt_yes_no(prompt: &str) -> Result<bool> {
    let mut confirm_prompt = confirm(prompt.to_string()).initial_value(false);
    let answer = confirm_prompt.interact()?;
    Ok(answer)
}

pub fn prompt_rule_name(prompt: &str) -> Result<String> {
    let mut name_prompt = input(prompt)
        .placeholder("example.md")
        .validate(|input: &String| {
            let trimmed = input.trim();
            if trimmed.is_empty() {
                Err("name cannot be empty".to_string())
            } else if !trimmed.ends_with(".md") {
                Err("file name must end with .md".to_string())
            } else {
                Ok(())
            }
        });
    let value: String = name_prompt.interact()?;
    Ok(value.trim().to_string())
}

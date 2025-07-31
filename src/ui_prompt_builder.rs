use colored::*;
use std::path;

pub struct PromptBuilder;

impl PromptBuilder {
    /// Creates a formatted shell prompt with current directory
    pub fn create_shell_prompt() -> Result<String, std::io::Error> {
        let current_dir = std::env::current_dir()?.display().to_string();
        let formatted_path = current_dir
            .split(path::MAIN_SEPARATOR)
            .map(|s| s.bright_cyan().to_string())
            .collect::<Vec<_>>()
            .join(&format!(
                "{}",
                path::MAIN_SEPARATOR.to_string().bright_black()
            ));

        Ok(format!("→ {} {} ", formatted_path, "NexSh →".green()))
    }

    /// Creates a confirmation prompt for dangerous commands
    pub fn create_danger_confirmation() -> String {
        " Execute potentially dangerous command? [y/N]: "
            .red()
            .to_string()
    }

    /// Creates a simple confirmation prompt
    pub fn create_simple_confirmation() -> String {
        "? Execute? [y/N]: ".red().to_string()
    }
}

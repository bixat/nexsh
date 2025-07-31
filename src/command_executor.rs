use crate::{types::CommandResult, ui_progress::ProgressManager};
use colored::*;
use std::{
    error::Error,
    io::{self, Write},
    process::Command,
};

pub struct CommandExecutor {
    progress_manager: ProgressManager,
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            progress_manager: ProgressManager::new(),
        }
    }

    pub fn execute(&self, command: &str) -> Result<CommandResult, Box<dyn Error>> {
        let pb = self
            .progress_manager
            .create_spinner("Running command...".green().to_string());

        let result = self.run_shell_command(command);
        pb.finish_and_clear();

        match result {
            Ok(output) => Ok(CommandResult::Success(output)),
            Err(e) => {
                let error_msg = format!("Command failed: {}", e);
                println!("{} {}", "⚠️ Command failed:".red(), command.yellow());
                Ok(CommandResult::Error(error_msg))
            }
        }
    }

    fn run_shell_command(&self, command: &str) -> Result<String, Box<dyn Error>> {
        let (program, args) = self.get_shell_command(command);
        let output = Command::new(program).args(args).output()?;

        io::stdout().write_all(&output.stdout)?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            println!("{} {}", "Exit code:".red(), exit_code.to_string().yellow());

            let error_message = format!("Command failed with exit code: {}", exit_code);
            return Err(error_message.into());
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    #[cfg(target_os = "windows")]
    fn get_shell_command<'a>(&self, command: &'a str) -> (&'a str, Vec<&'a str>) {
        ("cmd", vec!["/C", command])
    }

    #[cfg(not(target_os = "windows"))]
    fn get_shell_command<'a>(&self, command: &'a str) -> (&'a str, Vec<&'a str>) {
        ("sh", vec!["-c", command])
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

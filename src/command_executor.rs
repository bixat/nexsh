use crate::{tools::ToolDispatcher, types::CommandResult, ui_progress::ProgressManager};
use colored::*;
use std::{
    error::Error,
    io::{self, Write},
    process::Command,
    time::Instant,
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
        // Strip "run " prefix if present
        let actual_command = if command.trim().starts_with("run ") {
            command.trim().strip_prefix("run ").unwrap().trim()
        } else {
            command
        };

        // Check if this is a tool call (starts with a known tool name)
        if let Some(tool_result) = self.try_execute_tool(actual_command) {
            return Ok(tool_result);
        }

        // Otherwise, execute as shell command
        let pb = self.progress_manager.create_spinner(
            format!("Running: {}...", Self::truncate_command(actual_command, 50))
                .green()
                .to_string(),
        );

        let start = Instant::now();
        let result = self.run_shell_command(actual_command);
        let elapsed = start.elapsed();

        pb.finish_and_clear();

        match result {
            Ok((stdout, stderr)) => {
                // Combine stdout and stderr for complete output
                let mut output = stdout;
                if !stderr.is_empty() && !output.contains(&stderr) {
                    if !output.is_empty() {
                        output.push_str("\n[stderr]: ");
                    }
                    output.push_str(&stderr);
                }

                if elapsed.as_secs() > 5 {
                    println!(
                        "{} {:.2}s",
                        "⏱️ Completed in:".dimmed(),
                        elapsed.as_secs_f64()
                    );
                }

                Ok(CommandResult::Success(output))
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                println!(
                    "{} {}",
                    "⚠️ Command failed:".red(),
                    Self::truncate_command(actual_command, 80).yellow()
                );
                Ok(CommandResult::Error(error_msg))
            }
        }
    }

    /// Try to execute as a built-in tool
    /// Returns Some(result) if it's a tool call, None if it should be treated as shell command
    fn try_execute_tool(&self, command: &str) -> Option<CommandResult> {
        // List of known tools (excluding 'run' which is handled specially)
        let known_tools = [
            "read_file",
            "write_file",
            "list_files",
            "delete_file",
            "create_directory",
            "find_files",
            "copy_file",
            "move_file",
            "file_info",
        ];

        // Parse command to extract tool name and arguments
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let tool_name = parts[0];

        // Special handling for 'run' tool - extract the actual command and return None
        // so it gets executed as a shell command
        if tool_name == "run" {
            // The rest of the command after 'run' should be executed as shell
            return None; // Let it fall through to shell execution
        }

        // Check if this is a known tool
        if !known_tools.contains(&tool_name) {
            return None;
        }

        // Show progress
        let pb = self.progress_manager.create_spinner(
            format!("🔧 Running tool: {}...", tool_name)
                .cyan()
                .to_string(),
        );

        // Special handling for write_file - everything after the path is content
        let args: Vec<String> = if tool_name == "write_file" {
            self.parse_write_file_arguments(command)
        } else {
            self.parse_tool_arguments(&parts[1..])
        };

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        // Execute the tool
        let result = ToolDispatcher::execute(tool_name, &arg_refs);

        pb.finish_and_clear();

        // Convert ToolResult to CommandResult
        Some(if result.success {
            println!(
                "{} {} -> {}",
                "✓".green(),
                tool_name.cyan(),
                arg_refs.join(" ")
            );
            CommandResult::Success(result.output)
        } else {
            println!(
                "{} {} - {}",
                "⚠️ Tool failed:".red(),
                tool_name.yellow(),
                result.error.as_deref().unwrap_or("Unknown error")
            );
            CommandResult::Error(
                result
                    .error
                    .unwrap_or_else(|| "Tool execution failed".to_string()),
            )
        })
    }

    /// Parse write_file arguments specially: path and everything else as content
    /// Supports: write_file <path> <content> or write_file <path> "<content>"
    fn parse_write_file_arguments(&self, command: &str) -> Vec<String> {
        // Remove "write_file " prefix
        let args_str = command.trim_start_matches("write_file").trim();

        if args_str.is_empty() {
            return Vec::new();
        }

        // Find the first space to separate path from content
        // Handle quoted paths
        let (path, content) = if args_str.starts_with('"') {
            // Path is quoted
            if let Some(end_quote_idx) = args_str[1..].find('"') {
                let path = &args_str[1..end_quote_idx + 1];
                let content = args_str[end_quote_idx + 2..].trim();
                (path.to_string(), content.to_string())
            } else {
                // Malformed quoted path
                return vec![args_str.to_string()];
            }
        } else {
            // Path is not quoted, find first space
            if let Some(space_idx) = args_str.find(char::is_whitespace) {
                let path = &args_str[..space_idx];
                let content = args_str[space_idx..].trim();
                (path.to_string(), content.to_string())
            } else {
                // Only path provided, no content
                return vec![args_str.to_string()];
            }
        };

        // Handle quoted content
        let final_content =
            if content.starts_with('"') && content.ends_with('"') && content.len() > 1 {
                // Remove surrounding quotes
                content[1..content.len() - 1].to_string()
            } else {
                content
            };

        vec![path, final_content]
    }

    /// Parse tool arguments, handling quoted strings
    fn parse_tool_arguments(&self, parts: &[&str]) -> Vec<String> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut in_quotes = false;

        for part in parts {
            if part.starts_with('"') && part.ends_with('"') && part.len() > 1 {
                // Complete quoted string in one part
                args.push(part[1..part.len() - 1].to_string());
            } else if part.starts_with('"') {
                // Start of quoted string
                in_quotes = true;
                current_arg = part[1..].to_string();
            } else if part.ends_with('"') && in_quotes {
                // End of quoted string
                current_arg.push(' ');
                current_arg.push_str(&part[..part.len() - 1]);
                args.push(current_arg.clone());
                current_arg.clear();
                in_quotes = false;
            } else if in_quotes {
                // Middle of quoted string
                if !current_arg.is_empty() {
                    current_arg.push(' ');
                }
                current_arg.push_str(part);
            } else {
                // Unquoted argument
                args.push(part.to_string());
            }
        }

        // Handle unclosed quotes
        if !current_arg.is_empty() {
            args.push(current_arg);
        }

        args
    }

    fn run_shell_command(&self, command: &str) -> Result<(String, String), Box<dyn Error>> {
        let (program, args) = self.get_shell_command(command);

        // Set up the command with proper environment
        let output = Command::new(program)
            .args(args)
            .env("TERM", "dumb") // Disable terminal colors in output
            .env("NO_COLOR", "1") // Modern way to disable colors
            .output()?;

        // Print stdout in real-time
        io::stdout().write_all(&output.stdout)?;

        // Capture both stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);

            // Build comprehensive error message
            let mut error_msg = format!("Exit code: {}", exit_code);
            if !stderr.is_empty() {
                error_msg.push_str(&format!("\nError output: {}", stderr.trim()));
            }

            // Try to provide helpful context for common errors
            let helpful_hint = Self::get_error_hint(&stderr, command);
            if let Some(hint) = helpful_hint {
                error_msg.push_str(&format!("\nHint: {}", hint));
            }

            return Err(error_msg.into());
        }

        Ok((stdout, stderr))
    }

    /// Provide helpful hints for common errors
    fn get_error_hint(stderr: &str, command: &str) -> Option<String> {
        let stderr_lower = stderr.to_lowercase();
        let cmd_lower = command.to_lowercase();

        if stderr_lower.contains("command not found") || stderr_lower.contains("not found") {
            // Extract the command that wasn't found
            if let Some(cmd) = command.split_whitespace().next() {
                return Some(format!(
                    "'{}' may not be installed. Try installing it first.",
                    cmd
                ));
            }
        }

        if stderr_lower.contains("permission denied") {
            return Some(
                "Permission denied. You may need to use 'sudo' or check file permissions."
                    .to_string(),
            );
        }

        if stderr_lower.contains("no such file or directory") {
            return Some(
                "The specified path doesn't exist. Check if the path is correct.".to_string(),
            );
        }

        if stderr_lower.contains("connection refused") || stderr_lower.contains("could not resolve")
        {
            return Some(
                "Network issue. Check your connection or if the service is running.".to_string(),
            );
        }

        if cmd_lower.contains("git") && stderr_lower.contains("not a git repository") {
            return Some(
                "This directory is not a git repository. Use 'git init' to initialize one."
                    .to_string(),
            );
        }

        None
    }

    /// Truncate command for display
    fn truncate_command(command: &str, max_len: usize) -> String {
        if command.len() <= max_len {
            command.to_string()
        } else {
            format!("{}...", &command[..max_len - 3])
        }
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

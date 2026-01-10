//! Built-in tools for common operations
//! These are actual Rust implementations, not shell command wrappers

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
        }
    }
}

/// File system tools - actual Rust implementations
pub struct FileTools;

impl FileTools {
    /// Read file contents
    pub fn read_file(path: &str) -> ToolResult {
        match fs::read_to_string(path) {
            Ok(content) => ToolResult::success(content),
            Err(e) => ToolResult::error(format!("Failed to read file: {}", e)),
        }
    }

    /// Write content to file
    pub fn write_file(path: &str, content: &str) -> ToolResult {
        match fs::write(path, content) {
            Ok(_) => ToolResult::success(format!("Successfully wrote to {}", path)),
            Err(e) => ToolResult::error(format!("Failed to write file: {}", e)),
        }
    }

    /// List files in directory
    pub fn list_files(path: &str) -> ToolResult {
        let dir_path = if path.is_empty() { "." } else { path };

        match fs::read_dir(dir_path) {
            Ok(entries) => {
                let mut output = String::new();
                let mut files: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                files.sort_by_key(|e| e.file_name());

                for entry in files {
                    if let Ok(metadata) = entry.metadata() {
                        let file_type = if metadata.is_dir() { "DIR " } else { "FILE" };
                        let size = metadata.len();
                        let name = entry.file_name();
                        output.push_str(&format!(
                            "{} {:>10} {}\n",
                            file_type,
                            size,
                            name.to_string_lossy()
                        ));
                    }
                }
                ToolResult::success(output)
            }
            Err(e) => ToolResult::error(format!("Failed to list directory: {}", e)),
        }
    }

    /// Delete file
    pub fn delete_file(path: &str) -> ToolResult {
        match fs::remove_file(path) {
            Ok(_) => ToolResult::success(format!("Deleted {}", path)),
            Err(e) => ToolResult::error(format!("Failed to delete file: {}", e)),
        }
    }

    /// Create directory
    pub fn create_directory(path: &str) -> ToolResult {
        match fs::create_dir_all(path) {
            Ok(_) => ToolResult::success(format!("Created directory {}", path)),
            Err(e) => ToolResult::error(format!("Failed to create directory: {}", e)),
        }
    }

    /// Search for files by name pattern (recursive)
    pub fn find_files(dir: &str, pattern: &str) -> ToolResult {
        fn search_recursive(
            path: &Path,
            pattern: &str,
            results: &mut Vec<PathBuf>,
        ) -> io::Result<()> {
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let path = entry.path();

                    if let Some(name) = path.file_name() {
                        if name.to_string_lossy().contains(pattern) {
                            results.push(path.clone());
                        }
                    }

                    if path.is_dir() {
                        let _ = search_recursive(&path, pattern, results);
                    }
                }
            }
            Ok(())
        }

        let mut results = Vec::new();
        let search_path = Path::new(if dir.is_empty() { "." } else { dir });

        match search_recursive(search_path, pattern, &mut results) {
            Ok(_) => {
                if results.is_empty() {
                    ToolResult::success("No files found matching pattern".to_string())
                } else {
                    let output = results
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    ToolResult::success(output)
                }
            }
            Err(e) => ToolResult::error(format!("Search failed: {}", e)),
        }
    }

    /// Copy file
    pub fn copy_file(from: &str, to: &str) -> ToolResult {
        match fs::copy(from, to) {
            Ok(bytes) => {
                ToolResult::success(format!("Copied {} bytes from {} to {}", bytes, from, to))
            }
            Err(e) => ToolResult::error(format!("Failed to copy file: {}", e)),
        }
    }

    /// Move/rename file
    pub fn move_file(from: &str, to: &str) -> ToolResult {
        match fs::rename(from, to) {
            Ok(_) => ToolResult::success(format!("Moved {} to {}", from, to)),
            Err(e) => ToolResult::error(format!("Failed to move file: {}", e)),
        }
    }

    /// Get file info
    pub fn file_info(path: &str) -> ToolResult {
        match fs::metadata(path) {
            Ok(metadata) => {
                let mut info = String::new();
                info.push_str(&format!("Path: {}\n", path));
                info.push_str(&format!(
                    "Type: {}\n",
                    if metadata.is_dir() {
                        "Directory"
                    } else {
                        "File"
                    }
                ));
                info.push_str(&format!("Size: {} bytes\n", metadata.len()));
                info.push_str(&format!(
                    "Read-only: {}\n",
                    metadata.permissions().readonly()
                ));
                if let Ok(modified) = metadata.modified() {
                    info.push_str(&format!("Modified: {:?}\n", modified));
                }
                ToolResult::success(info)
            }
            Err(e) => ToolResult::error(format!("Failed to get file info: {}", e)),
        }
    }
}

/// Shell command tools
pub struct ShellTools;

impl ShellTools {
    /// Execute a shell command
    /// Note: This is a marker - actual execution happens in CommandExecutor
    /// This tool exists to make it explicit when running shell commands
    pub fn run_command(command: &str) -> ToolResult {
        // This is handled by CommandExecutor, but we provide a marker result
        ToolResult::success(format!("SHELL_COMMAND: {}", command))
    }
}

/// Tool dispatcher - routes tool calls to appropriate implementations
pub struct ToolDispatcher;

impl ToolDispatcher {
    /// Execute a tool by name with arguments
    pub fn execute(tool_name: &str, args: &[&str]) -> ToolResult {
        match tool_name {
            "read_file" => {
                if args.is_empty() {
                    return ToolResult::error("read_file requires a path argument".to_string());
                }
                FileTools::read_file(args[0])
            }
            "write_file" => {
                if args.len() < 2 {
                    return ToolResult::error(
                        "write_file requires path and content arguments".to_string(),
                    );
                }
                FileTools::write_file(args[0], args[1])
            }
            "list_files" => {
                let path = args.first().copied().unwrap_or(".");
                FileTools::list_files(path)
            }
            "delete_file" => {
                if args.is_empty() {
                    return ToolResult::error("delete_file requires a path argument".to_string());
                }
                FileTools::delete_file(args[0])
            }
            "create_directory" => {
                if args.is_empty() {
                    return ToolResult::error(
                        "create_directory requires a path argument".to_string(),
                    );
                }
                FileTools::create_directory(args[0])
            }
            "find_files" => {
                let dir = args.first().copied().unwrap_or(".");
                let pattern = args.get(1).copied().unwrap_or("");
                if pattern.is_empty() {
                    return ToolResult::error("find_files requires a pattern argument".to_string());
                }
                FileTools::find_files(dir, pattern)
            }
            "copy_file" => {
                if args.len() < 2 {
                    return ToolResult::error(
                        "copy_file requires from and to arguments".to_string(),
                    );
                }
                FileTools::copy_file(args[0], args[1])
            }
            "move_file" => {
                if args.len() < 2 {
                    return ToolResult::error(
                        "move_file requires from and to arguments".to_string(),
                    );
                }
                FileTools::move_file(args[0], args[1])
            }
            "file_info" => {
                if args.is_empty() {
                    return ToolResult::error("file_info requires a path argument".to_string());
                }
                FileTools::file_info(args[0])
            }
            "run" => {
                // Join all args back into a command string
                let command = args.join(" ");
                if command.is_empty() {
                    return ToolResult::error("run requires a command argument".to_string());
                }
                ShellTools::run_command(&command)
            }
            _ => ToolResult::error(format!("Unknown tool: {}", tool_name)),
        }
    }

    /// Get list of available tools with descriptions
    pub fn list_tools() -> String {
        r#"Available Tools:

FILE OPERATIONS (Rust-based, safe):
- read_file <path>: Read file contents
- write_file <path> <content>: Write content to file
- list_files [path]: List files in directory (default: current dir)
- delete_file <path>: Delete a file
- create_directory <path>: Create a directory (recursive)
- find_files [dir] <pattern>: Find files matching pattern
- copy_file <from> <to>: Copy a file
- move_file <from> <to>: Move/rename a file
- file_info <path>: Get file metadata

SHELL OPERATIONS:
- run <command>: Execute any shell command
- Or just use the command directly (e.g., "ls -la", "grep pattern file.txt")

USAGE GUIDELINES:
- Use file tools for simple CRUD operations (safer, validated)
- Use shell commands for complex operations, piping, system tasks
- Both approaches work - choose based on the task complexity"#
            .to_string()
    }
}

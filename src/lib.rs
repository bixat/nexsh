use colored::*;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::error::Error;

use crate::{
    ai_client::AIClient,
    available_models::list_available_models,
    command_executor::CommandExecutor,
    config_manager::ConfigManager,
    context_manager::ContextManager,
    migration::MigrationManager,
    react_loop::ReActLoop,
    types::{Message, NexShConfig},
    ui_progress::ProgressManager,
    ui_prompt_builder::PromptBuilder,
};

// Export modules
pub mod ai_client;
pub mod available_models;
pub mod command_executor;
pub mod config_manager;
pub mod context_manager;
pub mod migration;
pub mod prompt;
pub mod react_agent;
pub mod react_loop;
pub mod tools;
pub mod types;
pub mod ui_progress;
pub mod ui_prompt_builder;

impl Default for NexShConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            history_size: 1000,
            max_context_messages: 100,
            model: Some("anthropic/claude-sonnet-4".to_string()),
            verbose: false,
            max_iterations: 10,
        }
    }
}

pub struct NexSh {
    config_manager: ConfigManager,
    ai_client: AIClient,
    command_executor: CommandExecutor,
    context_manager: ContextManager,
    progress_manager: ProgressManager,
    react_loop: ReActLoop,
    messages: Vec<Message>,
    editor: DefaultEditor,
}

impl NexSh {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Run migrations first
        let migration_manager = MigrationManager::new()?;
        if let Err(e) = migration_manager.run_migrations() {
            eprintln!("Warning: Migration failed: {}", e);
        }

        let config_manager = ConfigManager::new()?;
        let editor = config_manager.create_editor()?;

        // Initialize AI client with config (allow empty API key for first-time setup)
        let ai_client = AIClient::new(
            config_manager.config.api_key.clone(),
            config_manager.config.model.clone(),
        )?;

        // Initialize context manager
        let context_manager = ContextManager::new(
            config_manager.context_file.clone(),
            config_manager.config.max_context_messages,
        );

        // Load existing messages from context
        let messages = context_manager.load_context().unwrap_or_default();

        // Create ReActLoop with configured max_iterations
        let react_loop = ReActLoop::new(config_manager.config.max_iterations);

        Ok(Self {
            config_manager,
            ai_client,
            command_executor: CommandExecutor::new(),
            context_manager,
            progress_manager: ProgressManager::new(),
            react_loop,
            messages,
            editor,
        })
    }

    pub fn set_model(&mut self, model: &str) -> Result<(), Box<dyn Error>> {
        // Update config
        self.config_manager.update_config(|config| {
            config.model = Some(model.to_string());
        })?;

        // Update AI client
        self.ai_client.set_model(model.to_string());

        println!("✅ AI model set to: {}", model.green());
        Ok(())
    }

    pub fn show_config(&self) -> Result<(), Box<dyn Error>> {
        println!("\n{}", "⚙️  Current Configuration".cyan().bold());
        println!("{}", "─".repeat(50).dimmed());

        let config = &self.config_manager.config;

        // API Key (masked)
        let api_key_display = if config.api_key.is_empty() {
            "Not configured".red().to_string()
        } else {
            format!(
                "{}...{}",
                &config.api_key[..8.min(config.api_key.len())],
                if config.api_key.len() > 8 { "****" } else { "" }
            )
            .green()
            .to_string()
        };
        println!("  {}: {}", "API Key".cyan(), api_key_display);

        // Model
        let model_display = config.model.as_deref().unwrap_or("Not set");
        println!("  {}: {}", "Model".cyan(), model_display.green());

        // Max iterations
        println!(
            "  {}: {}",
            "Max Iterations".cyan(),
            config.max_iterations.to_string().green()
        );

        // History size
        println!(
            "  {}: {}",
            "History Size".cyan(),
            config.history_size.to_string().green()
        );

        // Max context messages
        println!(
            "  {}: {}",
            "Max Context Messages".cyan(),
            config.max_context_messages.to_string().green()
        );

        // Verbose mode
        let verbose_display = if config.verbose {
            "Enabled".green()
        } else {
            "Disabled".dimmed()
        };
        println!("  {}: {}", "Verbose Mode".cyan(), verbose_display);

        println!("{}", "─".repeat(50).dimmed());
        println!(
            "\n{}",
            "💡 Tip: Use 'set max_iterations <number>' to change max iterations".dimmed()
        );
        println!(
            "{}",
            "💡 Tip: Use 'verbose on/off' to toggle verbose mode".dimmed()
        );
        println!("{}", "💡 Tip: Use 'models' to change AI model".dimmed());
        println!();

        Ok(())
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        println!("🤖 Welcome to NexSh Setup!");

        // Get API key
        let input = self
            .editor
            .readline("Enter your OpenRouter API key (leave blank to keep current if exists): ")?;
        let api_key = input.trim();
        if !api_key.is_empty() {
            self.config_manager.update_config(|config| {
                config.api_key = api_key.to_string();
            })?;
        }

        // Get history size
        if let Ok(input) = self.editor.readline("Enter history size (default 1000): ") {
            if let Ok(size) = input.trim().parse() {
                self.config_manager.update_config(|config| {
                    config.history_size = size;
                })?;
            }
        }

        // Get max context messages
        if let Ok(input) = self
            .editor
            .readline("Enter max context messages (default 100): ")
        {
            if let Ok(size) = input.trim().parse() {
                self.config_manager.update_config(|config| {
                    config.max_context_messages = size;
                })?;
            }
        }

        // Verbose mode setting
        if let Ok(input) = self
            .editor
            .readline("Enable verbose mode to show all thoughts and actions? (y/N): ")
        {
            let enable_verbose = matches!(input.trim().to_lowercase().as_str(), "y" | "yes");
            self.config_manager.update_config(|config| {
                config.verbose = enable_verbose;
            })?;
            if enable_verbose {
                println!("{}", "✅ Verbose mode enabled".green());
            } else {
                println!("{}", "✅ Verbose mode disabled (default)".green());
            }
        }

        // Model selection using presets
        println!("\n{}", "📋 Model Selection".cyan().bold());
        println!("Choose a preset:");
        println!("  1. Free models (recommended for getting started)");
        println!("  2. Programming models (optimized for code)");
        println!("  3. Reasoning models (advanced problem-solving)");
        println!();

        let preset_choice = self
            .editor
            .readline("Select preset (1-3, default 1): ")
            .unwrap_or_default();

        let preset = match preset_choice.trim() {
            "2" => "programming",
            "3" => "reasoning",
            _ => "free", // Default to free models
        };

        let models = crate::available_models::get_preset_models(preset);

        if models.is_empty() {
            // Fallback to static list if preset returns empty
            println!("{}", "⚠️  Using fallback model list".yellow());
            let fallback_models = list_available_models();
            println!("\nAvailable AI models:");
            for (i, m) in fallback_models.iter().enumerate() {
                let display = if m.contains(":free") {
                    format!("{}. {} {}", i + 1, m, "🆓".green())
                } else {
                    format!("{}. {}", i + 1, m)
                };
                println!("  {}", display);
            }

            let input = self
                .editor
                .readline("Select AI model by number or name (default 1): ")?;
            let model = input.trim();
            let selected = if model.is_empty() {
                fallback_models[0]
            } else if let Ok(idx) = model.parse::<usize>() {
                fallback_models
                    .get(idx.saturating_sub(1))
                    .copied()
                    .unwrap_or(fallback_models[0])
            } else {
                fallback_models
                    .iter()
                    .find(|m| m.starts_with(model))
                    .copied()
                    .unwrap_or(fallback_models[0])
            };
            self.set_model(selected)?;
        } else {
            println!("\n{} {}", "📦 Preset:".cyan().bold(), preset.green());
            for (i, model) in models.iter().enumerate() {
                println!("  {}. {}", i + 1, model.cyan());
            }

            let input = self
                .editor
                .readline("Select model by number (default 1): ")?;
            let selection = input.trim();

            let selected = if selection.is_empty() {
                &models[0]
            } else if let Ok(idx) = selection.parse::<usize>() {
                models.get(idx.saturating_sub(1)).unwrap_or(&models[0])
            } else {
                &models[0]
            };

            self.set_model(selected)?;
        }

        // Reinitialize AI client with new API key
        if !self.config_manager.config.api_key.is_empty() {
            self.ai_client = AIClient::new(
                self.config_manager.config.api_key.clone(),
                self.config_manager.config.model.clone(),
            )?;
        }

        println!("✅ Configuration saved successfully!");
        Ok(())
    }

    pub async fn process_command(&mut self, input: &str) -> Result<(), Box<dyn Error>> {
        if self.config_manager.config.api_key.is_empty() {
            self.initialize()?;
        }

        // Add user message to context
        self.context_manager
            .add_message(&mut self.messages, "user", input)?;

        // Create progress indicator
        let pb = self
            .progress_manager
            .create_spinner("💭 Thinking...".cyan().to_string());

        // Run the ReAct loop (multiple iterations of Think → Act → Observe)
        let result = self
            .react_loop
            .run(
                &self.ai_client,
                &self.command_executor,
                &self.messages,
                input,
            )
            .await;

        // Finish spinner before displaying results
        pb.finish_and_clear();

        match result {
            Ok(steps) => {
                // In verbose mode, show all steps with full details
                if self.config_manager.config.verbose {
                    for (i, step) in steps.iter().enumerate() {
                        println!("\n{} {}", "🔄 Iteration".cyan().bold(), i + 1);
                        println!("{} {}", "💭 Thought:".yellow(), step.thought.reasoning);

                        if !step.action.command.is_empty() {
                            println!("{} {}", "⚡ Action:".green(), step.action.command);
                        }

                        if let Some(obs) = &step.observation {
                            if obs.success {
                                println!("{} {}", "👁️  Observation:".blue(), obs.result);
                            } else if let Some(error) = &obs.error {
                                println!("{} {}", "❌ Error:".red(), error);
                            }
                        }
                    }
                    println!("\n{} Completed {} iterations", "✅".green(), steps.len());
                } else {
                    // In non-verbose mode, show only the final thought and answer
                    if let Some(last_step) = steps.last() {
                        println!("\n{} {}", "💭".yellow(), last_step.thought.reasoning);

                        if let Some(obs) = &last_step.observation {
                            if obs.success && !obs.result.is_empty() {
                                println!("\n{} {}", "🤖".green().bold(), obs.result);
                            } else if let Some(error) = &obs.error {
                                eprintln!("\n{} {}", "❌ Error:".red(), error);
                            }
                        }
                    }
                }

                // Save all steps to context (regardless of verbose mode)
                for step in &steps {
                    // Add thought to context
                    self.context_manager.add_message(
                        &mut self.messages,
                        "assistant",
                        &format!("Thought: {}", step.thought.reasoning),
                    )?;

                    // Add action to context
                    if !step.action.command.is_empty() {
                        self.context_manager.add_message(
                            &mut self.messages,
                            "assistant",
                            &format!("Action: {}", step.action.command),
                        )?;

                        // Add command to shell history
                        self.editor.add_history_entry(&step.action.command)?;
                    }

                    // Add observation to context
                    if let Some(obs) = &step.observation {
                        if obs.success {
                            self.context_manager.add_message(
                                &mut self.messages,
                                "assistant",
                                &format!("Observation: {}", obs.result),
                            )?;
                        } else if let Some(error) = &obs.error {
                            self.context_manager.add_message(
                                &mut self.messages,
                                "assistant",
                                &format!("Error: {}", error),
                            )?;
                        }
                    }
                }
            }
            Err(e) => {
                pb.finish_and_clear();
                eprintln!("Failed to process command: {}", e);
            }
        }

        Ok(())
    }

    fn clear_context(&mut self) -> Result<(), Box<dyn Error>> {
        self.context_manager.clear_context(&mut self.messages)?;
        println!("{}", "🧹 Conversation context cleared".green());
        Ok(())
    }

    pub fn print_help(&self) -> Result<(), Box<dyn Error>> {
        println!("\n{}", "🤖 NexSh Help".cyan().bold());
        println!("{}", "─".repeat(60).dimmed());

        println!("\n{}", "Basic Commands:".green().bold());
        println!("  {} - Exit the shell", "exit/quit".cyan());
        println!("  {} - Set up your API key", "init".cyan());
        println!("  {} - Clear conversation context", "clear".cyan());
        println!("  {} - Show this help message", "help".cyan());

        println!("\n{}", "Configuration:".green().bold());
        println!("  {} - Show current configuration", "config".cyan());
        println!("  {} - Change AI model", "models".cyan());
        println!(
            "  {} - Set max ReAct iterations (default: 10)",
            "set max_iterations <number>".cyan()
        );
        println!("  {} - Enable verbose mode", "verbose on".cyan());
        println!("  {} - Disable verbose mode", "verbose off".cyan());

        println!("\n{}", "Built-in Tools:".green().bold());
        println!("  {} - Read file contents", "read_file <path>".cyan());
        println!("  {} - Write to file", "write_file <path> <content>".cyan());
        println!("  {} - List directory contents", "list_files [path]".cyan());
        println!("  {} - Delete file", "delete_file <path>".cyan());
        println!("  {} - Create directory", "create_directory <path>".cyan());
        println!(
            "  {} - Find files by pattern",
            "find_files [dir] <pattern>".cyan()
        );
        println!("  {} - Copy file", "copy_file <from> <to>".cyan());
        println!("  {} - Move/rename file", "move_file <from> <to>".cyan());
        println!("  {} - Get file info", "file_info <path>".cyan());

        println!("\n{}", "Shell Commands:".green().bold());
        println!("  - Type any shell command directly (e.g., 'ls -la', 'grep pattern file.txt')");
        println!("  - Or use: {} <command>", "run".cyan());

        println!("\n{}", "How it works:".green().bold());
        println!("  - NexSh uses the ReAct (Reasoning and Acting) pattern");
        println!("  - The AI thinks, acts, observes, and repeats until task completion");
        println!(
            "  - Max iterations: {} (configurable)",
            self.config_manager
                .config
                .max_iterations
                .to_string()
                .yellow()
        );

        let verbose_status = if self.config_manager.config.verbose {
            "enabled".green()
        } else {
            "disabled".yellow()
        };
        println!(
            "\n{} Verbose mode is currently {}",
            "ℹ️".cyan(),
            verbose_status
        );
        println!("{}", "─".repeat(60).dimmed());
        println!();

        Ok(())
    }

    fn select_from_preset_models(
        &mut self,
        preset_name: &str,
        models: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        if models.is_empty() {
            println!("{}", "⚠️  No models found for this preset".yellow());
            return Ok(());
        }

        println!("\n{} {}", "📦 Preset:".cyan().bold(), preset_name.green());
        for (i, model) in models.iter().enumerate() {
            println!("  {}. {}", i + 1, model.cyan());
        }

        let input = self
            .editor
            .readline("Select model by number (Enter to cancel): ")
            .unwrap_or_default();
        let selection = input.trim();

        if !selection.is_empty() {
            if let Ok(idx) = selection.parse::<usize>() {
                if let Some(model) = models.get(idx.saturating_sub(1)) {
                    self.set_model(model)?;
                } else {
                    eprintln!("{} Invalid selection", "error:".red());
                }
            } else {
                eprintln!("{} Please enter a number", "error:".red());
            }
        }

        Ok(())
    }

    async fn handle_model_selection(&mut self) -> Result<(), Box<dyn Error>> {
        // Show preset options first
        println!("\n{}", "📋 Model Selection".cyan().bold());
        println!("Choose an option:");
        println!("  1. Browse all models (fetched from API)");
        println!("  2. Use preset: Programming (optimized for code)");
        println!("  3. Use preset: Reasoning (advanced problem-solving)");
        println!("  4. Use preset: Free (free-tier models only)");
        println!();

        let choice = self
            .editor
            .readline("Select option (1-4, default 1): ")
            .unwrap_or_default();

        match choice.trim() {
            "2" => {
                let models = crate::available_models::get_preset_models("programming");
                return self.select_from_preset_models("Programming", models);
            }
            "3" => {
                let models = crate::available_models::get_preset_models("reasoning");
                return self.select_from_preset_models("Reasoning", models);
            }
            "4" => {
                let models = crate::available_models::get_preset_models("free");
                return self.select_from_preset_models("Free", models);
            }
            _ => {
                // Continue with API fetch (option 1 or default)
            }
        }

        // Fetch models from API (already in async context, no need for new runtime)
        let models_result =
            crate::available_models::fetch_models_from_api(&self.ai_client.get_client()).await;

        match models_result {
            Ok(api_models) => {
                // Separate free and paid models
                let mut free_models: Vec<_> = api_models
                    .iter()
                    .filter(|m| crate::available_models::is_free_model(m))
                    .collect();
                let mut paid_models: Vec<_> = api_models
                    .iter()
                    .filter(|m| !crate::available_models::is_free_model(m))
                    .collect();

                // Sort by name
                free_models.sort_by(|a, b| a.name.cmp(&b.name));
                paid_models.sort_by(|a, b| a.name.cmp(&b.name));

                println!("\n{}", "🆓 FREE MODELS:".green().bold());
                for (i, m) in free_models.iter().enumerate() {
                    println!("  {}. {} ({})", i + 1, m.name.green(), m.id.cyan());
                }

                println!("\n{}", "💰 PAID MODELS (Popular):".yellow().bold());
                // Show only popular paid models (first 20)
                for (i, m) in paid_models.iter().take(20).enumerate() {
                    println!(
                        "  {}. {} ({})",
                        free_models.len() + i + 1,
                        m.name.yellow(),
                        m.id.cyan()
                    );
                }

                println!(
                    "\n{} {} paid models available",
                    "ℹ️".cyan(),
                    paid_models.len()
                );

                let input = self
                    .editor
                    .readline("Select model by number or ID (Enter to cancel): ")
                    .unwrap_or_default();
                let selection = input.trim();

                if !selection.is_empty() {
                    let all_models: Vec<_> = free_models
                        .iter()
                        .chain(paid_models.iter().take(20))
                        .collect();

                    let selected = if let Ok(idx) = selection.parse::<usize>() {
                        all_models.get(idx.saturating_sub(1)).map(|m| m.id.as_str())
                    } else {
                        // Search by ID or name
                        api_models
                            .iter()
                            .find(|m| m.id.contains(selection) || m.name.contains(selection))
                            .map(|m| m.id.as_str())
                    };

                    if let Some(model_id) = selected {
                        if let Err(e) = self.set_model(model_id) {
                            eprintln!("{} {}", "error:".red(), e);
                        }
                    } else {
                        eprintln!("{} Model not found", "error:".red());
                    }
                }
            }
            Err(_) => {
                // Fallback to static list if API fails
                println!(
                    "{}",
                    "⚠️  Using fallback model list (API unavailable)".yellow()
                );
                let models = list_available_models();

                println!("\n{}", "Available AI models:".cyan().bold());
                for (i, m) in models.iter().enumerate() {
                    let display = if m.contains(":free") {
                        format!("{}. {} {}", i + 1, m, "🆓".green())
                    } else {
                        format!("{}. {}", i + 1, m)
                    };
                    println!("  {}", display);
                }

                let input = self
                    .editor
                    .readline("Select model by number or name (Enter to cancel): ")
                    .unwrap_or_default();
                let model = input.trim();

                if !model.is_empty() {
                    let selected = if let Ok(idx) = model.parse::<usize>() {
                        models
                            .get(idx.saturating_sub(1))
                            .copied()
                            .unwrap_or(models[0])
                    } else {
                        models
                            .iter()
                            .find(|m| m.starts_with(model))
                            .copied()
                            .unwrap_or(models[0])
                    };

                    if let Err(e) = self.set_model(selected) {
                        eprintln!("{} {}", "error:".red(), e);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        println!("🤖 Welcome to NexSh!");

        // Check if API key is configured on startup
        if self.config_manager.config.api_key.is_empty() {
            println!("{}", "⚠️  No API key configured!".yellow());
            println!(
                "{}",
                "Please configure your OpenRouter API key to get started.".yellow()
            );
            println!(
                "{}",
                "Get your API key from: https://openrouter.ai/keys".cyan()
            );
            println!();

            // Prompt user to run init
            let input = self
                .editor
                .readline("Would you like to configure now? (Y/n): ")?;

            if input.trim().to_lowercase() != "n" {
                self.initialize()?;
                // Update AI client with new API key
                self.ai_client = AIClient::new(
                    self.config_manager.config.api_key.clone(),
                    self.config_manager.config.model.clone(),
                )?;
            } else {
                println!(
                    "{}",
                    "You can run 'init' command anytime to configure.".cyan()
                );
            }
        }

        loop {
            let prompt = PromptBuilder::create_shell_prompt()?;

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let input = line.trim();
                    if input.is_empty() {
                        continue;
                    }

                    match input {
                        "exit" | "quit" => break,
                        "clear" => self.clear_context()?,
                        "init" => self.initialize()?,
                        "help" => self.print_help()?,
                        "config" => self.show_config()?,
                        "verbose" | "verbose on" => {
                            self.config_manager.update_config(|config| {
                                config.verbose = true;
                            })?;
                            println!(
                                "{}",
                                "✅ Verbose mode enabled - will show all thoughts and actions"
                                    .green()
                            );
                        }
                        "verbose off" => {
                            self.config_manager.update_config(|config| {
                                config.verbose = false;
                            })?;
                            println!(
                                "{}",
                                "✅ Verbose mode disabled - will show only final answers".green()
                            );
                        }
                        "models" => {
                            self.handle_model_selection().await?;
                            continue;
                        }
                        _ => {
                            // Check for config set commands
                            if input.starts_with("set max_iterations ") {
                                if let Some(value_str) = input.strip_prefix("set max_iterations ") {
                                    if let Ok(value) = value_str.parse::<usize>() {
                                        self.config_manager.update_config(|config| {
                                            config.max_iterations = value;
                                        })?;
                                        // Update the react_loop with new value
                                        self.react_loop = ReActLoop::new(value);
                                        println!(
                                            "✅ max_iterations set to: {}",
                                            value.to_string().green()
                                        );
                                    } else {
                                        eprintln!("{} Invalid number", "error:".red());
                                    }
                                } else {
                                    eprintln!(
                                        "{} Usage: set max_iterations <number>",
                                        "error:".red()
                                    );
                                }
                                continue;
                            }

                            if let Err(e) = self.process_command(input).await {
                                eprintln!("{} {}", "error:".red(), e);
                            }
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Use 'exit' to quit");
                    continue;
                }
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    break;
                }
            }
        }

        // Save history
        self.editor
            .save_history(&self.config_manager.history_file)?;
        Ok(())
    }
}

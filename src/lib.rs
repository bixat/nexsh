use colored::*;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::error::Error;

use crate::{
    ai_client::AIClient,
    available_models::list_available_models,
    command_executor::CommandExecutor,
    config_manager::ConfigManager,
    context_manager::ContextManager,
    types::{CommandResult, Message, NexShConfig},
    ui_progress::ProgressManager,
    ui_prompt_builder::PromptBuilder,
};

// Export modules
pub mod ai_client;
pub mod ai_request_builder;
pub mod available_models;
pub mod command_executor;
pub mod config_manager;
pub mod context_manager;
pub mod prompt;
pub mod types;
pub mod ui_progress;
pub mod ui_prompt_builder;

impl Default for NexShConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            history_size: 1000,
            max_context_messages: 100,
            model: Some("gemini-2.0-flash".to_string()),
        }
    }
}

pub struct NexSh {
    config_manager: ConfigManager,
    ai_client: AIClient,
    command_executor: CommandExecutor,
    context_manager: ContextManager,
    progress_manager: ProgressManager,
    messages: Vec<Message>,
    editor: DefaultEditor,
}

impl NexSh {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let config_manager = ConfigManager::new()?;
        let editor = config_manager.create_editor()?;

        // Initialize AI client with config
        let ai_client = AIClient::new(
            config_manager.config.api_key.clone(),
            config_manager.config.model.clone(),
        );

        // Initialize context manager
        let context_manager = ContextManager::new(
            config_manager.context_file.clone(),
            config_manager.config.max_context_messages,
        );

        // Load existing messages from context
        let messages = context_manager.load_context().unwrap_or_default();

        Ok(Self {
            config_manager,
            ai_client,
            command_executor: CommandExecutor::new(),
            context_manager,
            progress_manager: ProgressManager::new(),
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

        println!("✅ Gemini model set to: {}", model.green());
        Ok(())
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        println!("🤖 Welcome to NexSh Setup!");

        // Get API key
        let input = self
            .editor
            .readline("Enter your Gemini API key (leave blank to keep current if exists): ")?;
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

        // Model selection
        let models = list_available_models();
        println!("Available Gemini models:");
        for (i, m) in models.iter().enumerate() {
            println!("  {}. {}", i + 1, m);
        }

        let input = self
            .editor
            .readline("Select Gemini model by number or name (default 1): ")?;
        let model = input.trim();
        let selected = if model.is_empty() {
            models[0]
        } else if let Ok(idx) = model.parse::<usize>() {
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

        self.set_model(selected)?;
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
            .create_spinner("Thinking...".yellow().to_string());

        // Process command with AI client
        match self
            .ai_client
            .process_command_request(input, &self.messages)
            .await
        {
            Ok(response) => {
                pb.finish_and_clear();

                println!("{} {}", "🤖 →".green(), response.message.yellow());

                if response.command.is_empty() {
                    // Add model response to context
                    self.context_manager.add_message(
                        &mut self.messages,
                        "model",
                        &response.message,
                    )?;
                    return Ok(());
                }

                // Add command to history
                self.editor.add_history_entry(&response.command)?;

                println!("{} {}", "Category :".green(), response.category.yellow());
                println!("{} {}", "→".blue(), response.command);

                // Add model response to context
                self.context_manager.add_message(
                    &mut self.messages,
                    "model",
                    &format!(
                        "Command: {}, message: {}",
                        response.command, response.message
                    ),
                )?;

                // Execute command if not dangerous or user confirms
                if !response.dangerous || self.confirm_execution()? {
                    match self.command_executor.execute(&response.command)? {
                        CommandResult::Success(output) => {
                            if !output.is_empty() {
                                self.context_manager.add_message(
                                    &mut self.messages,
                                    "model",
                                    &format!("Command output:\n{}", output),
                                )?;
                            }
                        }
                        CommandResult::Error(error) => {
                            // Get AI explanation for the error
                            let _pb = self
                                .progress_manager
                                .create_spinner("Requesting explanation ...".blue().to_string());
                            if let Ok(explanation) = self
                                .ai_client
                                .get_command_explanation(&response.command, &error)
                                .await
                            {
                                _pb.finish_and_clear();
                                println!(
                                    "{} {}",
                                    "🤖 AI Explanation:".green(),
                                    explanation.yellow()
                                );
                            }
                        }
                    }
                } else {
                    println!("Command execution cancelled.");
                }
            }
            Err(e) => {
                pb.finish_and_clear();
                eprintln!("Failed to process command: {}", e);
            }
        }

        Ok(())
    }

    fn confirm_execution(&mut self) -> Result<bool, Box<dyn Error>> {
        let _input = self
            .editor
            .readline(&PromptBuilder::create_simple_confirmation())?;

        if _input.trim().to_lowercase() == "n" {
            return Ok(false);
        }

        let _input = self
            .editor
            .readline(&PromptBuilder::create_danger_confirmation())?;

        Ok(_input.trim().to_lowercase() == "y")
    }

    fn clear_context(&mut self) -> Result<(), Box<dyn Error>> {
        self.context_manager.clear_context(&mut self.messages)?;
        println!("{}", "🧹 Conversation context cleared".green());
        Ok(())
    }

    pub fn print_help(&self) -> Result<(), Box<dyn Error>> {
        println!("🤖 NexSh Help:");
        println!("  - Type 'exit' or 'quit' to exit the shell.");
        println!("  - Type any command to execute it.");
        println!("  - Use 'init' to set up your API key.");
        println!("  - Use 'clear' to clear conversation context.");
        println!("  - Type 'models' to list and select available Gemini models interactively.");
        Ok(())
    }

    fn handle_model_selection(&mut self) -> Result<(), Box<dyn Error>> {
        let models = list_available_models();
        println!("Available Gemini models:");
        for (i, m) in models.iter().enumerate() {
            println!("  {}. {}", i + 1, m);
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

        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        println!("🤖 Welcome to NexSh!");

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
                        "models" => {
                            self.handle_model_selection()?;
                            continue;
                        }
                        _ => {
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

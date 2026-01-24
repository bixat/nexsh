use chrono::{DateTime, Utc};
use colored::*;
use nexsh::config_manager::ConfigManager;
use std::env;

pub fn print_header() {
    // ASCII Art Logo
    let logo = r#"
    ███╗   ██╗███████╗██╗  ██╗███████╗██╗  ██╗
    ████╗  ██║██╔════╝╚██╗██╔╝██╔════╝██║  ██║
    ██╔██╗ ██║█████╗   ╚███╔╝ ███████╗███████║
    ██║╚██╗██║██╔══╝   ██╔██╗ ╚════██║██╔══██║
    ██║ ╚████║███████╗██╔╝ ██╗███████║██║  ██║
    ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝"#;

    // System Info
    let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let now: DateTime<Utc> = Utc::now();
    let version = env!("CARGO_PKG_VERSION");

    // Load configuration
    let config_manager = ConfigManager::new().unwrap();
    let config = config_manager.config;

    
    // Print Header
    println!("{}", logo.bright_cyan());
    println!("{}", "━".repeat(65).bright_blue());
    println!(
        "{} {} {} {} {} {} {}",
        "🤖".cyan(),
        "AI-Powered Shell".bright_white(),
        "|".bright_blue(),
        format!("v{}", version).yellow(),
        "|".bright_blue(),
        username.green(),
        now.format("(%Y-%m-%d %H:%M UTC)")
            .to_string()
            .bright_black()
    );
    println!("{}", "━".repeat(65).bright_blue());
    
    // Current Configuration
    println!("{} {}", "⚙️".bright_yellow(), "Current Configuration".bright_white().bold());
    println!("  {} {}: {}", "•".bright_blue(), "Model".white(), config.model.unwrap_or("Not set".to_string()).green());
    println!("  {} {}: {}", "•".bright_blue(), "Verbose Mode".white(), if config.verbose { "ON".green() } else { "OFF".red() });
    println!();
    
    // Categorized Commands
    println!("{} {}", "🚀".bright_yellow(), "Quick Start".bright_white().bold());
    println!("  {} {} - Set up your API key", "•".bright_blue(), "init".cyan());
    println!("  {} {} - Get help and documentation", "•".bright_blue(), "help".cyan());
    println!();
    
    println!("{} {}", "🤖".bright_yellow(), "AI Features".bright_white().bold());
    println!("  {} {} - Browse and select AI models", "•".bright_blue(), "models".cyan());
    println!("  {} {} - Show AI reasoning process", "•".bright_blue(), "verbose on".cyan());
    println!("  {} {} - Show only final answers (default)", "•".bright_blue(), "verbose off".cyan());
    println!();
    
    println!("{} {}", "🔧".bright_yellow(), "Session Management".bright_white().bold());
    println!("  {} {} - Clear conversation context", "•".bright_blue(), "clear".cyan());
    println!("  {} {} - Exit the shell", "•".bright_blue(), "exit/quit".cyan());
    println!();
    
    println!("{} {}", "💡".bright_yellow(), "How It Works".bright_white().bold());
    println!("  {} NexSh uses {} for intelligent command generation", "•".bright_blue(), "ReAct (Reasoning and Acting)".green());
    println!("  {} Type any command or describe what you want to do", "•".bright_blue());
    println!();

    println!(
        "{} Type {} for detailed help or {} to exit",
        "→".bright_yellow(),
        "'help'".cyan(),
        "'exit'".cyan()
    );
}

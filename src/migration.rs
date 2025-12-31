use crate::types::NexShConfig;
use colored::*;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationState {
    pub version: String,
    pub migrations_applied: Vec<String>,
    pub last_migration_date: u64,
}

impl Default for MigrationState {
    fn default() -> Self {
        Self {
            version: "0.9.0".to_string(),
            migrations_applied: Vec::new(),
            last_migration_date: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

pub struct MigrationManager {
    config_dir: PathBuf,
    migration_file: PathBuf,
}

impl MigrationManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let proj_dirs = ProjectDirs::from("com", "nexsh", "nexsh")
            .ok_or("Failed to get project directories")?;

        let config_dir = proj_dirs.config_dir().to_path_buf();
        fs::create_dir_all(&config_dir)?;

        let migration_file = config_dir.join("migrations.json");

        Ok(Self {
            config_dir,
            migration_file,
        })
    }

    pub fn load_migration_state(&self) -> Result<MigrationState, Box<dyn Error>> {
        if self.migration_file.exists() {
            let content = fs::read_to_string(&self.migration_file)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(MigrationState::default())
        }
    }

    pub fn save_migration_state(&self, state: &MigrationState) -> Result<(), Box<dyn Error>> {
        let content = serde_json::to_string_pretty(state)?;
        fs::write(&self.migration_file, content)?;
        Ok(())
    }

    /// Migrate from old Gemini config to new OpenRouter config
    pub fn migrate_gemini_to_openrouter(&self) -> Result<bool, Box<dyn Error>> {
        let old_config_dir = ProjectDirs::from("com", "gemini-shell", "nexsh")
            .ok_or("Failed to get old project directories")?
            .config_dir()
            .to_path_buf();

        let old_config_file = old_config_dir.join("nexsh_config.json");

        if !old_config_file.exists() {
            return Ok(false); // No old config to migrate
        }

        println!("{}", "🔄 Migrating from Gemini to OpenRouter...".cyan());

        // Load old config
        let old_content = fs::read_to_string(&old_config_file)?;
        let old_config: serde_json::Value = serde_json::from_str(&old_content)?;

        // Create new config with migrated values
        let mut new_config = NexShConfig::default();

        if let Some(api_key) = old_config.get("api_key").and_then(|v| v.as_str()) {
            new_config.api_key = api_key.to_string();
            println!("  ✓ Migrated API key (you'll need to update to OpenRouter key)");
        }

        if let Some(history_size) = old_config.get("history_size").and_then(|v| v.as_u64()) {
            new_config.history_size = history_size as usize;
            println!("  ✓ Migrated history size: {}", history_size);
        }

        if let Some(max_context) = old_config
            .get("max_context_messages")
            .and_then(|v| v.as_u64())
        {
            new_config.max_context_messages = max_context as usize;
            println!("  ✓ Migrated max context messages: {}", max_context);
        }

        // Migrate model - convert Gemini model to OpenRouter equivalent
        if let Some(old_model) = old_config.get("model").and_then(|v| v.as_str()) {
            new_config.model = Some(self.convert_gemini_model_to_openrouter(old_model));
            println!(
                "  ✓ Migrated model: {} -> {}",
                old_model,
                new_config.model.as_ref().unwrap()
            );
        }

        // Save new config
        let new_config_file = self.config_dir.join("nexsh_config.json");
        let content = serde_json::to_string_pretty(&new_config)?;
        fs::write(new_config_file, content)?;

        // Copy history and context files if they exist
        self.migrate_file(&old_config_dir, "nexsh_history.txt")?;
        self.migrate_file(&old_config_dir, "nexsh_context.json")?;

        println!("{}", "✅ Migration completed successfully!".green());
        println!(
            "{}",
            "⚠️  Note: Please update your API key to an OpenRouter key using 'nexsh init'"
                .yellow()
        );

        Ok(true)
    }

    fn convert_gemini_model_to_openrouter(&self, gemini_model: &str) -> String {
        match gemini_model {
            "gemini-2.0-flash" | "gemini-2.0-pro" => "google/gemini-2.5-flash".to_string(),
            "gemini-1.5-flash" => "google/gemini-flash-1.5".to_string(),
            "gemini-1.5-pro" => "google/gemini-pro-1.5".to_string(),
            _ => "anthropic/claude-sonnet-4".to_string(), // Default to Claude
        }
    }

    fn migrate_file(&self, old_dir: &PathBuf, filename: &str) -> Result<(), Box<dyn Error>> {
        let old_file = old_dir.join(filename);
        if old_file.exists() {
            let new_file = self.config_dir.join(filename);
            fs::copy(&old_file, &new_file)?;
            println!("  ✓ Migrated {}", filename);
        }
        Ok(())
    }

    pub fn run_migrations(&self) -> Result<(), Box<dyn Error>> {
        let mut state = self.load_migration_state()?;

        // Check if we need to migrate from Gemini
        if !state.migrations_applied.contains(&"gemini_to_openrouter".to_string()) {
            if self.migrate_gemini_to_openrouter()? {
                state.migrations_applied.push("gemini_to_openrouter".to_string());
                state.last_migration_date = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                self.save_migration_state(&state)?;
            }
        }

        Ok(())
    }
}


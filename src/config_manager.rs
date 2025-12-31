use crate::types::NexShConfig;
use directories::ProjectDirs;
use rustyline::{Config as EditorConfig, DefaultEditor};
use serde_json;
use std::{error::Error, fs, path::PathBuf};

pub struct ConfigManager {
    pub config: NexShConfig,
    pub config_dir: PathBuf,
    pub history_file: PathBuf,
    pub context_file: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Use new directory structure
        let proj_dirs = ProjectDirs::from("com", "nexsh", "nexsh")
            .ok_or("Failed to get project directories")?;

        let config_dir = proj_dirs.config_dir().to_path_buf();
        fs::create_dir_all(&config_dir)?;

        let config_file = config_dir.join("nexsh_config.json");
        let history_file = config_dir.join("nexsh_history.txt");
        let context_file = config_dir.join("nexsh_context.json");

        let config = if config_file.exists() {
            Self::load_config(&config_file)?
        } else {
            NexShConfig::default()
        };

        Ok(Self {
            config,
            config_dir,
            history_file,
            context_file,
        })
    }

    fn load_config(config_file: &PathBuf) -> Result<NexShConfig, Box<dyn Error>> {
        let content = fs::read_to_string(config_file)?;
        let parsed: serde_json::Value = serde_json::from_str(&content)?;

        Ok(NexShConfig {
            api_key: parsed
                .get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            history_size: parsed
                .get("history_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000) as usize,
            max_context_messages: parsed
                .get("max_context_messages")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize,
            model: parsed
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(Some("anthropic/claude-sonnet-4".to_string())),
            verbose: parsed
                .get("verbose")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        })
    }

    pub fn save_config(&self) -> Result<(), Box<dyn Error>> {
        let config_file = self.config_dir.join("nexsh_config.json");
        let content = serde_json::to_string_pretty(&self.config)?;
        fs::write(config_file, content)?;
        Ok(())
    }

    pub fn create_editor(&self) -> Result<DefaultEditor, Box<dyn Error>> {
        let editor_config = EditorConfig::builder()
            .max_history_size(self.config.history_size)?
            .build();
        let mut editor = DefaultEditor::with_config(editor_config)?;

        if self.history_file.exists() {
            let _ = editor.load_history(&self.history_file);
        }

        Ok(editor)
    }

    pub fn update_config<F>(&mut self, updater: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce(&mut NexShConfig),
    {
        updater(&mut self.config);
        self.save_config()
    }
}

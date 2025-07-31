use crate::types::Message;
use serde_json;
use std::{error::Error, fs, path::PathBuf};

pub struct ContextManager {
    context_file: PathBuf,
    max_context_messages: usize,
}

impl ContextManager {
    pub fn new(context_file: PathBuf, max_context_messages: usize) -> Self {
        Self {
            context_file,
            max_context_messages,
        }
    }

    /// Load conversation context from file
    pub fn load_context(&self) -> Result<Vec<Message>, Box<dyn Error>> {
        if !self.context_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.context_file)?;
        let messages: Vec<Message> = serde_json::from_str(&content).unwrap_or_default();
        Ok(messages)
    }

    /// Save conversation context to file
    pub fn save_context(&self, messages: &[Message]) -> Result<(), Box<dyn Error>> {
        let content = serde_json::to_string_pretty(messages)?;
        fs::write(&self.context_file, content)?;
        Ok(())
    }

    /// Add a new message to the context, managing size limits
    pub fn add_message(
        &self,
        messages: &mut Vec<Message>,
        role: &str,
        content: &str,
    ) -> Result<(), Box<dyn Error>> {
        let message = Message {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        messages.push(message);

        // Trim old messages if we exceed max_context_messages
        if messages.len() > self.max_context_messages {
            *messages = messages.split_off(messages.len() - self.max_context_messages);
        }

        self.save_context(messages)
    }

    /// Clear all messages from context
    pub fn clear_context(&self, messages: &mut Vec<Message>) -> Result<(), Box<dyn Error>> {
        messages.clear();
        self.save_context(messages)
    }
}

use crate::types::Message;
use serde_json::{json, Value};

pub struct RequestBuilder;

impl RequestBuilder {
    /// Build a command request JSON
    pub fn build_command_request(messages: &[Message], system_prompt: &str) -> Value {
        let mut contents = Vec::new();

        // Add conversation history
        for msg in messages {
            contents.push(json!({
                "parts": [{
                    "text": msg.content
                }],
                "role": msg.role
            }));
        }

        json!({
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseSchema": {
                    "type": "object",
                    "required": ["message", "command", "dangerous", "category"],
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Clear, concise message with relevant emoji",
                            "minLength": 1
                        },
                        "command": {
                            "type": "string",
                            "description": "Shell command to execute, empty if no action needed"
                        },
                        "dangerous": {
                            "type": "boolean",
                            "description": "True if command could be potentially harmful"
                        },
                        "category": {
                            "type": "string",
                            "description": "Classification of the command type",
                            "enum": ["system", "file", "network", "package", "text", "process", "other"]
                        }
                    }
                },
            },
            "system_instruction": {
                "parts": [
                    {
                        "text": system_prompt
                    }
                ],
                "role": "system"
            },
            "contents": contents,
            "tools": []
        })
    }

    /// Build an explanation request JSON
    pub fn build_explanation_request(prompt: &str) -> Value {
        json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }],
                "role": "user"
            }],
            "tools": []
        })
    }
}

use crate::{
    prompt::{EXPLANATION_PROMPT, SYSTEM_PROMPT},
    types::{GeminiResponse, Message},
};
use gemini_client_rs::{
    types::{GenerateContentRequest, PartResponse},
    GeminiClient,
};
use serde_json::json;
use std::error::Error;

pub struct AIClient {
    client: GeminiClient,
    model: String,
}

impl AIClient {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let client = GeminiClient::new(api_key);
        let model = model.unwrap_or_else(|| "gemini-2.0-flash".to_string());

        Self { client, model }
    }

    /// Process a command request and return the AI response
    pub async fn process_command_request(
        &self,
        _input: &str,
        messages: &[Message],
    ) -> Result<GeminiResponse, Box<dyn Error>> {
        let os = std::env::consts::OS.to_string();
        let prompt = SYSTEM_PROMPT.replace("{OS}", &os);

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

        let req_json = json!({
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
                        "text": prompt
                    }
                ],
                "role": "system"
            },
            "contents": contents,
            "tools": []
        });

        let request: GenerateContentRequest = serde_json::from_value(req_json)?;
        let response = self.client.generate_content(&self.model, &request).await?;

        if let Some(candidates) = response.candidates {
            for candidate in &candidates {
                for part in &candidate.content.parts {
                    if let PartResponse::Text(json_str) = part {
                        let clean_json = json_str
                            .trim()
                            .trim_start_matches("```json")
                            .trim_end_matches("```")
                            .trim();

                        match serde_json::from_str::<GeminiResponse>(clean_json) {
                            Ok(response) => return Ok(response),
                            Err(e) => {
                                eprintln!("Failed to parse response: {}", e);
                                println!("Raw response: {}", clean_json);

                                if cfg!(debug_assertions) {
                                    println!(
                                        "Debug: Response contains markdown block: {}",
                                        json_str.contains("```")
                                    );
                                    println!("Debug: Cleaned JSON: {}", clean_json);
                                }
                                return Err(e.into());
                            }
                        }
                    }
                }
            }
        }

        Err("No valid response received from AI".into())
    }

    /// Get explanation for a failed command
    pub async fn get_command_explanation(
        &self,
        command: &str,
        error_message: &str,
    ) -> Result<String, Box<dyn Error>> {
        let prompt = EXPLANATION_PROMPT
            .replace("{COMMAND}", command)
            .replace("{ERROR}", error_message);

        let req_json = json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }],
                "role": "user"
            }],
            "tools": []
        });

        let request: GenerateContentRequest = serde_json::from_value(req_json)?;
        let response = self.client.generate_content(&self.model, &request).await?;

        if let Some(candidates) = response.candidates {
            for candidate in &candidates {
                for part in &candidate.content.parts {
                    if let PartResponse::Text(explanation) = part {
                        return Ok(explanation.clone());
                    }
                }
            }
        }

        Err("No explanation available".into())
    }

    /// Update the model used by the AI client
    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }
}

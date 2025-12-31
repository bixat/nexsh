use crate::{
    prompt::{SYSTEM_PROMPT},
    types::{ActionType, GeminiResponse, Message, ReActResponse},
};
use colored::Colorize;
use openrouter_rs::{
    api::chat::{ChatCompletionRequest, Message as OpenRouterMessage},
    types::Role,
    OpenRouterClient,
};
use std::error::Error;

pub struct AIClient {
    client: OpenRouterClient,
    model: String,
}

impl AIClient {
    pub fn new(api_key: String, model: Option<String>) -> Result<Self, Box<dyn Error>> {
        // Allow empty API key during initialization, but use a placeholder
        let api_key_to_use = if api_key.is_empty() {
            "placeholder_key_not_configured".to_string()
        } else {
            api_key
        };

        let client = OpenRouterClient::builder()
            .api_key(&api_key_to_use)
            .http_referer("https://bixat.dev/products/nexsh")
            .x_title("NexSh - AI Shell Assistant")
            .build()?;

        let model = model.unwrap_or_else(|| "anthropic/claude-sonnet-4".to_string());

        Ok(Self { client, model })
    }

    /// Parse the simple text format response
    /// Format:
    /// Thought: <reasoning>
    /// Action: <command or "">
    /// Dangerous: <true or false>
    /// Category: <category>
    /// Final Answer: <optional message>
    fn parse_simple_format(content: &str) -> Option<GeminiResponse> {
        let mut thought = String::new();
        let mut action = String::new();
        let mut dangerous = false;
        let mut category = String::from("other");
        let mut final_answer = String::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(value) = line.strip_prefix("Thought:") {
                thought = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("Action:") {
                action = value.trim().trim_matches('"').to_string();
            } else if let Some(value) = line.strip_prefix("Dangerous:") {
                dangerous = value.trim().eq_ignore_ascii_case("true");
            } else if let Some(value) = line.strip_prefix("Category:") {
                category = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("Final Answer:") {
                final_answer = value.trim().to_string();
            }
        }

        // Validate that we have at least a thought
        if thought.is_empty() {
            return None;
        }

        // Use final answer if present, otherwise use thought
        let message = if !final_answer.is_empty() {
            final_answer
        } else {
            thought
        };

        Some(GeminiResponse {
            message,
            command: action,
            dangerous,
            category,
        })
    }

    /// Process a command request using ReAct pattern and return the AI response
    pub async fn process_command_request(
        &self,
        _input: &str,
        messages: &[Message],
    ) -> Result<GeminiResponse, Box<dyn Error>> {
        let os = std::env::consts::OS.to_string();
        let system_prompt = SYSTEM_PROMPT.replace("{OS}", &os);

        // Convert messages to OpenRouter format
        let mut openrouter_messages = vec![OpenRouterMessage::new(Role::System, &system_prompt)];

        // Add conversation history
        for msg in messages {
            let role = match msg.role.as_str() {
                "user" => Role::User,
                "model" | "assistant" => Role::Assistant,
                "system" => Role::System,
                _ => Role::User,
            };
            openrouter_messages.push(OpenRouterMessage::new(role, &msg.content));
        }

        // Add format reminder as a separate system message for better attention
        let format_reminder = r#"RESPONSE FORMAT REMINDER:
Use this EXACT format for your response:

Thought: <your reasoning>
Action: <command or "">
Dangerous: <true or false>
Category: <system|file|network|package|text|process|other>
Final Answer: <optional message>

Each field on its own line. No extra text before or after."#;

        openrouter_messages.push(OpenRouterMessage::new(Role::System, format_reminder));

        // Build the request with low temperature for consistent output
        let request = ChatCompletionRequest::builder()
            .model(&self.model)
            .messages(openrouter_messages)
            .temperature(0.2) // Low temperature for consistent structured output
            .max_tokens(2000)
            .build()?;

        // Send request
        let response = match self.client.send_chat_completion(&request).await {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("401") || error_msg.contains("User not found") {
                    return Err(
                        "Invalid API key. Please run 'nexsh init' to configure a valid OpenRouter API key.\nGet your API key from: https://openrouter.ai/keys"
                            .into(),
                    );
                }
                return Err(e.into());
            }
        };

        // Extract the response content
        if let Some(content) = response.choices.first().and_then(|c| c.content()) {
            // Try to parse the new simple text format first
            if let Some(parsed) = Self::parse_simple_format(content) {
                return Ok(parsed);
            }

            // Fallback: try JSON parsing for backward compatibility
            let mut clean_json = content.trim();

            // Remove markdown code blocks
            if clean_json.starts_with("```json") {
                clean_json = clean_json.trim_start_matches("```json").trim();
            }
            if clean_json.starts_with("```") {
                clean_json = clean_json.trim_start_matches("```").trim();
            }
            if clean_json.ends_with("```") {
                clean_json = clean_json.trim_end_matches("```").trim();
            }

            // Try to extract JSON from the response if it contains other text
            if let Some(start) = clean_json.find('{') {
                if let Some(end) = clean_json.rfind('}') {
                    clean_json = &clean_json[start..=end];
                }
            }

            // Try to parse as ReActResponse first
            match serde_json::from_str::<ReActResponse>(clean_json) {
                Ok(react_response) => {
                    // Convert ReActResponse to GeminiResponse for backward compatibility
                    let message = react_response
                        .final_answer
                        .unwrap_or_else(|| react_response.thought.reasoning.clone());

                    let command = match react_response.action.action_type {
                        ActionType::Execute => react_response.action.command,
                        _ => String::new(),
                    };

                    return Ok(GeminiResponse {
                        message,
                        command,
                        dangerous: react_response.action.dangerous,
                        category: react_response.action.category,
                    });
                }
                Err(_) => {
                    // Try old GeminiResponse format
                    if let Ok(response) = serde_json::from_str::<GeminiResponse>(clean_json) {
                        return Ok(response);
                    }

                    // Last resort: wrap as plain text
                    eprintln!(
                        "{}",
                        "⚠️  Could not parse AI response format. Using as plain text...".yellow()
                    );
                    return Ok(GeminiResponse {
                        message: content.trim().to_string(),
                        command: String::new(),
                        dangerous: false,
                        category: "other".to_string(),
                    });
                }
            }
        }

        Err("No valid response received from AI".into())
    }

    /// Update the model used by the AI client
    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    pub fn get_client(&self) -> &OpenRouterClient {
        &self.client
    }
}

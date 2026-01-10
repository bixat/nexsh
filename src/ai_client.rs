use crate::{
    prompt::SYSTEM_PROMPT,
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

        let mut action_lines = Vec::new();
        let mut final_answer_lines = Vec::new();
        let mut in_action = false;
        let mut in_final_answer = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Check if this line starts a new field
            if trimmed.starts_with("Thought:") {
                in_action = false;
                in_final_answer = false;
                if let Some(value) = trimmed.strip_prefix("Thought:") {
                    thought = value.trim().to_string();
                }
            } else if trimmed.starts_with("Action:") {
                in_action = true;
                in_final_answer = false;
                action_lines.clear();
                if let Some(value) = trimmed.strip_prefix("Action:") {
                    let val = value.trim().trim_matches('"');
                    if !val.is_empty() {
                        action_lines.push(val.to_string());
                    }
                }
            } else if trimmed.starts_with("Dangerous:") {
                in_action = false;
                in_final_answer = false;
                if let Some(value) = trimmed.strip_prefix("Dangerous:") {
                    dangerous = value.trim().eq_ignore_ascii_case("true");
                }
            } else if trimmed.starts_with("Category:") {
                in_action = false;
                in_final_answer = false;
                if let Some(value) = trimmed.strip_prefix("Category:") {
                    category = value.trim().to_string();
                }
            } else if trimmed.starts_with("Final Answer:") {
                in_action = false;
                in_final_answer = true;
                final_answer_lines.clear();
                if let Some(value) = trimmed.strip_prefix("Final Answer:") {
                    let val = value.trim();
                    if !val.is_empty() {
                        final_answer_lines.push(val.to_string());
                    }
                }
            } else if in_action && !trimmed.is_empty() {
                // Continue collecting action lines until we hit another field
                action_lines.push(trimmed.to_string());
            } else if in_final_answer {
                // Continue collecting final answer lines (including empty lines for formatting)
                final_answer_lines.push(line.to_string());
            }
        }

        // Join action lines with newlines to preserve multiline commands
        if !action_lines.is_empty() {
            action = action_lines.join("\n");
        }

        // Join final answer lines with newlines to preserve formatting
        if !final_answer_lines.is_empty() {
            final_answer = final_answer_lines.join("\n");
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
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());

        let system_prompt = SYSTEM_PROMPT
            .replace("{OS}", &os)
            .replace("{CWD}", &cwd)
            .replace("{SHELL}", &shell);

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
        let format_reminder = r#"RESPONSE FORMAT - Use this EXACT structure:

Thought: <brief reasoning about the task and your approach>
Action: <shell command to run, OR "" if providing final answer>
Dangerous: <true or false>
Category: <system|file|network|package|text|process|other>
Final Answer: <message for the user - required when Action is "">

IMPORTANT GUIDELINES:
- For SIMPLE tasks: Execute immediately, be concise
- For COMPLEX tasks: Break into steps, execute one command per iteration
- If a command FAILS: Try an alternative approach, don't repeat the same command
- After gathering enough info: Set Action to "" and provide Final Answer
- Each field MUST be on its own line, starting with the field name and colon"#;

        openrouter_messages.push(OpenRouterMessage::new(Role::System, format_reminder));

        // Build the request with balanced temperature
        let request = ChatCompletionRequest::builder()
            .model(&self.model)
            .messages(openrouter_messages)
            .temperature(0.3) // Slightly higher for better reasoning
            .max_tokens(2500) // Increased for complex responses
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

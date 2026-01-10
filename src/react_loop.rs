use crate::{
    ai_client::AIClient,
    command_executor::CommandExecutor,
    types::{ActionType, AgentObservation, AgentStep, CommandResult, Message},
};
use std::error::Error;

const MAX_ITERATIONS: usize = 15; // Increased for complex tasks
const MAX_CONSECUTIVE_ERRORS: usize = 3; // Stop after repeated failures

pub struct ReActLoop {
    max_iterations: usize,
    max_consecutive_errors: usize,
}

impl Default for ReActLoop {
    fn default() -> Self {
        Self {
            max_iterations: MAX_ITERATIONS,
            max_consecutive_errors: MAX_CONSECUTIVE_ERRORS,
        }
    }
}

impl ReActLoop {
    pub fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            max_consecutive_errors: MAX_CONSECUTIVE_ERRORS,
        }
    }

    /// Run the ReAct loop: Think → Act → Observe → Repeat until final answer
    pub async fn run(
        &self,
        ai_client: &AIClient,
        command_executor: &CommandExecutor,
        messages: &[Message],
        user_input: &str,
    ) -> Result<Vec<AgentStep>, Box<dyn Error>> {
        let mut steps = Vec::new();
        let mut iteration = 0;
        let mut consecutive_errors = 0;
        let mut context_messages = messages.to_vec();

        // Add user input to context
        let enhanced_input = format!("User request: {}", user_input);

        context_messages.push(Message {
            role: "user".to_string(),
            content: enhanced_input,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        });

        loop {
            iteration += 1;

            // Check iteration limit
            if iteration > self.max_iterations {
                // Add a message to context asking for final summary
                context_messages.push(Message {
                    role: "user".to_string(),
                    content: "Maximum iterations reached. Please provide a final summary of what was accomplished and any remaining steps.".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                });

                // Get final summary from AI
                if let Ok(final_response) = ai_client
                    .process_command_request(user_input, &context_messages)
                    .await
                {
                    let final_step = AgentStep {
                        thought: crate::types::AgentThought {
                            reasoning: "Providing final summary after max iterations".to_string(),
                            plan: String::new(),
                        },
                        action: crate::types::AgentAction {
                            action_type: ActionType::Respond,
                            command: String::new(),
                            dangerous: false,
                            category: "other".to_string(),
                        },
                        observation: Some(AgentObservation {
                            result: final_response.message,
                            success: true,
                            error: None,
                        }),
                    };
                    steps.push(final_step);
                }
                break;
            }

            // Check consecutive error limit
            if consecutive_errors >= self.max_consecutive_errors {
                // Ask AI to explain the issue
                context_messages.push(Message {
                    role: "user".to_string(),
                    content: format!(
                        "There have been {} consecutive errors. Please explain what's going wrong and suggest how to proceed.",
                        consecutive_errors
                    ),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                });
                consecutive_errors = 0; // Reset to allow AI to explain
            }

            // Get AI response
            let response = ai_client
                .process_command_request(user_input, &context_messages)
                .await?;

            // Create step
            let mut step = AgentStep {
                thought: crate::types::AgentThought {
                    reasoning: response.message.clone(),
                    plan: format!("Iteration {}/{}", iteration, self.max_iterations),
                },
                action: crate::types::AgentAction {
                    action_type: if response.command.is_empty() {
                        ActionType::Respond
                    } else {
                        ActionType::Execute
                    },
                    command: response.command.clone(),
                    dangerous: response.dangerous,
                    category: response.category.clone(),
                },
                observation: None,
            };

            // Check if this is a final answer (no command to execute)
            if response.command.is_empty() {
                steps.push(step);
                break;
            }

            // Execute command
            match command_executor.execute(&response.command)? {
                CommandResult::Success(output) => {
                    consecutive_errors = 0; // Reset on success

                    // Truncate very long outputs to keep context manageable
                    let truncated_output = Self::truncate_output(&output, 2000);

                    step.observation = Some(AgentObservation {
                        result: output.clone(),
                        success: true,
                        error: None,
                    });

                    // Add observation to context for next iteration
                    context_messages.push(Message {
                        role: "assistant".to_string(),
                        content: format!(
                            "Command executed: {}\nOutput:\n{}",
                            response.command, truncated_output
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                    });
                }
                CommandResult::Error(error) => {
                    consecutive_errors += 1;

                    step.observation = Some(AgentObservation {
                        result: String::new(),
                        success: false,
                        error: Some(error.clone()),
                    });

                    // Add detailed error context to help AI recover
                    context_messages.push(Message {
                        role: "assistant".to_string(),
                        content: format!(
                            "Command FAILED: {}\nError: {}\n\nPlease try an alternative approach or explain what went wrong.",
                            response.command, error
                        ),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                    });
                }
            }

            steps.push(step);
        }

        Ok(steps)
    }

    /// Truncate long output to keep context manageable
    fn truncate_output(output: &str, max_chars: usize) -> String {
        if output.len() <= max_chars {
            return output.to_string();
        }

        let half = max_chars / 2;
        let start = &output[..half];
        let end = &output[output.len() - half..];

        format!(
            "{}\n\n... [Output truncated: {} characters omitted] ...\n\n{}",
            start,
            output.len() - max_chars,
            end
        )
    }
}

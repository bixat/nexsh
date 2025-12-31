use crate::{
    ai_client::AIClient,
    command_executor::CommandExecutor,
    types::{ActionType, AgentObservation, AgentStep, CommandResult, Message},
};
use std::error::Error;

const MAX_ITERATIONS: usize = 10; // Prevent infinite loops

pub struct ReActLoop {
    max_iterations: usize,
}

impl Default for ReActLoop {
    fn default() -> Self {
        Self {
            max_iterations: MAX_ITERATIONS,
        }
    }
}

impl ReActLoop {
    pub fn new(max_iterations: usize) -> Self {
        Self { max_iterations }
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
        let mut context_messages = messages.to_vec();

        // Add user input to context
        context_messages.push(Message {
            role: "user".to_string(),
            content: user_input.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        });

        loop {
            iteration += 1;

            if iteration > self.max_iterations {
                // Don't print here - let the caller handle display
                break;
            }

            // Get AI response
            let response = ai_client
                .process_command_request(user_input, &context_messages)
                .await?;

            // Create step
            let mut step = AgentStep {
                thought: crate::types::AgentThought {
                    reasoning: response.message.clone(),
                    plan: String::new(),
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
                // Don't print here - let the caller handle display
                steps.push(step);
                break;
            }

            // Execute command (no printing here - let caller handle display)
            match command_executor.execute(&response.command)? {
                CommandResult::Success(output) => {
                    // Create observation
                    step.observation = Some(AgentObservation {
                        result: output.clone(),
                        success: true,
                        error: None,
                    });

                    // Add observation to context for next iteration
                    context_messages.push(Message {
                        role: "assistant".to_string(),
                        content: format!("I executed: {}\nResult: {}", response.command, output),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                    });
                }
                CommandResult::Error(error) => {
                    // Don't print here - let the caller handle display

                    // Create error observation
                    step.observation = Some(AgentObservation {
                        result: String::new(),
                        success: false,
                        error: Some(error.clone()),
                    });

                    // Add error to context so agent can try to fix it
                    context_messages.push(Message {
                        role: "assistant".to_string(),
                        content: format!("I executed: {}\nError: {}", response.command, error),
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
}

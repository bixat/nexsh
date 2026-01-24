use crate::{
    ai_client::AIClient,
    command_executor::CommandExecutor,
    types::{ActionType, AgentObservation, AgentStep, CommandResult, GeminiResponse, Message},
};
use colored::*;
use std::error::Error;

pub struct ReActAgent {
    ai_client: AIClient,
    command_executor: CommandExecutor,
    max_iterations: usize,
}

impl ReActAgent {
    pub fn new(ai_client: AIClient, max_iterations: usize) -> Self {
        Self {
            ai_client,
            command_executor: CommandExecutor::new(),
            max_iterations,
        }
    }

    /// Execute the ReAct loop: Thought -> Action -> Observation
    pub async fn execute(
        &self,
        input: &str,
        messages: &[Message],
    ) -> Result<Vec<AgentStep>, Box<dyn Error>> {
        let mut steps = Vec::new();
        let mut iteration = 0;

        loop {
            if iteration >= self.max_iterations {
                eprintln!(
                    "{}",
                    "⚠️  Maximum iterations reached. Stopping ReAct loop.".yellow()
                );
                break;
            }

            // Get AI response (Thought + Action)
            let response = self
                .ai_client
                .process_command_request(input, messages)
                .await?;

            // Display thought process
            self.display_thought(&response);

            // Determine action type based on response
            let action_type = if response.command.is_empty() {
                ActionType::Respond
            } else {
                ActionType::Execute
            };

            // Create observation based on action
            let observation = match action_type {
                ActionType::Execute => {
                    // Execute the command and observe the result
                    self.execute_and_observe(&response.command).await?
                }
                ActionType::Respond => {
                    // No execution needed, just respond
                    Some(AgentObservation {
                        result: response.message.clone(),
                        success: true,
                        error: None,
                    })
                }
                ActionType::Clarify => {
                    // Request clarification
                    Some(AgentObservation {
                        result: response.message.clone(),
                        success: true,
                        error: None,
                    })
                }
            };

            // Create step record
            let step = AgentStep {
                thought: crate::types::AgentThought {
                    reasoning: response.message.clone(),
                    plan: format!("Action: {:?}", action_type),
                },
                action: crate::types::AgentAction {
                    action_type: action_type.clone(),
                    command: response.command.clone(),
                    dangerous: response.dangerous,
                    category: response.category.clone(),
                },
                observation: observation.clone(),
            };

            steps.push(step);

            // Break if we're done (no command to execute or it's a response)
            if matches!(action_type, ActionType::Respond | ActionType::Clarify) {
                break;
            }

            iteration += 1;
        }

        Ok(steps)
    }

    fn display_thought(&self, response: &GeminiResponse) {
        println!("{}", "💭 Thought:".cyan().bold());
        println!("   {}", response.message.yellow());
    }

    async fn execute_and_observe(
        &self,
        command: &str,
    ) -> Result<Option<AgentObservation>, Box<dyn Error>> {
        println!("{} {}", "🔧 Action:".green().bold(), command);

        match self.command_executor.execute(command)? {
            CommandResult::Success(output) => Ok(Some(AgentObservation {
                result: if output.is_empty() {
                    "Command executed successfully".to_string()
                } else {
                    output
                },
                success: true,
                error: None,
            })),
            CommandResult::Error(error) => Ok(Some(AgentObservation {
                result: String::new(),
                success: false,
                error: Some(error),
            })),
        }
    }
}

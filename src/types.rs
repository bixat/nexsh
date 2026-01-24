use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NexShConfig {
    pub api_key: String,
    pub history_size: usize,
    pub max_context_messages: usize,
    pub model: Option<String>,
    #[serde(default = "default_verbose")]
    pub verbose: bool,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

fn default_verbose() -> bool {
    false
}

fn default_max_iterations() -> usize {
    10 // Default to 10 iterations to prevent infinite loops
}

#[derive(Debug, Deserialize)]
pub struct GeminiResponse {
    pub message: String,
    pub command: String,
    pub dangerous: bool,
    pub category: String,
}

#[derive(Debug)]
pub enum CommandResult {
    Success(String),
    Error(String),
}

#[derive(Debug)]
pub struct ExecutionContext {
    pub working_dir: String,
    pub environment: std::collections::HashMap<String, String>,
}

// ReAct Agent Types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentThought {
    /// Your analysis of what the user wants and current situation
    pub reasoning: String,
    /// Your step-by-step plan to accomplish the task
    pub plan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentAction {
    /// Type of action to take
    pub action_type: ActionType,
    /// Shell command to execute (empty if action_type is not Execute)
    #[serde(default)]
    pub command: String,
    /// Whether the command could be potentially harmful
    pub dangerous: bool,
    /// Classification of the command type
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ActionType {
    /// Execute a shell command
    Execute,
    /// Respond to user without executing
    Respond,
    /// Ask for clarification
    Clarify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentObservation {
    pub result: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub thought: AgentThought,
    pub action: AgentAction,
    pub observation: Option<AgentObservation>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReActResponse {
    /// Your thought process for this iteration
    pub thought: AgentThought,
    /// The action you want to take
    pub action: AgentAction,
    /// Optional final answer when you have enough information (only set when action_type is Respond)
    pub final_answer: Option<String>,
}

impl ReActResponse {
    /// Generate JSON schema for this type
    pub fn json_schema() -> String {
        let schema = schemars::schema_for!(ReActResponse);
        serde_json::to_string_pretty(&schema).unwrap_or_default()
    }
}

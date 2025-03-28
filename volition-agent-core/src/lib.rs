// volition-agent-core/src/lib.rs

// Declare modules moved from cli
pub mod api;
pub mod config;

// Re-export core types for easier access by consumers
pub use config::{load_runtime_config, ModelConfig, RuntimeConfig};
pub use models::chat::{ChatMessage, ResponseMessage, Choice, ApiResponse}; // Updated chat exports
pub use models::tools::{ToolDefinition, ToolInput, ToolParameter, ToolParameterType, ToolParametersDefinition, ToolCall, ToolFunction}; // Added ToolCall, ToolFunction

// Add placeholder for ToolProvider trait (to be defined next)
pub use async_trait::async_trait; // Re-export for convenience
use std::path::Path;
use anyhow::Result;

#[async_trait]
pub trait ToolProvider: Send + Sync {
    fn get_tool_definitions(&self) -> Vec<ToolDefinition>;
    async fn execute_tool(&self, tool_name: &str, input: ToolInput, working_dir: &Path) -> Result<String>;
}

// Placeholder Agent struct and output (Phase 3)
use std::sync::Arc;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentOutput {
    pub suggested_summary: Option<String>,
    pub applied_tool_results: Vec<ToolExecutionResult>,
    pub final_state_description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolExecutionResult {
   pub tool_name: String,
   pub input: serde_json::Value, // Store input as JSON value for flexibility
   pub output: String,
   pub status: ToolExecutionStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ToolExecutionStatus { Success, Failure }

pub struct Agent {
    config: RuntimeConfig,
    tool_provider: Arc<dyn ToolProvider>,
    // Add http client etc. later
}

impl Agent {
    pub fn new(config: RuntimeConfig, tool_provider: Arc<dyn ToolProvider>) -> Result<Self> {
        Ok(Self { config, tool_provider })
    }

    // Placeholder run method
    pub async fn run(&self, goal: &str, working_dir: &Path) -> Result<AgentOutput> {
        println!("Agent running with goal: {}", goal);
        println!("Working directory: {:?}", working_dir);
        println!("Using model: {}", self.config.selected_model);
        println!("Available tools:");
        for tool_def in self.tool_provider.get_tool_definitions() {
            println!("  - {}", tool_def.name);
        }

        // --- TODO: Implement core agent loop ---
        // 1. Format initial prompt with goal and tool definitions
        // 2. Call self.api.get_chat_completion(...)
        // 3. Check response for tool calls
        // 4. If tool calls exist:
        //    a. For each call, call self.tool_provider.execute_tool(...)
        //    b. Format tool results
        //    c. Call self.api.get_chat_completion(...) again with results
        // 5. Repeat 3-4 if necessary
        // 6. Format final AgentOutput
        // -----------------------------------------

        // Dummy output for now
        Ok(AgentOutput {
            suggested_summary: Some("Placeholder summary".to_string()),
            applied_tool_results: vec![],
            final_state_description: Some("Placeholder final state".to_string()),
        })
    }
}

// Add module declaration for models
pub mod models {
    pub mod chat;
    pub mod tools;
    // Add other model submodules if created
}

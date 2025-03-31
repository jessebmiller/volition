// volition-agent-core/src/lib.rs

#![doc = include_str!("../../README.md")]

pub mod api;
pub mod config;
pub mod errors;
pub mod strategies;
pub mod tools; // This will likely be removed/refactored for MCP
pub mod utils;
pub mod mcp; // Added MCP module
pub mod providers; // Added providers module

// Add agent module declaration
pub mod agent;

#[cfg(test)]
mod agent_tests;

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};

// Use new AgentConfig, remove RuntimeConfig export
pub use config::{AgentConfig, ModelConfig};
pub use models::chat::{ApiResponse, ChatMessage, Choice};
pub use models::tools::{ // These might change or be removed with MCP
    ToolCall, ToolDefinition, ToolFunction, ToolInput, ToolParameter, ToolParameterType,
    ToolParametersDefinition,
};
pub use strategies::{DelegationInput, DelegationOutput, Strategy};

pub use async_trait::async_trait;

use crate::errors::AgentError;
use crate::strategies::NextStep;

/// Trait defining the interface for providing tools to the [`Agent`].
/// **NOTE:** This will likely be replaced by MCP interactions.
#[async_trait]
pub trait ToolProvider: Send + Sync {
    /// Returns the definitions of all tools available.
    fn get_tool_definitions(&self) -> Vec<ToolDefinition>;
    /// Executes the tool with the given name and input arguments.
    async fn execute_tool(
        &self,
        tool_name: &str,
        input: ToolInput,
        working_dir: &Path,
    ) -> Result<String>;
}

/// Trait defining the interface for handling user interaction needed by the Agent core.
#[async_trait]
pub trait UserInteraction: Send + Sync {
    /// Asks the user a question with optional predefined options.
    /// Returns the user\'s response string.
    /// If options are provided, the implementation should guide the user.
    /// An empty response or case-insensitive \"yes\"/\"y\" typically signifies confirmation.
    async fn ask(&self, prompt: String, options: Vec<String>) -> Result<String>;
}

// --- Structs for Strategy Interaction ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)] // Added Serialize/Deserialize for potential persistence
pub struct AgentState {
    pub messages: Vec<ChatMessage>,
    pub pending_tool_calls: Vec<ToolCall>, // This will likely change with MCP
    // Add other relevant state fields here if needed
}

impl AgentState {
    pub fn new(initial_task: String) -> Self {
        Self {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some(initial_task),
                ..Default::default()
            }],
            pending_tool_calls: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    // This method might need changes for MCP
    pub fn set_tool_calls(&mut self, tool_calls: Vec<ToolCall>) {
        self.pending_tool_calls = tool_calls;
    }

    // This method will likely be replaced by MCP interactions
    pub fn add_tool_results(&mut self, results: Vec<ToolResult>) {
        for result in results {
            self.messages.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(result.output),
                tool_call_id: Some(result.tool_call_id),
                ..Default::default()
            });
        }
        self.pending_tool_calls.clear(); // Clear pending calls after adding results
    }
}

// This struct will likely be replaced by MCP interactions
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: String,
    pub status: ToolExecutionStatus, // Re-use existing enum
}

#[derive(Debug, Clone)]
pub struct DelegationResult {
    pub result: String,
    // Potentially add artifacts, logs, etc.
}

// --- Old AgentOutput Structs (kept for reference, potentially remove later) ---

/// Represents the final output of an [`Agent::run`] execution. (Old version)
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentOutput {
    /// A list detailing the results of each tool executed during the run.
    pub applied_tool_results: Vec<ToolExecutionResult>,
    /// The content of the AI\'s final message after all tool calls (if any).
    pub final_state_description: Option<String>,
}

/// Details the execution result of a single tool call within an [`AgentOutput`]. (Old version)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolExecutionResult {
    /// The unique ID associated with the AI\'s request to call this tool.
    pub tool_call_id: String,
    /// The name of the tool that was executed.
    pub tool_name: String,
    /// The input arguments passed to the tool (represented as a JSON value).
    pub input: serde_json::Value,
    /// The string output produced by the tool (or an error message if status is Failure).
    pub output: String,
    /// The status of the execution.
    pub status: ToolExecutionStatus,
}

/// Indicates whether a tool execution succeeded or failed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ToolExecutionStatus {
    /// The tool executed successfully.
    Success,
    /// The tool failed during execution or argument parsing.
    Failure,
}

// --- Agent Struct and Implementation (Now in agent.rs) ---
// Remove old Agent struct definition from here

pub mod models {
    pub mod chat;
    pub mod tools;
}

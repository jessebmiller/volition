// volition-agent-core/src/lib.rs

#![doc = include_str!("../../README.md")]

pub mod agent;
pub mod api;
pub mod config;
pub mod errors;
pub mod mcp;
pub mod providers;
pub mod strategies;
pub mod tools;
pub mod utils;

#[cfg(test)]
mod agent_tests;

use anyhow::Result;
use std::path::Path;

pub use config::{AgentConfig, ModelConfig};
pub use models::chat::{ApiResponse, ChatMessage, Choice};
pub use models::tools::{
    ToolCall,
    ToolDefinition,
    ToolFunction,
    ToolInput,
    ToolParameter,
    ToolParameterType,
    ToolParametersDefinition,
};
pub use strategies::{DelegationInput, DelegationOutput, Strategy};

pub use async_trait::async_trait;

/// Trait defining the interface for providing tools to the [`Agent`].
/// **NOTE:** This is unused by the MCP agent.
#[async_trait]
pub trait ToolProvider: Send + Sync {
    fn get_tool_definitions(&self) -> Vec<ToolDefinition>;
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
    async fn ask(&self, prompt: String, options: Vec<String>) -> Result<String>;
}

// --- Structs for Strategy Interaction ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentState {
    pub messages: Vec<ChatMessage>,
    // This field is specific to the old tool system
    pub pending_tool_calls: Vec<ToolCall>,
}

impl AgentState {
    // New constructor for interactive turns or starting with history
    pub fn new_turn(history: Option<Vec<ChatMessage>>, current_user_input: String) -> Self {
        let mut messages = history.unwrap_or_default(); // Start with history or empty vec
        // Only add user message if input is not empty
        if !current_user_input.is_empty() {
            messages.push(ChatMessage {
                role: "user".to_string(),
                content: Some(current_user_input),
                ..Default::default()
            });
        }
        Self {
            messages,
            pending_tool_calls: Vec::new(),
        }
    }

    // Keep the old `new` for compatibility? Or rename? Let's remove it for now.
    // pub fn new(initial_task: String) -> Self { ... }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn set_tool_calls(&mut self, tool_calls: Vec<ToolCall>) {
        self.pending_tool_calls = tool_calls;
    }

    pub fn add_tool_results(&mut self, results: Vec<ToolResult>) {
        for result in results {
            self.messages.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(result.output),
                tool_call_id: Some(result.tool_call_id),
                ..Default::default()
            });
        }
        self.pending_tool_calls.clear();
    }
}

// This struct is specific to the old tool system
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: String,
    pub status: ToolExecutionStatus,
}

#[derive(Debug, Clone)]
pub struct DelegationResult {
    pub result: String,
}

// --- Old AgentOutput Structs (Unused by MCP agent) ---

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentOutput {
    pub applied_tool_results: Vec<ToolExecutionResult>,
    pub final_state_description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolExecutionResult {
    pub tool_call_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: String,
    pub status: ToolExecutionStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ToolExecutionStatus {
    Success,
    Failure,
}

pub mod models {
    pub mod chat;
    pub mod tools;
}

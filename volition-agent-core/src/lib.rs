// volition-agent-core/src/lib.rs

#![doc = include_str!("../../README.md")] // Corrected path

pub mod api;
pub mod config;
// pub mod models; // REMOVED!
pub mod tools;

#[cfg(test)]
mod agent_tests;

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, error};

pub use config::{ModelConfig, RuntimeConfig};
pub use models::chat::{ApiResponse, Choice, ChatMessage};
pub use models::tools::{
    ToolCall, ToolDefinition, ToolFunction, ToolInput, ToolParameter, ToolParameterType,
    ToolParametersDefinition,
};

pub use async_trait::async_trait;

/// Trait defining the interface for providing tools to the [`Agent`].
#[async_trait]
pub trait ToolProvider: Send + Sync {
    /// Returns the definitions of all tools available.
    fn get_tool_definitions(&self) -> Vec<ToolDefinition>;
    /// Executes the tool with the given name and input arguments.
    async fn execute_tool(&self, tool_name: &str, input: ToolInput, working_dir: &Path) -> Result<String>;
}

/// Represents the final output of an [`Agent::run`] execution.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentOutput {
    /// A list detailing the results of each tool executed during the run.
    pub applied_tool_results: Vec<ToolExecutionResult>,
    /// The content of the AI's final message after all tool calls (if any).
    pub final_state_description: Option<String>,
}

/// Details the execution result of a single tool call within an [`AgentOutput`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolExecutionResult {
    /// The unique ID associated with the AI's request to call this tool.
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

use reqwest::Client;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

const MAX_ITERATIONS: usize = 10;

/// The main struct for interacting with the AI agent.
pub struct Agent {
    config: RuntimeConfig,
    tool_provider: Arc<dyn ToolProvider>,
    http_client: Client,
}

impl Agent {
    /// Creates a new `Agent` instance.
    pub fn new(config: RuntimeConfig, tool_provider: Arc<dyn ToolProvider>) -> Result<Self> {
        let http_client = Client::builder()
            .build()
            .context("Failed to build HTTP client for Agent")?;
        Ok(Self {
            config,
            tool_provider,
            http_client,
        })
    }

    /// Runs the agent to achieve a given goal.
    pub async fn run(&self, goal: &str, working_dir: &Path) -> Result<AgentOutput> {
        info!(agent_goal = goal, working_dir = ?working_dir, "Starting agent run.");

        let mut messages: Vec<ChatMessage> = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(self.config.system_prompt.clone()),
                ..Default::default()
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(goal.to_string()),
                ..Default::default()
            },
        ];

        let mut iteration = 0;
        let mut collected_tool_results: Vec<ToolExecutionResult> = Vec::new();

        loop {
            if iteration >= MAX_ITERATIONS {
                error!("Agent reached maximum iteration limit ({})", MAX_ITERATIONS);
                return Err(anyhow!(
                    "Agent stopped after reaching maximum iterations ({})",
                    MAX_ITERATIONS
                ));
            }
            iteration += 1;
            info!(iteration = iteration, "Starting agent iteration.");

            let tool_definitions = self.tool_provider.get_tool_definitions();
            debug!(
                num_tools = tool_definitions.len(),
                "Got tool definitions from provider."
            );

            debug!(
                num_messages = messages.len(),
                "Calling get_chat_completion."
            );
            let api_response = match api::get_chat_completion(
                &self.http_client,
                &self.config,
                messages.clone(),
                &tool_definitions,
            )
            .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    error!(error = ?e, "API call failed during agent run.");
                    return Err(e.context("API call failed during agent run"));
                }
            };

            let choice = api_response.choices.into_iter().next().ok_or_else(|| {
                error!("API response contained no choices.");
                anyhow!("API response contained no choices")
            })?;
            let response_message = choice.message;

            messages.push(response_message.clone());

            if let Some(tool_calls) = response_message.tool_calls {
                info!(num_calls = tool_calls.len(), "Received tool calls from AI.");
                let mut tool_outputs: Vec<ChatMessage> = Vec::new();

                for tool_call in tool_calls {
                    debug!(tool_call_id = %tool_call.id, tool_name = %tool_call.function.name, "Processing tool call.");

                    let input_result: Result<HashMap<String, JsonValue>, serde_json::Error> =
                        serde_json::from_str(&tool_call.function.arguments);

                    let tool_input = match input_result {
                        Ok(args_map) => ToolInput {
                            arguments: args_map,
                        },
                        Err(e) => {
                            error!(tool_call_id = %tool_call.id, tool_name = %tool_call.function.name, error = ?e, "Failed to parse tool arguments.");
                            let error_output = format!(
                                "Error parsing arguments for tool '{}': {}. Arguments received: {}",
                                tool_call.function.name, e, tool_call.function.arguments
                            );
                            tool_outputs.push(ChatMessage {
                                role: "tool".to_string(),
                                content: Some(error_output.clone()),
                                tool_call_id: Some(tool_call.id.clone()),
                                ..Default::default()
                            });
                            collected_tool_results.push(ToolExecutionResult {
                                tool_call_id: tool_call.id,
                                tool_name: tool_call.function.name,
                                input: serde_json::from_str(&tool_call.function.arguments)
                                    .unwrap_or_default(),
                                output: error_output,
                                status: ToolExecutionStatus::Failure,
                            });
                            continue;
                        }
                    };

                    let execution_result = self
                        .tool_provider
                        .execute_tool(&tool_call.function.name, tool_input.clone(), working_dir)
                        .await;

                    let (output_str, status) = match execution_result {
                        Ok(output) => {
                            info!(tool_call_id = %tool_call.id, tool_name = %tool_call.function.name, "Tool executed successfully.");
                            (output, ToolExecutionStatus::Success)
                        }
                        Err(e) => {
                            error!(tool_call_id = %tool_call.id, tool_name = %tool_call.function.name, error = ?e, "Tool execution failed.");
                            (
                                format!(
                                    "Error executing tool '{}': {}",
                                    tool_call.function.name, e
                                ),
                                ToolExecutionStatus::Failure,
                            )
                        }
                    };

                    tool_outputs.push(ChatMessage {
                        role: "tool".to_string(),
                        content: Some(output_str.clone()),
                        tool_call_id: Some(tool_call.id.clone()),
                        ..Default::default()
                    });

                    collected_tool_results.push(ToolExecutionResult {
                        tool_call_id: tool_call.id,
                        tool_name: tool_call.function.name,
                        input: serde_json::to_value(tool_input.arguments).unwrap_or_default(),
                        output: output_str,
                        status,
                    });
                }

                messages.extend(tool_outputs);
                debug!("Added tool outputs to messages, continuing loop.");
                continue;
            }

            info!("Received final response from AI.");
            let final_description = response_message.content;

            let agent_output = AgentOutput {
                applied_tool_results: collected_tool_results,
                final_state_description: final_description,
                ..Default::default()
            };

            debug!(output = ?agent_output, "Agent run finished.");
            return Ok(agent_output);
        }
    }
}

// --- Modules ---
// This is the ONLY definition for the models module
pub mod models {
    pub mod chat;
    pub mod tools;
}

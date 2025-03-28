// volition-agent-core/src/lib.rs
pub mod api;
pub mod config;
// pub mod models; // Ensure this line is REMOVED
pub mod tools;

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, error};

pub use config::{load_runtime_config, ModelConfig, RuntimeConfig};
pub use models::chat::{ApiResponse, Choice, ChatMessage};
pub use models::tools::{
    ToolCall, ToolDefinition, ToolFunction, ToolInput, ToolParameter, ToolParameterType,
    ToolParametersDefinition,
};

pub use async_trait::async_trait;

#[async_trait]
pub trait ToolProvider: Send + Sync {
    fn get_tool_definitions(&self) -> Vec<ToolDefinition>;
    async fn execute_tool(&self, tool_name: &str, input: ToolInput, working_dir: &Path) -> Result<String>;
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AgentOutput {
    pub suggested_summary: Option<String>,
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

use reqwest::Client;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

const MAX_ITERATIONS: usize = 10;

pub struct Agent {
    config: RuntimeConfig,
    tool_provider: Arc<dyn ToolProvider>,
    http_client: Client,
}

impl Agent {
    pub fn new(config: RuntimeConfig, tool_provider: Arc<dyn ToolProvider>) -> Result<Self> {
        let http_client = Client::builder()
            .build()
            .context("Failed to build HTTP client for Agent")?;
        Ok(Self { config, tool_provider, http_client })
    }

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
                return Err(anyhow!("Agent stopped after reaching maximum iterations ({})", MAX_ITERATIONS));
            }
            iteration += 1;
            info!(iteration = iteration, "Starting agent iteration.");

            let tool_definitions = self.tool_provider.get_tool_definitions();
            debug!(num_tools = tool_definitions.len(), "Got tool definitions from provider.");

            debug!(num_messages = messages.len(), "Calling get_chat_completion.");
            let api_response = match api::get_chat_completion(
                &self.http_client,
                &self.config,
                messages.clone(),
                &tool_definitions,
            ).await {
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
                        Ok(args_map) => ToolInput { arguments: args_map },
                        Err(e) => {
                            error!(tool_call_id = %tool_call.id, tool_name = %tool_call.function.name, error = ?e, "Failed to parse tool arguments.");
                            let error_output = format!("Error parsing arguments for tool '{}': {}. Arguments received: {}",
                                tool_call.function.name, e, tool_call.function.arguments);
                            tool_outputs.push(ChatMessage {
                                role: "tool".to_string(),
                                content: Some(error_output.clone()),
                                tool_call_id: Some(tool_call.id.clone()),
                                ..Default::default()
                            });
                            collected_tool_results.push(ToolExecutionResult {
                                tool_call_id: tool_call.id,
                                tool_name: tool_call.function.name,
                                input: serde_json::from_str(&tool_call.function.arguments).unwrap_or_default(),
                                output: error_output,
                                status: ToolExecutionStatus::Failure,
                            });
                            continue;
                        }
                    };

                    let execution_result = self.tool_provider.execute_tool(
                        &tool_call.function.name,
                        tool_input.clone(),
                        working_dir
                    ).await;

                    let (output_str, status) = match execution_result {
                        Ok(output) => {
                            info!(tool_call_id = %tool_call.id, tool_name = %tool_call.function.name, "Tool executed successfully.");
                            (output, ToolExecutionStatus::Success)
                        }
                        Err(e) => {
                            error!(tool_call_id = %tool_call.id, tool_name = %tool_call.function.name, error = ?e, "Tool execution failed.");
                            (format!("Error executing tool '{}': {}", tool_call.function.name, e), ToolExecutionStatus::Failure)
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
                suggested_summary: Some(goal.to_string()),
                applied_tool_results: collected_tool_results,
                final_state_description: final_description,
            };

            debug!(output = ?agent_output, "Agent run finished.");
            return Ok(agent_output);
        }
    }
}

// --- Modules ---
// This should be the ONLY declaration of the models module
pub mod models {
    pub mod chat;
    pub mod tools;
}

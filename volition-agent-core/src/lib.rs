// volition-agent-core/src/lib.rs

#![doc = include_str!("../../README.md")] // Corrected path

pub mod api;
pub mod config;
// pub mod models; // REMOVED! - Keep removed
pub mod tools;

#[cfg(test)]
mod agent_tests;

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::sync::Arc;
// Ensure trace is imported
use tracing::{debug, error, info, trace, warn};

// --- Remove dotenvy/env imports ---
// use std::env; // REMOVED
// use dotenvy::dotenv; // REMOVED
// --- End Remove ---

pub use config::{ModelConfig, RuntimeConfig};
pub use models::chat::{ApiResponse, ChatMessage, Choice};
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
    async fn execute_tool(
        &self,
        tool_name: &str,
        input: ToolInput,
        working_dir: &Path,
    ) -> Result<String>;
}

// --- Keep UserInteraction Trait ---
/// Trait defining the interface for handling user interaction needed by the Agent core.
#[async_trait]
pub trait UserInteraction: Send + Sync {
    /// Asks the user a question with optional predefined options.
    /// Returns the user's response string.
    /// If options are provided, the implementation should guide the user.
    /// An empty response or case-insensitive "yes"/"y" typically signifies confirmation.
    async fn ask(&self, prompt: String, options: Vec<String>) -> Result<String>;
}
// --- End Keep UserInteraction Trait ---


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
    pub output: String, // Added newline for clarity
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

// --- Remove const MAX_ITERATIONS, DEFAULT_MAX_ITERATIONS, ITERATION_ENV_VAR ---
// const MAX_ITERATIONS: usize = 10; // REMOVED
// const DEFAULT_MAX_ITERATIONS: usize = 20; // REMOVED
// const ITERATION_ENV_VAR: &str = "VOLITION_MAX_ITERATIONS"; // REMOVED
// --- End Remove ---

/// The main struct for interacting with the AI agent.
// --- Add Generic Parameter, ui_handler, and initial_max_iterations fields ---
pub struct Agent<UI: UserInteraction> {
    config: RuntimeConfig,
    tool_provider: Arc<dyn ToolProvider>,
    http_client: Client,
    ui_handler: Arc<UI>,
    initial_max_iterations: usize, // Add field to store the limit
}
// --- End Add ---

// --- Update Agent impl block ---
impl<UI: UserInteraction + 'static> Agent<UI> { // Add 'static bound for Arc
// --- End Update ---

    /// Creates a new `Agent` instance.
    // --- Update `new` signature ---
    pub fn new(
        config: RuntimeConfig,
        tool_provider: Arc<dyn ToolProvider>,
        ui_handler: Arc<UI>,      // Add parameter
        max_iterations: usize, // Add parameter for max iterations
    ) -> Result<Self> {
    // --- End Update ---
        let http_client = Client::builder()
            .build()
            .context("Failed to build HTTP client for Agent")?;
        Ok(Self {
            config,
            tool_provider,
            http_client,
            ui_handler,
            initial_max_iterations: max_iterations, // Store the provided limit
        })
    }

    /// Runs the agent based on the provided message history.
    pub async fn run(
        &self,
        mut messages: Vec<ChatMessage>, // Takes ownership and makes mutable
        working_dir: &Path,
    ) -> Result<AgentOutput> {
        // --- Remove environment variable loading ---
        // dotenv().ok(); // REMOVED
        // --- End Remove ---

        info!(num_initial_messages = messages.len(), working_dir = ?working_dir, "Starting agent run.");

        // Basic validation: Ensure we have some messages to work with.
        if messages.is_empty() {
            error!("Agent run started with empty message history.");
            return Err(anyhow!("Cannot run agent with empty message history"));
        }

        // --- Initialize iteration limit from stored field ---
        let mut current_iteration_limit = self.initial_max_iterations;
        info!(
            limit = current_iteration_limit,
            "Agent iteration limit set to {}.", current_iteration_limit
        );
        // --- End Initialize limit ---

        let mut iteration = 0;
        let mut collected_tool_results: Vec<ToolExecutionResult> = Vec::new();

        loop {
            // --- Keep iteration check and prompting logic ---
            if iteration >= current_iteration_limit {
                warn!(
                    iteration = iteration,
                    limit = current_iteration_limit,
                    "Agent reached iteration limit."
                );

                // Calculate additional iterations (at least 1)
                let additional_iterations = ((current_iteration_limit as f64 / 3.0).ceil() as usize).max(1);

                // --- Update prompt message ---
                let prompt = format!(
                    "Agent reached iteration limit ({current_iteration_limit}). \
                    Would you like to continue for {additional_iterations} more iterations? [Y/n] \
                    (The initial limit is configured when the agent starts)",
                );
                // --- End Update prompt ---

                // Provide options for clarity, though the handler might interpret directly
                let options = vec!["Yes".to_string(), "No".to_string()];

                match self.ui_handler.ask(prompt, options).await {
                    Ok(response) => {
                        let response_lower = response.trim().to_lowercase();
                        if response_lower.is_empty() || response_lower == "yes" || response_lower == "y" {
                            info!(additional = additional_iterations, new_limit = current_iteration_limit + additional_iterations, "User chose to continue agent run.");
                            current_iteration_limit += additional_iterations;
                            // Allow loop to continue with the increased limit
                        } else {
                            info!("User chose to stop agent run at iteration limit.");
                            return Err(anyhow!(
                                "Agent stopped by user after reaching iteration limit ({})",
                                iteration // Report the limit that was actually hit
                            ));
                        }
                    }
                    Err(e) => {
                        error!(error = ?e, "Failed to get user input about continuing iteration.");
                        return Err(e.context(format!(
                            "Failed to get user input at iteration limit ({})",
                            iteration
                        )));
                    }
                }
            }
            // --- End Keep iteration check ---

            iteration += 1;
            info!(
                iteration = iteration,
                limit = current_iteration_limit, // Log current limit
                "Starting agent iteration {}/{}.", iteration, current_iteration_limit
            );

            let tool_definitions = self.tool_provider.get_tool_definitions();
            debug!(
                count = tool_definitions.len(),
                "Providing {} tool definitions to AI.",
                tool_definitions.len()
            );

            // --- Tracing before API call ---
            let model_config = self.config.selected_model_config().expect("Selected model config should be valid");
            debug!(
                model = %model_config.model_name,
                endpoint = %model_config.endpoint,
                num_messages = messages.len(),
                "Sending request to AI model."
            );
            trace!(payload = %serde_json::to_string_pretty(&messages).unwrap_or_else(|e| format!("Serialization error: {}", e)), "Messages sent to API");
            trace!(tools = %serde_json::to_string_pretty(&tool_definitions).unwrap_or_else(|e| format!("Serialization error: {}", e)), "Tools sent to API");
            // ---

            let api_response = match api::get_chat_completion(
                &self.http_client,
                &self.config,
                messages.clone(), // Still need to clone here for the API call
                &tool_definitions,
            )
            .await
            {
                Ok(resp) => {
                    // --- Tracing after successful API call ---
                    debug!("Received successful response from AI.");
                    // Use serde_json::to_string_pretty for better readability
                    trace!(response = %serde_json::to_string_pretty(&resp).unwrap_or_else(|e| format!("Serialization error: {}", e)), "Full API Response Body");
                    // ---
                    resp
                }
                Err(e) => {
                    error!(error = ?e, "API call failed during agent run.");
                    // --- Tracing after failed API call ---
                    // Log the error contextually
                    trace!(error = %e, "API call failed.");
                    // ---
                    return Err(e.context("API call failed during agent run"));
                }
            };

            let choice = api_response.choices.into_iter().next().ok_or_else(|| {
                error!("API response contained no choices.");
                anyhow!("API response contained no choices")
            })?;
            let response_message = choice.message;

            // --- Tracing the assistant's response message ---
            debug!("Received assistant message from AI.");
            trace!(message = %serde_json::to_string_pretty(&response_message).unwrap_or_else(|e| format!("Serialization error: {}", e)), "Assistant Message Details");
            // ---

            // Add the assistant's response (potentially with tool calls) to our history
            messages.push(response_message.clone());

            if let Some(tool_calls) = response_message.tool_calls {
                // --- Tracing detected tool calls ---
                info!(
                    count = tool_calls.len(),
                    "AI requested {} tool call(s).",
                    tool_calls.len()
                );
                trace!(tool_calls = %serde_json::to_string_pretty(&tool_calls).unwrap_or_else(|e| format!("Serialization error: {}", e)), "Tool Call Details");
                // ---
                let mut tool_outputs: Vec<ChatMessage> = Vec::new();

                for tool_call in tool_calls {
                    // --- Tracing individual tool call processing ---
                    // Clone the name early if needed multiple times
                    let tool_name = tool_call.function.name.clone();
                    debug!(tool_call_id = %tool_call.id, tool_name = %tool_name, "Processing request for tool '{}'.", tool_name);
                    trace!(arguments = %tool_call.function.arguments, "Raw Tool Arguments for '{}'", tool_name);
                    // ---

                    let input_result: Result<HashMap<String, JsonValue>, serde_json::Error> =
                        serde_json::from_str(&tool_call.function.arguments);

                    let tool_input = match input_result {
                        Ok(args_map) => ToolInput {
                            arguments: args_map,
                        },
                        Err(e) => {
                            error!(tool_call_id = %tool_call.id, tool_name = %tool_name, error = ?e, "Failed to parse arguments for tool '{}'.", tool_name);
                            let error_output = format!(
                                "Error parsing arguments for tool '{}': {}. Arguments received: {}",
                                tool_name, e, tool_call.function.arguments
                            );
                            // --- Tracing argument parsing failure ---
                            trace!(tool_call_id = %tool_call.id, error = %e, arguments = %tool_call.function.arguments, "Tool argument parsing failed for '{}'.", tool_name);
                            // ---
                            tool_outputs.push(ChatMessage {
                                role: "tool".to_string(),
                                content: Some(error_output.clone()),
                                tool_call_id: Some(tool_call.id.clone()),
                                ..Default::default()
                            });
                            collected_tool_results.push(ToolExecutionResult {
                                tool_call_id: tool_call.id,
                                tool_name: tool_name, // Use the cloned name
                                input: serde_json::from_str(&tool_call.function.arguments)
                                    .unwrap_or_else(|_| {
                                        JsonValue::String("Invalid JSON".to_string())
                                    }),
                                output: error_output,
                                status: ToolExecutionStatus::Failure,
                            });
                            continue; // Move to the next tool call
                        }
                    };

                    // --- Tracing before tool execution ---
                    debug!(tool_call_id = %tool_call.id, tool_name = %tool_name, "Executing tool: '{}'", tool_name);
                    trace!(input = %serde_json::to_string_pretty(&tool_input.arguments).unwrap_or_default(), "Parsed Input for tool '{}'", tool_name);
                    // ---

                    let execution_result = self
                        .tool_provider
                        .execute_tool(&tool_name, tool_input.clone(), working_dir)
                        .await;

                    let (output_str, status) = match execution_result {
                        Ok(output) => {
                            // --- Tracing successful tool execution ---
                            info!(tool_call_id = %tool_call.id, tool_name = %tool_name, "Tool '{}' executed successfully.", tool_name);
                            // Avoid overly long outputs in default trace, maybe truncate or summarize? For now, log full.
                            trace!(tool_call_id = %tool_call.id, output = %output, "Output from tool '{}'", tool_name);
                            // ---
                            (output, ToolExecutionStatus::Success)
                        }
                        Err(e) => {
                            // --- Tracing failed tool execution ---
                            error!(tool_call_id = %tool_call.id, tool_name = %tool_name, error = ?e, "Execution failed for tool '{}'.", tool_name);
                            trace!(tool_call_id = %tool_call.id, error = %e, "Error during execution of tool '{}'", tool_name);
                            // ---
                            (
                                format!(
                                    "Error executing tool '{}': {}",
                                    tool_name, e
                                ),
                                ToolExecutionStatus::Failure,
                            )
                        }
                    };

                    // Create a tool message with the execution output/error
                    let tool_output_message = ChatMessage {
                        role: "tool".to_string(),
                        content: Some(output_str.clone()),
                        tool_call_id: Some(tool_call.id.clone()),
                        ..Default::default()
                    };
                    // --- Tracing the tool output message being added ---
                    trace!(message = %serde_json::to_string_pretty(&tool_output_message).unwrap_or_default(), "Tool Output Message for '{}'", tool_name);
                    // ---
                    tool_outputs.push(tool_output_message);

                    // Log the result (success or failure)
                    let exec_result_log = ToolExecutionResult {
                        tool_call_id: tool_call.id,
                        tool_name: tool_name.clone(), // CLONE HERE to avoid move
                        // Convert arguments map back to JsonValue for storing
                        input: serde_json::to_value(tool_input.arguments)
                            .unwrap_or(JsonValue::Null),
                        output: output_str,
                        status,
                    };
                    // --- Tracing the collected tool result ---
                    trace!(result = %serde_json::to_string_pretty(&exec_result_log).unwrap_or_default(), "Collected Execution Result for '{}'", tool_name);
                    // ---
                    collected_tool_results.push(exec_result_log);
                }

                // Add all tool results to the message history
                messages.extend(tool_outputs.clone()); // Clone here if needed below, otherwise move
                                                       // --- Tracing adding tool outputs to history ---
                debug!(
                    count = tool_outputs.len(),
                    "Added {} tool output(s) to messages, continuing loop.",
                    tool_outputs.len()
                );
                trace!(tool_outputs = %serde_json::to_string_pretty(&tool_outputs).unwrap_or_default(), "Tool Output Messages Added to History");
                // ---
                continue; // Go back to the API with the updated history including tool results
            }

            // If there were no tool calls, the loop terminates.
            info!("Received final response from AI (no further tool calls requested).");
            let final_description = response_message.content; // Extract final content

            let agent_output = AgentOutput {
                applied_tool_results: collected_tool_results,
                final_state_description: final_description,
                // Note: suggested_summary is no longer part of AgentOutput
            };

            // --- Tracing final agent output ---
            debug!("Agent run finished successfully.");
            trace!(output = %serde_json::to_string_pretty(&agent_output).unwrap_or_default(), "Final Agent Output");
            // ---
            return Ok(agent_output);
        }
    }
}

// --- Modules ---
// Ensure the models module definition is present if required
pub mod models {
    pub mod chat;
    pub mod tools;
}

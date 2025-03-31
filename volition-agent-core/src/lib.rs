// volition-agent-core/src/lib.rs

#![doc = include_str!("../../README.md")]

pub mod api;
pub mod config;
pub mod errors;
pub mod strategies;
pub mod tools;
pub mod utils;

#[cfg(test)]
mod agent_tests;

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};

pub use config::{ModelConfig, RuntimeConfig};
pub use models::chat::{ApiResponse, ChatMessage, Choice};
pub use models::tools::{
    ToolCall, ToolDefinition, ToolFunction, ToolInput, ToolParameter, ToolParameterType,
    ToolParametersDefinition,
};
pub use strategies::{DelegationInput, DelegationOutput, Strategy};

pub use async_trait::async_trait;

use crate::errors::AgentError;
use crate::strategies::NextStep;

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
    pub pending_tool_calls: Vec<ToolCall>,
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
        self.pending_tool_calls.clear(); // Clear pending calls after adding results
    }
}

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

// --- Agent Struct and Implementation ---

use reqwest::Client;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// The main struct for interacting with the AI agent using Strategies.
pub struct Agent<UI: UserInteraction> {
    config: RuntimeConfig,
    tool_provider: Arc<dyn ToolProvider>,
    http_client: Client,
    #[allow(dead_code)] // Keep UI handler even if unused in current loop
    ui_handler: Arc<UI>,
    strategy: Box<dyn Strategy + Send + Sync>,
    state: AgentState,
}

impl<UI: UserInteraction + 'static> Agent<UI> {
    /// Creates a new `Agent` instance with a specific strategy.
    pub fn new(
        config: RuntimeConfig,
        tool_provider: Arc<dyn ToolProvider>,
        ui_handler: Arc<UI>,
        strategy: Box<dyn Strategy + Send + Sync>,
        initial_task: String,
    ) -> Result<Self> {
        let http_client = Client::builder()
            .build()
            .context("Failed to build HTTP client for Agent")?;
        let initial_state = AgentState::new(initial_task);
        info!(
            strategy = strategy.name(),
            "Initializing Agent with strategy."
        );
        Ok(Self {
            config,
            tool_provider,
            http_client,
            ui_handler,
            strategy,
            state: initial_state,
        })
    }

    /// Creates a new `Agent` instance initialized with existing conversation context.
    /// The passed `strategy` should ideally be a `ConversationStrategy` wrapping
    /// the desired inner strategy and initialized with the `conversation_state`.
    /// The `Agent`'s internal state is initialized only with the `new_user_message`.
    pub fn with_conversation_state(
        config: RuntimeConfig,
        tool_provider: Arc<dyn ToolProvider>,
        ui_handler: Arc<UI>,
        strategy: Box<dyn Strategy + Send + Sync>,
        // conversation_state: AgentState, // State is managed *within* the ConversationStrategy now
        new_user_message: String,
    ) -> Result<Self> {
        let http_client = Client::builder()
            .build()
            .context("Failed to build HTTP client for Agent")?;

        // Create a new state with just the new user message.
        // The ConversationStrategy (passed in as `strategy`) is responsible
        // for merging this with the historical context during initialize_interaction.
        let initial_state = AgentState::new(new_user_message);

        info!(
            strategy = strategy.name(),
            "Initializing Agent with strategy and existing conversation context." // Updated log message
        );

        Ok(Self {
            config,
            tool_provider,
            http_client,
            ui_handler,
            strategy,
            state: initial_state, // Agent state starts with just the new message
        })
    }


    /// Runs the agent's strategy loop until completion or error.
    /// Returns the final message and the final AgentState.
    pub async fn run(&mut self, working_dir: &Path) -> Result<(String, AgentState), AgentError> { // <-- MODIFIED Return Type
        info!(working_dir = ?working_dir, strategy = self.strategy.name(), "Starting agent run.");

        // The strategy's initialize_interaction is responsible for setting up the initial state,
        // potentially merging history if it's a ConversationStrategy.
        let mut next_step = self
            .strategy
            .initialize_interaction(&mut self.state)
            .map_err(|e| {
                AgentError::Strategy(format!(
                    "Initialization failed for {}: {}",
                    self.strategy.name(),
                    e
                ))
            })?;

        loop {
            trace!(next_step = ?next_step, "Processing next step.");
            match next_step {
                NextStep::CallApi(state_from_strategy) => {
                    self.state = state_from_strategy; // Update agent state

                    let tool_definitions = self.tool_provider.get_tool_definitions();
                    debug!(
                        count = tool_definitions.len(),
                        "Providing {} tool definitions to AI.",
                        tool_definitions.len()
                    );

                    let model_config = match self.config.selected_model_config() {
                        Ok(config) => config,
                        Err(e) => {
                            return Err(AgentError::Config(format!(
                                "Failed to get selected model config: {}",
                                e
                            )));
                        }
                    };

                    debug!(
                        model = %model_config.model_name,
                        endpoint = %model_config.endpoint,
                        num_messages = self.state.messages.len(),
                        "Sending request to AI model."
                    );
                    trace!(payload = %serde_json::to_string_pretty(&self.state.messages).unwrap_or_else(|e| format!("Serialization error: {}", e)), "Messages sent to API");
                    trace!(tools = %serde_json::to_string_pretty(&tool_definitions).unwrap_or_else(|e| format!("Serialization error: {}", e)), "Tools sent to API");

                    let api_response = match api::get_chat_completion(
                        &self.http_client,
                        &self.config,
                        self.state.messages.clone(), // Clone messages for the API call
                        &tool_definitions,
                    )
                    .await
                    {
                        Ok(resp) => {
                            debug!("Received successful response from AI.");
                            trace!(response = %serde_json::to_string_pretty(&resp).unwrap_or_else(|e| format!("Serialization error: {}", e)), "Full API Response Body");
                            resp
                        }
                        Err(e) => {
                            error!(error = ?e, "API call failed during agent run.");
                            return Err(AgentError::Api(
                                e.context("API call failed during agent run"),
                            ));
                        }
                    };

                    // Pass the API response to the strategy for processing
                    next_step = self
                        .strategy
                        .process_api_response(&mut self.state, api_response)
                        .map_err(|e| {
                            AgentError::Strategy(format!(
                                "Processing API response failed for {}: {}",
                                self.strategy.name(),
                                e
                            ))
                        })?;
                }
                NextStep::CallTools(state_from_strategy) => {
                    self.state = state_from_strategy; // Update agent state
                    let tool_calls = self.state.pending_tool_calls.clone(); // Clone to avoid borrow issues

                    if tool_calls.is_empty() {
                        warn!("Strategy requested tool calls, but none were pending in the state.");
                        // Decide if this is an error or just needs a re-prompt
                        // For now, treat as strategy error
                        return Err(AgentError::Strategy(
                            "Strategy requested CallTools, but no tools were pending.".to_string(),
                        ));
                    }

                    info!(
                        count = tool_calls.len(),
                        "Executing {} requested tool call(s).",
                        tool_calls.len()
                    );

                    let mut tool_results: Vec<ToolResult> = Vec::new();

                    for tool_call in tool_calls {
                        let tool_name = tool_call.function.name.clone();
                        let tool_call_id = tool_call.id.clone();
                        debug!(tool_call_id = %tool_call_id, tool_name = %tool_name, "Processing request for tool \"{}\".", tool_name);
                        trace!(arguments = %tool_call.function.arguments, "Raw Tool Arguments for \"{}\"", tool_name);

                        let tool_input_result: Result<HashMap<String, JsonValue>, _> =
                            serde_json::from_str(&tool_call.function.arguments);

                        let tool_result = match tool_input_result {
                            Ok(args_map) => {
                                let tool_input = ToolInput {
                                    arguments: args_map,
                                };
                                debug!(tool_call_id = %tool_call_id, tool_name = %tool_name, "Executing tool: \"{}\"", tool_name);
                                trace!(input = %serde_json::to_string_pretty(&tool_input.arguments).unwrap_or_default(), "Parsed Input for tool \"{}\"", tool_name);

                                match self
                                    .tool_provider
                                    .execute_tool(&tool_name, tool_input.clone(), working_dir)
                                    .await
                                {
                                    Ok(output) => {
                                        info!(tool_call_id = %tool_call_id, tool_name = %tool_name, "Tool \"{}\" executed successfully.", tool_name);
                                        trace!(tool_call_id = %tool_call_id, output = %output, "Output from tool \"{}\"", tool_name);
                                        ToolResult {
                                            tool_call_id: tool_call_id.clone(),
                                            output,
                                            status: ToolExecutionStatus::Success,
                                        }
                                    }
                                    Err(e) => {
                                        error!(tool_call_id = %tool_call_id, tool_name = %tool_name, error = ?e, "Execution failed for tool \"{}\".", tool_name);
                                        ToolResult {
                                            tool_call_id: tool_call_id.clone(),
                                            output: format!(
                                                "Error executing tool \"{}\": {}",
                                                tool_name, e
                                            ),
                                            status: ToolExecutionStatus::Failure,
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!(tool_call_id = %tool_call_id, tool_name = %tool_name, error = ?e, "Failed to parse arguments for tool \"{}\".", tool_name);
                                ToolResult {
                                    tool_call_id: tool_call_id.clone(),
                                    output: format!(
                                        "Error parsing arguments for tool \"{}\": {}. Arguments received: {}",
                                        tool_name, e, tool_call.function.arguments
                                    ),
                                    status: ToolExecutionStatus::Failure,
                                }
                            }
                        };
                        trace!(tool_call_id = %tool_result.tool_call_id, result = ?tool_result.status, "Collected Tool Result");
                        tool_results.push(tool_result);
                    }

                    debug!(
                        count = tool_results.len(),
                        "Passing {} tool result(s) back to strategy.",
                        tool_results.len()
                    );
                    // Pass tool results to the strategy for processing
                    next_step = self
                        .strategy
                        .process_tool_results(&mut self.state, tool_results)
                        .map_err(|e| {
                            AgentError::Strategy(format!(
                                "Processing tool results failed for {}: {}",
                                self.strategy.name(),
                                e
                            ))
                        })?;
                }
                NextStep::DelegateTask(delegation_input) => {
                    // Update agent state before delegation if needed
                    // self.state = delegation_input.current_state; // Assuming input carries state

                    warn!(task = ?delegation_input.task_description, "Delegation requested, but not yet implemented.");
                    // Placeholder implementation
                    let delegation_result = DelegationResult {
                        result: "Delegation is not implemented in this agent.".to_string(),
                    };
                    // Pass delegation result back to the strategy
                    next_step = self
                        .strategy
                        .process_delegation_result(&mut self.state, delegation_result)
                        .map_err(|e| {
                            AgentError::Delegation(format!(
                                "Processing delegation result failed for {}: {}",
                                self.strategy.name(),
                                e
                            ))
                        })?;
                }
                NextStep::Completed(final_message) => {
                    info!("Strategy indicated completion.");
                    trace!(message = %final_message, "Final message from strategy.");
                    // Return the final message AND the final state
                    return Ok((final_message, self.state.clone())); // <-- MODIFIED Return Value
                }
            }
        }
    }
}

pub mod models {
    pub mod chat;
    pub mod tools;
}

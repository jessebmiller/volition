// volition-agent-core/src/agent.rs
use crate::config::AgentConfig;
use crate::errors::AgentError;
use crate::mcp::McpConnection;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::ToolDefinition;
use crate::providers::{Provider, ProviderRegistry};
use crate::strategies::{NextStep, Strategy};
use crate::UserInteraction;
use anyhow::{anyhow, Context, Result};
use rmcp::model::Tool as McpTool;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};

use crate::AgentState;

pub struct Agent<UI: UserInteraction> {
    provider_registry: ProviderRegistry,
    mcp_connections: HashMap<String, Arc<McpConnection>>,
    http_client: reqwest::Client,
    ui_handler: Arc<UI>,
    strategy: Box<dyn Strategy<UI> + Send + Sync>,
    state: AgentState,
    current_provider_id: String,
}

impl<UI: UserInteraction + 'static> Agent<UI> {
    pub fn new(
        config: AgentConfig,
        ui_handler: Arc<UI>,
        strategy: Box<dyn Strategy<UI> + Send + Sync>,
        initial_task: String,
    ) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .build()
            .context("Failed to build HTTP client for Agent")?;

        let mut provider_registry = ProviderRegistry::new(config.default_provider.clone());
        for (id, provider_conf) in config.providers {
            let api_key = std::env::var(&provider_conf.api_key_env_var)
                .with_context(|| format!("API key env var {} not set for provider {}", provider_conf.api_key_env_var, id))?;

            let provider: Box<dyn Provider> = match provider_conf.provider_type.as_str() {
                "gemini" => Box::new(crate::providers::gemini::GeminiProvider::new(
                    provider_conf.model_config,
                    http_client.clone(),
                    api_key,
                )),
                _ => return Err(anyhow!("Unsupported provider type: {}", provider_conf.provider_type)),
            };
            provider_registry.register(id, provider);
        }

        let mut mcp_connections = HashMap::new();
        for (id, server_conf) in config.mcp_servers {
            let connection = McpConnection::new(server_conf.command, server_conf.args);
            mcp_connections.insert(id, Arc::new(connection));
        }

        let initial_state = AgentState::new(initial_task);
        let default_provider_id = provider_registry.default_provider_id().to_string();

        info!(
            strategy = strategy.name(),
            default_provider = %default_provider_id,
            "Initializing MCP Agent with strategy."
        );

        Ok(Self {
            provider_registry,
            mcp_connections,
            http_client,
            ui_handler,
            strategy,
            state: initial_state,
            current_provider_id: default_provider_id,
        })
    }

    pub fn switch_provider(&mut self, provider_id: &str) -> Result<()> {
        self.provider_registry.get(provider_id)?;
        if self.current_provider_id != provider_id {
             debug!(old_provider = %self.current_provider_id, new_provider = %provider_id, "Switching provider");
             self.current_provider_id = provider_id.to_string();
        }
        Ok(())
    }

    pub async fn get_completion(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        let provider = self.provider_registry.get(&self.current_provider_id)?;
        debug!(provider = %self.current_provider_id, num_messages = messages.len(), "Getting completion from provider");
        provider.get_completion(messages).await
    }

    pub async fn call_mcp_tool(&self, server_id: &str, tool_name: &str, args: Value) -> Result<Value> {
        let connection = self.mcp_connections.get(server_id)
            .ok_or_else(|| anyhow!("MCP server connection not found: {}", server_id))?;
        debug!(server = %server_id, tool = %tool_name, "Calling MCP tool");
        connection.call_tool(tool_name, args).await
    }

     pub async fn get_mcp_resource(&self, server_id: &str, uri: &str) -> Result<Value> {
        let connection = self.mcp_connections.get(server_id)
            .ok_or_else(|| anyhow!("MCP server connection not found: {}", server_id))?;
        debug!(server = %server_id, uri = %uri, "Getting MCP resource");
        connection.get_resource(uri).await
    }

    pub async fn list_mcp_tools(&self) -> Result<Vec<McpTool>> {
        let mut all_tools = Vec::new();
        for (id, conn) in &self.mcp_connections {
            match conn.list_tools().await {
                Ok(tools) => all_tools.extend(tools),
                Err(e) => warn!(server_id = %id, error = ?e, "Failed to list tools from MCP server"),
            }
        }
        Ok(all_tools)
    }

    pub async fn run(&mut self, _working_dir: &Path) -> Result<(String, AgentState), AgentError> {
        info!(strategy = self.strategy.name(), "Starting MCP agent run.");

        let mut next_step = self.strategy.initialize_interaction(&mut self.state)?;

        loop {
            trace!(?next_step, "Processing next step.");
            match next_step {
                NextStep::CallApi(state_from_strategy) => {
                    self.state = state_from_strategy;
                    let mcp_tools = self.list_mcp_tools().await
                        .map_err(|e| AgentError::Mcp(e.context("Failed to list MCP tools")))?;
                    
                    let tool_definitions: Vec<ToolDefinition> = mcp_tools.iter().map(|mcp_tool| {
                        ToolDefinition {
                            name: mcp_tool.name.to_string(),
                            // Use empty string if description is None
                            description: mcp_tool.description.clone().map(|s| s.to_string()).unwrap_or_default(),
                            parameters: crate::models::tools::ToolParametersDefinition { 
                                param_type: "object".to_string(), 
                                properties: std::collections::HashMap::new(), 
                                required: vec![] 
                            }
                        }
                    }).collect();

                    debug!(
                        provider = %self.current_provider_id,
                        num_messages = self.state.messages.len(),
                        num_tools = tool_definitions.len(),
                        "Sending request to AI provider."
                    );
                    let api_response = self.get_completion(self.state.messages.clone()).await
                        .map_err(|e| AgentError::Api(e.context("API call failed during agent run")))?;

                    debug!("Received response from AI.");
                    trace!(response = %serde_json::to_string_pretty(&api_response).unwrap_or_default(), "Full API Response");

                    next_step = self.strategy.process_api_response(&mut self.state, api_response)?;
                }
                NextStep::CallTools(state_from_strategy) => {
                    self.state = state_from_strategy;
                    let tool_calls = self.state.pending_tool_calls.clone();

                    if tool_calls.is_empty() {
                        warn!("Strategy requested tool calls, but none were pending.");
                        return Err(AgentError::Strategy("No tools pending".to_string()));
                    }

                    info!(count = tool_calls.len(), "Executing {} requested tool call(s) via MCP.", tool_calls.len());

                    let mut tool_results = Vec::new();
                    for tool_call in tool_calls {
                        let tool_name = &tool_call.function.name;
                        let args: Value = serde_json::from_str(&tool_call.function.arguments)
                            .unwrap_or(Value::Null);

                        let server_id = if tool_name.starts_with("git_") {
                            "git"
                        } else if tool_name == "shell" {
                            "shell"
                        } else if tool_name == "search_text" {
                             "search"
                        } else {
                             "filesystem"
                        };
                        
                        match self.call_mcp_tool(server_id, tool_name, args).await {
                            Ok(output_value) => {
                                info!(tool_call_id = %tool_call.id, tool_name = %tool_name, server_id = %server_id, "MCP Tool executed successfully.");
                                let output_str = match output_value {
                                    Value::String(s) => s,
                                    Value::Null => "<no output>".to_string(),
                                    other => serde_json::to_string_pretty(&other).unwrap_or_default(),
                                };
                                tool_results.push(crate::ToolResult {
                                    tool_call_id: tool_call.id.clone(),
                                    output: output_str,
                                    status: crate::ToolExecutionStatus::Success,
                                });
                            }
                            Err(e) => {
                                error!(tool_call_id = %tool_call.id, tool_name = %tool_name, server_id = %server_id, error = ?e, "MCP Tool execution failed.");
                                tool_results.push(crate::ToolResult {
                                    tool_call_id: tool_call.id.clone(),
                                    output: format!("Error executing MCP tool '{}' on server '{}': {}", tool_name, server_id, e),
                                    status: crate::ToolExecutionStatus::Failure,
                                });
                            }
                        }
                    }

                    debug!(count = tool_results.len(), "Passing {} tool result(s) back to strategy.", tool_results.len());
                    next_step = self.strategy.process_tool_results(&mut self.state, tool_results)?;
                }
                NextStep::DelegateTask(delegation_input) => {
                    warn!(task = ?delegation_input.task_description, "Delegation requested, but not yet implemented.");
                    let delegation_result = crate::DelegationResult {
                        result: "Delegation is not implemented.".to_string(),
                    };
                    next_step = self.strategy.process_delegation_result(&mut self.state, delegation_result)?;
                }
                NextStep::Completed(final_message) => {
                    info!("Strategy indicated completion.");
                    trace!(message = %final_message, "Final message from strategy.");
                    return Ok((final_message, self.state.clone()));
                }
            }
        }
    }
}

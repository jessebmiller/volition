// volition-agent-core/src/agent.rs
use crate::config::{AgentConfig, ProviderInstanceConfig}; // Import ProviderInstanceConfig
use crate::errors::AgentError;
use crate::mcp::McpConnection;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::models::tools::{ToolDefinition, ToolParameter, ToolParameterType, ToolParametersDefinition};
use crate::providers::{Provider, ProviderRegistry};
use crate::strategies::{NextStep, Strategy};
use crate::UserInteraction;
use anyhow::{anyhow, Context, Result};
use rmcp::model::Tool as McpTool;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn};

use crate::AgentState;

pub struct Agent<UI: UserInteraction> {
    provider_registry: ProviderRegistry,
    mcp_connections: HashMap<String, Arc<Mutex<McpConnection>>>,
    #[allow(dead_code)] // Field currently unused
    http_client: reqwest::Client,
    #[allow(dead_code)] // Field currently unused
    ui_handler: Arc<UI>,
    strategy: Box<dyn Strategy<UI> + Send + Sync>,
    state: AgentState,
    current_provider_id: String,
}

fn mcp_schema_to_tool_params(schema_val: Option<&Map<String, Value>>) -> ToolParametersDefinition {
    let default_params = ToolParametersDefinition {
        param_type: "object".to_string(),
        properties: HashMap::new(),
        required: Vec::new(),
    };
    let schema = match schema_val {
        Some(s) => s,
        None => return default_params,
    };
    let props_val = schema.get("properties").and_then(Value::as_object);
    let required_val = schema.get("required").and_then(Value::as_array);
    let mut properties = HashMap::new();
    if let Some(props_map) = props_val {
        for (key, val) in props_map {
            if let Some(prop_obj) = val.as_object() {
                let param_type_str = prop_obj.get("type").and_then(Value::as_str).unwrap_or("string");
                let description = prop_obj.get("description").and_then(Value::as_str).unwrap_or("").to_string();
                let param_type = match param_type_str {
                    "string" => ToolParameterType::String,
                    "integer" => ToolParameterType::Integer,
                    "number" => ToolParameterType::Number,
                    "boolean" => ToolParameterType::Boolean,
                    "array" => ToolParameterType::Array,
                    "object" => ToolParameterType::Object,
                    _ => ToolParameterType::String,
                };
                properties.insert(key.clone(), ToolParameter {
                    param_type,
                    description,
                    enum_values: None,
                    items: None, 
                });
            }
        }
    }
    let required = required_val
        .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
        .unwrap_or_default();
    ToolParametersDefinition {
        param_type: "object".to_string(),
        properties,
        required,
    }
}

struct DummyClientService;
impl rmcp::service::Service<rmcp::service::RoleClient> for DummyClientService {
    #[allow(refining_impl_trait)] // Allow Pin<Box<dyn Future>> where trait uses impl Future
    fn handle_request(
        &self,
        _request: rmcp::model::ServerRequest,
        _context: rmcp::service::RequestContext<rmcp::service::RoleClient>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<rmcp::model::ClientResult, rmcp::Error>> + Send>> {
        Box::pin(async { Err(rmcp::Error::method_not_found::<rmcp::model::InitializeResultMethod>()) })
    }
    #[allow(refining_impl_trait)] // Allow Pin<Box<dyn Future>> where trait uses impl Future
    fn handle_notification(
        &self,
        _notification: rmcp::model::ServerNotification,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), rmcp::Error>> + Send>> {
        Box::pin(async { Ok(()) })
    }
    fn get_peer(&self) -> Option<rmcp::service::Peer<rmcp::service::RoleClient>> { None }
    fn set_peer(&mut self, _peer: rmcp::service::Peer<rmcp::service::RoleClient>) {}
    fn get_info(&self) -> rmcp::model::ClientInfo { rmcp::model::ClientInfo::default() }
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
        // Use into_iter to consume the config
        for (id, provider_conf) in config.providers {
            let api_key = if !provider_conf.api_key_env_var.is_empty() {
                 match std::env::var(&provider_conf.api_key_env_var) {
                     Ok(key) => key,
                     Err(e) => {
                         warn!(provider_id = %id, env_var = %provider_conf.api_key_env_var, error = %e, "API key environment variable not set or invalid");
                         String::new() // Use empty string if env var is missing/invalid
                     }
                 }
            } else {
                String::new()
            };
            
            // Extract model_config before matching
            let model_config = provider_conf.model_config; 
            
            let provider: Box<dyn Provider> = match provider_conf.provider_type.as_str() {
                "gemini" => Box::new(crate::providers::gemini::GeminiProvider::new(
                    model_config, // Pass the extracted ModelConfig
                    http_client.clone(),
                    api_key,
                )),
                 "ollama" => Box::new(crate::providers::ollama::OllamaProvider::new(
                    model_config, // Pass the extracted ModelConfig
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
            mcp_connections.insert(id, Arc::new(Mutex::new(connection)));
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

    async fn ensure_mcp_connection(&self, server_id: &str) -> Result<()> {
        let conn_mutex = self.mcp_connections.get(server_id)
            .ok_or_else(|| anyhow!("MCP server config not found: {}", server_id))?;
        let conn_guard = conn_mutex.lock().await;
        let ct = tokio_util::sync::CancellationToken::new(); 
        conn_guard.establish_connection_external(DummyClientService, ct).await
    }

    pub fn switch_provider(&mut self, provider_id: &str) -> Result<()> {
        self.provider_registry.get(provider_id)?;
        if self.current_provider_id != provider_id {
             debug!(old_provider = %self.current_provider_id, new_provider = %provider_id, "Switching provider");
             self.current_provider_id = provider_id.to_string();
        }
        Ok(())
    }

    pub async fn get_completion(&self, messages: Vec<ChatMessage>, tools: Option<&[ToolDefinition]>) -> Result<ApiResponse> {
        let provider = self.provider_registry.get(&self.current_provider_id)?;
        debug!(provider = %self.current_provider_id, num_messages = messages.len(), "Getting completion from provider");
        provider.get_completion(messages, tools).await
    }

    pub async fn call_mcp_tool(&self, server_id: &str, tool_name: &str, args: Value) -> Result<Value> {
        self.ensure_mcp_connection(server_id).await?;
        let conn_mutex = self.mcp_connections.get(server_id).unwrap(); 
        let conn = conn_mutex.lock().await;
        debug!(server = %server_id, tool = %tool_name, "Calling MCP tool");
        conn.call_tool(tool_name, args).await
    }

     pub async fn get_mcp_resource(&self, server_id: &str, uri: &str) -> Result<Value> {
        self.ensure_mcp_connection(server_id).await?;
        let conn_mutex = self.mcp_connections.get(server_id).unwrap(); 
        let conn = conn_mutex.lock().await;
        debug!(server = %server_id, uri = %uri, "Getting MCP resource");
        conn.get_resource(uri).await
    }

    pub async fn list_mcp_tools(&self) -> Result<Vec<McpTool>> {
        let mut all_tools = Vec::new();
        for (id, conn_mutex) in &self.mcp_connections {
            match self.ensure_mcp_connection(id).await {
                 Ok(_) => {
                      let conn = conn_mutex.lock().await;
                      match conn.list_tools().await {
                           Ok(tools) => all_tools.extend(tools),
                           Err(e) => warn!(server_id = %id, error = ?e, "Failed to list tools from MCP server (post-connection)"),
                      }
                 },
                 Err(e) => {
                      warn!(server_id = %id, error = ?e, "Failed to ensure MCP connection for listing tools");
                 }
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
                        let schema_map = mcp_tool.input_schema.as_ref(); 
                        ToolDefinition {
                            name: mcp_tool.name.to_string(),
                            description: mcp_tool.description.clone().map(|s| s.to_string()).unwrap_or_default(),
                            parameters: mcp_schema_to_tool_params(Some(schema_map)), 
                        }
                    }).collect();

                    debug!(
                        provider = %self.current_provider_id,
                        num_messages = self.state.messages.len(),
                        num_tools = tool_definitions.len(),
                        "Sending request to AI provider."
                    );
                    
                    let api_response = self.get_completion(
                        self.state.messages.clone(), 
                        if tool_definitions.is_empty() { None } else { Some(&tool_definitions) }
                    ).await
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
                        return Err(AgentError::Strategy("Strategy requested tool calls, but none were pending in state".to_string()));
                    }

                    info!(count = tool_calls.len(), "Executing {} requested tool call(s) via MCP.", tool_calls.len());

                    let mut tool_results = Vec::new();
                    for tool_call in tool_calls {
                        let tool_name = &tool_call.function.name;
                        let args: Value = serde_json::from_str(&tool_call.function.arguments)
                            .unwrap_or(Value::Null);

                        let server_id = match tool_name.as_str() {
                            "read_file" | "write_file" => "filesystem",
                            "shell" => "shell",
                            "git_diff" | "git_status" => "git",
                            "search_text" => "search",
                            _ => {
                                warn!(tool_name = %tool_name, "Cannot map tool to MCP server, skipping.");
                                tool_results.push(crate::ToolResult {
                                    tool_call_id: tool_call.id.clone(),
                                    output: format!("Error: Unknown tool name '{}'", tool_name),
                                    status: crate::ToolExecutionStatus::Failure,
                                });
                                continue;
                            }
                        };
                        
                        match self.call_mcp_tool(server_id, tool_name, args).await {
                            Ok(output_value) => {
                                info!(tool_call_id = %tool_call.id, tool_name = %tool_name, server_id = %server_id, "MCP Tool executed successfully.");
                                let output_str = match output_value {
                                    Value::String(s) => s,
                                    Value::Object(map) if map.contains_key("text") => { 
                                        map.get("text").and_then(Value::as_str).unwrap_or("").to_string()
                                    }
                                    Value::Array(arr) if arr.is_empty() => "<empty result>".to_string(),
                                    Value::Array(arr) => serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "<invalid JSON array>".to_string()),
                                    Value::Object(map) => serde_json::to_string_pretty(&map).unwrap_or_else(|_| "<invalid JSON object>".to_string()),
                                    Value::Null => "<no output>".to_string(),
                                    other => other.to_string(),
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

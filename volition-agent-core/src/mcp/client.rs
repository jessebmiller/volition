// volition-agent-core/src/mcp/client.rs
use anyhow::{anyhow, Result};
use rmcp::service::{Peer, RoleClient};
use rmcp::model::{
    Annotated, // Added
    CallToolRequestParam, ClientInfo, ClientResult,
    InitializeResultMethod, // Corrected type for method_not_found
    RawContent, // Added
    ReadResourceRequestParam, ReadResourceResult, ServerNotification, ServerRequest,
    Tool,
};
use rmcp::transport::TokioChildProcess;
use rmcp::Error as McpError;
use serde_json::{Map, Value};
use std::borrow::Cow; // Keep Cow for Owned variant
use tokio::process::Command;

// Rename the struct to avoid confusion with rmcp types
pub struct McpConnection {
    server_command: String,
    server_args: Vec<String>,
    // Store the Peer which represents the connection to the server
    peer: Option<Peer<RoleClient>>,
}

// Dummy Service implementation needed for serve_client_with_ct
struct DummyClientService;
impl rmcp::service::Service<RoleClient> for DummyClientService {
    fn handle_request(
        &self,
        _request: ServerRequest,
        _context: rmcp::service::RequestContext<RoleClient>,
    ) -> impl std::future::Future<Output = Result<ClientResult, McpError>> + Send {
        // Specify the corrected method type
        async { Err(McpError::method_not_found::<InitializeResultMethod>()) }
    }
    fn handle_notification(
        &self,
        _notification: ServerNotification,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send {
        async { Ok(()) }
    }
    fn get_peer(&self) -> Option<Peer<RoleClient>> { None }
    fn set_peer(&mut self, _peer: Peer<RoleClient>) {}
    fn get_info(&self) -> ClientInfo { ClientInfo::default() }
}

impl McpConnection {
    pub fn new(server_command: String, server_args: Vec<String>) -> Self {
        Self {
            server_command,
            server_args,
            peer: None,
        }
    }

    pub async fn establish(&mut self) -> Result<()> {
        if self.peer.is_some() {
            return Ok(());
        }
        let mut cmd = Command::new(&self.server_command);
        cmd.args(&self.server_args);

        let transport = TokioChildProcess::new(&mut cmd)?;

        let running_service = rmcp::service::serve_client_with_ct(
            DummyClientService,
            transport,
            Default::default(),
        )
        .await
        .map_err(|e| anyhow!("Failed to establish MCP connection: {}", e))?;

        self.peer = Some(running_service.peer().clone());

        // TODO: Decide how to manage the background task handle
        // let _handle = tokio::spawn(async move { running_service.waiting().await });

        Ok(())
    }

    fn get_peer(&self) -> Result<&Peer<RoleClient>> {
        self.peer.as_ref().ok_or_else(|| anyhow!("MCP connection not established"))
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let peer = self.get_peer()?;
        peer.list_all_tools().await
            .map_err(|e| anyhow!("Failed to list tools via MCP: {}", e))
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        let peer = self.get_peer()?;
        let arguments: Option<Map<String, Value>> = match args {
            Value::Object(map) => Some(map),
            Value::Null => None,
            _ => return Err(anyhow!("Tool arguments must be a JSON object or null"))
        };
        // Ensure name is owned (e.g., Cow::Owned) for the request
        let params = CallToolRequestParam { name: Cow::Owned(name.to_string()), arguments };
        let result = peer.call_tool(params).await
            .map_err(|e| anyhow!("Failed to call tool '{}' via MCP: {}", name, e))?;
        serde_json::to_value(result.content)
            .map_err(|e| anyhow!("Failed to serialize tool result content: {}", e))
    }

    pub async fn get_resource(&self, uri: &str) -> Result<Value> {
        let peer = self.get_peer()?;
        let params = ReadResourceRequestParam { uri: uri.to_string() };
        let result: ReadResourceResult = peer.read_resource(params).await
            .map_err(|e| anyhow!("Failed to get resource '{}' via MCP: {}", uri, e))?;
        serde_json::to_value(result.contents)
            .map_err(|e| anyhow!("Failed to serialize resource contents: {}", e))
    }
}

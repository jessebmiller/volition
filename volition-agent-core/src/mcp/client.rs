// volition-agent-core/src/mcp/client.rs
use anyhow::{anyhow, Result};
use rmcp::{
    model::*,
    service::{Peer, RoleClient}, 
    transport::TokioChildProcess,
    Error as McpError,
};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub struct McpConnection {
    server_command: String,
    server_args: Vec<String>,
    peer: Arc<Mutex<Option<Peer<RoleClient>>>>, 
}

impl McpConnection {
    pub fn new(server_command: String, server_args: Vec<String>) -> Self {
        Self {
            server_command,
            server_args,
            peer: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn establish_connection_external(
        &self, 
        service: impl rmcp::service::Service<RoleClient> + 'static, 
        ct: CancellationToken
    ) -> Result<()> { 
        let mut peer_guard = self.peer.lock().await;
        if peer_guard.is_some() {
            return Ok(());
        }
        info!(command = %self.server_command, args = ?self.server_args, "Establishing MCP connection...");
        let mut cmd = Command::new(&self.server_command);
        cmd.args(&self.server_args);
        let transport = TokioChildProcess::new(&mut cmd)
            .map_err(|e| anyhow!("Failed to create MCP server process: {}", e))?;
        let running_service = rmcp::service::serve_client_with_ct(service, transport, ct)
            .await
            .map_err(|e| anyhow!("Failed to establish MCP connection: {}", e))?;
        *peer_guard = Some(running_service.peer().clone());
        info!("MCP connection established (Peer stored).");
        Ok(())
    }

    async fn get_peer_guard(&self) -> Result<tokio::sync::MutexGuard<'_, Option<Peer<RoleClient>>>> {
        let guard = self.peer.lock().await;
        if guard.is_none() {
            Err(anyhow!("MCP connection not established"))
        } else {
            Ok(guard)
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let guard = self.get_peer_guard().await?;
        let peer = guard.as_ref().unwrap();
        peer.list_all_tools().await
            .map_err(|e| anyhow!("Failed to list tools via MCP: {}", e))
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        let guard = self.get_peer_guard().await?;
        let peer = guard.as_ref().unwrap();
        let arguments: Option<Map<String, Value>> = match args {
            Value::Object(map) => Some(map),
            Value::Null => None,
            _ => return Err(anyhow!("Tool arguments must be a JSON object or null"))
        };
        let params = CallToolRequestParam { name: Cow::Owned(name.to_string()), arguments };
        let result = peer.call_tool(params).await
            .map_err(|e| anyhow!("Failed to call tool '{}' via MCP: {}", name, e))?;
        serde_json::to_value(result.content)
            .map_err(|e| anyhow!("Failed to serialize tool result content: {}", e))
    }

    pub async fn get_resource(&self, uri: &str) -> Result<Value> {
        let guard = self.get_peer_guard().await?;
        let peer = guard.as_ref().unwrap();
        let params = ReadResourceRequestParam { uri: uri.to_string() };
        let result: ReadResourceResult = peer.read_resource(params).await
            .map_err(|e| anyhow!("Failed to get resource '{}': {}", uri, e))?;
            
        // Extract text content via pattern matching on the correct variant
        let text_content = result.contents.into_iter().find_map(|item| {
             match item {
                 ResourceContents::TextResourceContents { text, .. } => Some(text),
                 _ => None, // Ignore BlobResourceContents for now
             }
         }).unwrap_or_default();
         
        // Return just the text content as a JSON string value
        Ok(Value::String(text_content))
    }
}

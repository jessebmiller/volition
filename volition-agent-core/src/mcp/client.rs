// volition-agent-core/src/mcp/client.rs
use anyhow::{anyhow, Result};
use rmcp::{
    model::*,
    service::{Peer, RoleClient}, 
    transport::TokioChildProcess,
    // Removed unused Error import
};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::sync::Arc;
use std::fs::File;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace}; // Added error, removed warn

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
            trace!("MCP connection already established.");
            return Ok(());
        }
        
        info!(command = %self.server_command, args = ?self.server_args, "Establishing MCP connection...");
        
        trace!("Creating command for MCP server...");
        let mut cmd = Command::new(&self.server_command);
        cmd.args(&self.server_args);
        // Ensure stdio is piped for MCP communication
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        // Redirect stderr to a file
        match File::create("/tmp/volition-shell-server.stderr.log") {
            Ok(stderr_file) => {
                cmd.stderr(stderr_file);
            }
            Err(e) => {
                error!(error = %e, path = "/tmp/volition-shell-server.stderr.log", "Failed to open stderr log file, using pipe instead");
                // Fallback to piped if file creation fails
                cmd.stderr(std::process::Stdio::piped());
            }
        }
        
        debug!(command = ?cmd, "Prepared command for MCP server.");

        trace!("Attempting to spawn server process and create transport...");
        let transport = match TokioChildProcess::new(&mut cmd) {
            Ok(t) => {
                debug!("MCP server process spawned successfully.");
                t
            },
            Err(e) => {
                error!(command = ?cmd, error = %e, "Failed to create MCP server process");
                return Err(anyhow!("Failed to create MCP server process: {}", e));
            }
        };
        
        trace!("Attempting MCP handshake with serve_client_with_ct...");
        match rmcp::service::serve_client_with_ct(service, transport, ct).await {
            Ok(running_service) => {
                debug!("MCP handshake successful.");
                *peer_guard = Some(running_service.peer().clone());
                info!("MCP connection established (Peer stored).");
                Ok(())
            },
            Err(e) => {
                 error!(error = %e, "Failed to establish MCP connection during handshake");
                 Err(anyhow!("Failed to establish MCP connection: {}", e))
            }
        }
    }

    async fn get_peer_guard(&self) -> Result<tokio::sync::MutexGuard<'_, Option<Peer<RoleClient>>>> {
        let guard = self.peer.lock().await;
        if guard.is_none() {
            // This error might be triggered if establish_connection failed previously
            error!("Attempted to get MCP peer, but connection is not established.");
            Err(anyhow!("MCP connection not established"))
        } else {
            Ok(guard)
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        trace!("Attempting to list tools...");
        let guard = self.get_peer_guard().await?;
        let peer = guard.as_ref().ok_or_else(|| anyhow!("Peer unavailable after lock"))?; // Should not happen if get_peer_guard succeeds
        debug!("Calling peer.list_all_tools().");
        peer.list_all_tools().await
            .map_err(|e| {
                error!(error = %e, "peer.list_all_tools() failed");
                anyhow!("Failed to list tools via MCP: {}", e)
            })
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        trace!(tool_name = %name, "Attempting to call tool...");
        let guard = self.get_peer_guard().await?;
        let peer = guard.as_ref().ok_or_else(|| anyhow!("Peer unavailable after lock"))?; 
        let arguments: Option<Map<String, Value>> = match args {
            Value::Object(map) => Some(map),
            Value::Null => None,
            _ => {
                error!(args = ?args, "Invalid tool arguments type");
                return Err(anyhow!("Tool arguments must be a JSON object or null"))
            }
        };
        let params = CallToolRequestParam { name: Cow::Owned(name.to_string()), arguments };
        debug!(?params, "Calling peer.call_tool().");
        let result = peer.call_tool(params).await
            .map_err(|e| {
                 error!(tool_name = %name, error = %e, "peer.call_tool() failed");
                 anyhow!("Failed to call tool '{}' via MCP: {}", name, e)
            })?;
        serde_json::to_value(result.content)
            .map_err(|e| {
                error!(error = %e, "Failed to serialize tool result content");
                anyhow!("Failed to serialize tool result content: {}", e)
            })
    }

    pub async fn get_resource(&self, uri: &str) -> Result<Value> {
        trace!(%uri, "Attempting to get resource...");
        let guard = self.get_peer_guard().await?;
        let peer = guard.as_ref().ok_or_else(|| anyhow!("Peer unavailable after lock"))?;
        let params = ReadResourceRequestParam { uri: uri.to_string() };
        debug!(?params, "Calling peer.read_resource().");
        let result: ReadResourceResult = peer.read_resource(params).await
            .map_err(|e| {
                error!(%uri, error = %e, "peer.read_resource() failed");
                anyhow!("Failed to get resource '{}': {}", uri, e)
            })?;
            
        let text_content = result.contents.into_iter().find_map(|item| {
             match item {
                 ResourceContents::TextResourceContents { text, .. } => Some(text),
                 _ => None,
             }
         }).unwrap_or_default();
         
        Ok(Value::String(text_content))
    }
}

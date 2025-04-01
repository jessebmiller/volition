// volition-servers/shell/src/main.rs
// Removed unused anyhow import
use rmcp::{
    model::{*}, // Keep model::*
    service::*,
    transport::io,
    Error as McpError,
};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use duct;

fn create_schema_object(properties: Vec<(&str, Value)>, required: Vec<&str>) -> Arc<Map<String, Value>> {
    let props_map: Map<String, Value> = properties.into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    let req_vec: Vec<Value> = required.into_iter().map(|s| Value::String(s.to_string())).collect();
    let schema = json!({
        "type": "object",
        "properties": props_map,
        "required": req_vec
    });
    let map = match schema {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    Arc::new(map)
}

#[derive(Debug, Clone)]
struct ShellServer {
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    tools: Arc<HashMap<String, Tool>>,
}

impl ShellServer {
    fn new() -> Self {
        let mut tools = HashMap::new();
        let shell_schema = create_schema_object(
            vec![
                ("command", json!({ "type": "string", "description": "The shell command to execute." })),
                ("workdir", json!({ "type": "string", "description": "Optional working directory." })), 
            ],
            vec!["command"],
        );
        tools.insert(
            "shell".to_string(),
            Tool {
                name: "shell".into(),
                description: Some("Executes a shell command.".into()),
                input_schema: shell_schema,
            },
        );
        Self {
            peer: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
        }
    }

    fn handle_shell_call(&self, params: CallToolRequestParam) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            let args_map: Map<String, Value> = params.arguments
                .ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
            let command = args_map.get("command").and_then(Value::as_str)
                .ok_or_else(|| McpError::invalid_params("Missing 'command' argument", None))?;
            let workdir = args_map.get("workdir").and_then(Value::as_str);
            let cmd_expr = duct::cmd!(command);
            let cmd_expr = if let Some(dir) = workdir {
                cmd_expr.dir(dir)
            } else {
                cmd_expr
            };
            let output_result = cmd_expr.stdout_capture().stderr_capture().unchecked().run();
            let (content_vec, is_error) = match output_result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let exit_code = output.status.code().unwrap_or(-1);
                    let result_text = format!(
                        "Exit Code: {}\\n--- STDOUT ---\\n{}\\n--- STDERR ---\\n{}",
                        exit_code, stdout, stderr
                    );
                    let raw_content = RawContent::Text(RawTextContent { text: result_text });
                    let annotated = Annotated { raw: raw_content, annotations: None };
                    (vec![annotated], !output.status.success())
                }
                Err(e) => {
                    let error_text = format!("Failed to execute command '{}': {}", command, e);
                    let raw_content = RawContent::Text(RawTextContent { text: error_text });
                    let annotated = Annotated { raw: raw_content, annotations: None };
                    (vec![annotated], true)
                }
            };
            Ok(CallToolResult { content: content_vec, is_error: Some(is_error) })
        })
    }
}

impl Service<RoleServer> for ShellServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: Some(true) }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "volition-shell-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: None,
        }
    }
    fn get_peer(&self) -> Option<Peer<RoleServer>> {
        self.peer.lock().unwrap().clone()
    }
    fn set_peer(&mut self, peer: Peer<RoleServer>) {
        *self.peer.lock().unwrap() = Some(peer);
    }
    
    #[allow(refining_impl_trait)]
    fn handle_request(
        &self,
        request: ClientRequest,
        _context: RequestContext<RoleServer>,
    ) -> Pin<Box<dyn Future<Output = Result<ServerResult, McpError>> + Send + '_>> {
        let self_clone = self.clone();
        Box::pin(async move {
            match request {
                ClientRequest::ListToolsRequest(Request { .. }) => {
                    Ok(ServerResult::ListToolsResult(ListToolsResult {
                        tools: self_clone.tools.values().cloned().collect(),
                        next_cursor: None,
                    }))
                }
                ClientRequest::CallToolRequest(Request { params, .. }) => {
                    if params.name == "shell" {
                        self_clone.handle_shell_call(params).await.map(ServerResult::CallToolResult)
                    } else {
                        Err(McpError::method_not_found::<CallToolRequestMethod>())
                    }
                }
                _ => Err(McpError::method_not_found::<InitializeResultMethod>()),
            }
        })
    }
    
    #[allow(refining_impl_trait)]
    fn handle_notification(
        &self,
        _notification: ClientNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), McpError>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> { // Return Box<dyn Error>
    let server = ShellServer::new();
    let transport = io::stdio();
    let ct = CancellationToken::new();
    
    eprintln!("Starting shell MCP server...");
    
    if let Err(e) = server.serve_with_ct(transport, ct.clone()).await {
         eprintln!("Server loop failed: {}", e); 
    }
        
    ct.cancelled().await;
    
    eprintln!("Shell MCP server stopped.");
    
    Ok(())
}

// volition-servers/filesystem/src/main.rs
// Removed unused anyhow import
use rmcp::{
    Error as McpError,
    model::*, // Import model::*
    service::*,
    transport::io, // Import transport::io module for stdio()
};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio_util::sync::CancellationToken;

// Helper to create JSON schema object
fn create_schema_object(
    properties: Vec<(&str, Value)>,
    required: Vec<&str>,
) -> Arc<Map<String, Value>> {
    let props_map: Map<String, Value> = properties
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    let req_vec: Vec<Value> = required
        .into_iter()
        .map(|s| Value::String(s.to_string()))
        .collect();

    let schema = json!({
        "type": "object",
        "properties": props_map,
        "required": req_vec
    });
    // Ensure it's a map
    let map = match schema {
        Value::Object(map) => map,
        _ => Map::new(), // Should not happen
    };
    Arc::new(map)
}

// Define the server struct
#[derive(Debug, Clone)]
struct FileSystemServer {
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    tools: Arc<HashMap<String, Tool>>,
}

impl FileSystemServer {
    fn new() -> Self {
        let mut tools = HashMap::new();

        // read_file schema
        let read_file_schema = create_schema_object(
            vec![(
                "path",
                json!({ "type": "string", "description": "Path to the file to read." }),
            )],
            vec!["path"],
        );
        tools.insert(
            "read_file".to_string(),
            Tool {
                name: "read_file".into(),
                description: Some("Reads the content of a file at the given path.".into()),
                input_schema: read_file_schema,
            },
        );

        // write_file schema
        let write_file_schema = create_schema_object(
            vec![
                (
                    "path",
                    json!({ "type": "string", "description": "Path to the file to write." }),
                ),
                (
                    "content",
                    json!({ "type": "string", "description": "Content to write to the file." }),
                ),
            ],
            vec!["path", "content"],
        );
        tools.insert(
            "write_file".to_string(),
            Tool {
                name: "write_file".into(),
                description: Some(
                    "Writes the given content to a file at the specified path.".into(),
                ),
                input_schema: write_file_schema,
            },
        );

        Self {
            peer: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
        }
    }

    fn handle_tool_call(
        &self,
        params: CallToolRequestParam,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            match params.name.as_ref() {
                "read_file" => {
                    let args_map: Map<String, Value> = params
                        .arguments
                        .ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                    let path = args_map
                        .get("path")
                        .and_then(Value::as_str)
                        .ok_or_else(|| McpError::invalid_params("Missing 'path' argument", None))?;
                    let content_string = fs::read_to_string(path).await.map_err(|e| {
                        McpError::internal_error(format!("Failed to read file: {}", e), None)
                    })?;
                    let raw_content = RawContent::Text(RawTextContent {
                        text: content_string,
                    });
                    let annotated_content = Annotated {
                        raw: raw_content,
                        annotations: None,
                    };
                    Ok(CallToolResult {
                        content: vec![annotated_content],
                        is_error: Some(false),
                    })
                }
                "write_file" => {
                    let args_map: Map<String, Value> = params
                        .arguments
                        .ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
                    let path = args_map
                        .get("path")
                        .and_then(Value::as_str)
                        .ok_or_else(|| McpError::invalid_params("Missing 'path' argument", None))?;
                    let content_string = args_map
                        .get("content")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            McpError::invalid_params("Missing 'content' argument", None)
                        })?;
                    fs::write(path, content_string).await.map_err(|e| {
                        McpError::internal_error(format!("Failed to write file: {}", e), None)
                    })?;
                    Ok(CallToolResult {
                        content: vec![],
                        is_error: Some(false),
                    })
                }
                _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
            }
        })
    }

    fn handle_read_resource(
        &self,
        params: ReadResourceRequestParam,
    ) -> Pin<Box<dyn Future<Output = Result<ReadResourceResult, McpError>> + Send + '_>> {
        let path = params.uri;
        Box::pin(async move {
            let content_string = fs::read_to_string(&path).await.map_err(|e| {
                McpError::internal_error(format!("Failed to read resource (file): {}", e), None)
            })?;
            let contents_item = ResourceContents::text(content_string, path);
            Ok(ReadResourceResult {
                contents: vec![contents_item],
            })
        })
    }
}

impl Service<RoleServer> for FileSystemServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(true),
                    list_changed: Some(true),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "volition-filesystem-server".into(),
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

    #[allow(refining_impl_trait)] // Allow Pin<Box<dyn Future>> where trait uses impl Future
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
                ClientRequest::CallToolRequest(Request { params, .. }) => self_clone
                    .handle_tool_call(params)
                    .await
                    .map(ServerResult::CallToolResult),
                ClientRequest::ReadResourceRequest(Request { params, .. }) => self_clone
                    .handle_read_resource(params)
                    .await
                    .map(ServerResult::ReadResourceResult),
                _ => Err(McpError::method_not_found::<InitializeResultMethod>()),
            }
        })
    }

    #[allow(refining_impl_trait)] // Allow Pin<Box<dyn Future>> where trait uses impl Future
    fn handle_notification(
        &self,
        _notification: ClientNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), McpError>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Return Box<dyn Error>
    let server = FileSystemServer::new();
    let transport = io::stdio();
    let ct = CancellationToken::new();

    eprintln!("Starting filesystem MCP server...");

    if let Err(e) = server.serve_with_ct(transport, ct.clone()).await {
        eprintln!("Server loop failed: {}", e);
    }

    ct.cancelled().await;

    eprintln!("Filesystem MCP server stopped.");

    Ok(())
}

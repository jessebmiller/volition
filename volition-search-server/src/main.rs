// volition-servers/search/src/main.rs
use anyhow::{anyhow, Result};
use rmcp::{
    model::*,
    service::*,
    transport::io,
    Error as McpError,
};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs::File;
use std::future::Future;
use std::io::{BufRead, BufReader};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

use ignore::WalkBuilder;

// Helper to create JSON schema object
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

// Define the server struct
#[derive(Debug, Clone)]
struct SearchServer {
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    tools: Arc<HashMap<String, Tool>>,
}

impl SearchServer {
    fn new() -> Self {
        let mut tools = HashMap::new();
        let search_schema = create_schema_object(
            vec![
                ("pattern", json!({ "type": "string", "description": "Text or regex pattern to search for." })),
                ("path", json!({ "type": "string", "description": "Optional directory or file path to search in (defaults to current directory)." })),
                ("case_sensitive", json!({ "type": "boolean", "description": "Perform case-sensitive search (defaults to false)." })),
                // TODO: context_lines, file_glob, max_results
            ],
            vec!["pattern"],
        );
        tools.insert(
            "search_text".to_string(),
            Tool {
                name: "search_text".into(),
                description: Some("Search for text patterns in files, respecting .gitignore.".into()),
                input_schema: search_schema,
            },
        );

        Self {
            peer: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
        }
    }

    fn handle_search_call(&self, params: CallToolRequestParam) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            let args_map: Map<String, Value> = params.arguments
                .ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;

            let pattern = args_map.get("pattern").and_then(Value::as_str)
                .ok_or_else(|| McpError::invalid_params("Missing 'pattern' argument", None))?;
            let path = args_map.get("path").and_then(Value::as_str).unwrap_or(".");
            let case_sensitive = args_map.get("case_sensitive").and_then(Value::as_bool).unwrap_or(false);

            let mut results = Vec::new();
            let walker = WalkBuilder::new(path).build();

            for result in walker {
                match result {
                    Ok(entry) => {
                        if entry.file_type().map_or(false, |ft| ft.is_file()) {
                            let file_path = entry.path();
                            // Use blocking read for simplicity, consider spawn_blocking for large files/searches
                            if let Ok(file) = File::open(file_path) {
                                let reader = BufReader::new(file);
                                for (line_num, line_result) in reader.lines().enumerate() {
                                    if let Ok(line) = line_result {
                                        let matches = if case_sensitive {
                                            line.contains(pattern)
                                        } else {
                                            line.to_lowercase().contains(&pattern.to_lowercase())
                                        };
                                        if matches {
                                            results.push(format!(
                                                " {}:{}:{}",
                                                file_path.display(),
                                                line_num + 1,
                                                line
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => results.push(format!("ERROR walking directory: {}", err)),
                }
            }

            let result_text = if results.is_empty() {
                "No matches found.".to_string()
            } else {
                results.join("\n")
            };

            let raw_content = RawContent::Text(RawTextContent { text: result_text });
            let annotated = Annotated { raw: raw_content, annotations: None };
            Ok(CallToolResult { content: vec![annotated], is_error: Some(false) })
        })
    }
}

impl Service<RoleServer> for SearchServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: Some(true) }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "volition-search-server".into(),
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
                    if params.name == "search_text" {
                         self_clone.handle_search_call(params).await.map(ServerResult::CallToolResult)
                    } else {
                         Err(McpError::method_not_found::<CallToolRequestMethod>())
                    }
                }
                _ => Err(McpError::method_not_found::<InitializeResultMethod>()),
            }
        })
    }

    fn handle_notification(
        &self,
        _notification: ClientNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), McpError>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = SearchServer::new();
    let transport = io::stdio();
    let ct = CancellationToken::new();

    println!("Starting search MCP server...");

    server.serve_with_ct(transport, ct.clone()).await
        .map_err(|e| anyhow!("Server failed: {}", e))?;

    println!("Search MCP server stopped.");

    Ok(())
}

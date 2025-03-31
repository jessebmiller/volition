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
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

// Use ignore crate for searching
use ignore::WalkBuilder;
use std::io::BufRead; // For reading file lines
use std::fs::File;
use std::io::BufReader;

// Define the server struct
#[derive(Debug, Clone)]
struct SearchServer {
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    tools: Arc<HashMap<String, Tool>>,
}

impl SearchServer {
    fn new() -> Self {
        let mut tools = HashMap::new();
        tools.insert(
            "search_text".to_string(),
            Tool {
                name: "search_text".into(),
                description: Some("Search for text patterns in files, respecting .gitignore.".into()),
                // TODO: Define schema for pattern, path, case_sensitive, context_lines, file_glob, max_results
                input_schema: Arc::new(Map::new()),
            },
        );

        Self {
            peer: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
        }
    }

    // Helper to handle search call
    fn handle_search_call(&self, params: CallToolRequestParam) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            let args_map: Map<String, Value> = params.arguments
                .ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;

            let pattern = args_map.get("pattern").and_then(Value::as_str)
                .ok_or_else(|| McpError::invalid_params("Missing 'pattern' argument", None))?;
            let path = args_map.get("path").and_then(Value::as_str).unwrap_or(".");
            let case_sensitive = args_map.get("case_sensitive").and_then(Value::as_bool).unwrap_or(false);
            // TODO: Implement context_lines, file_glob, max_results

            let mut results = Vec::new();
            let walker = WalkBuilder::new(path).build(); // Respects .gitignore by default

            for result in walker {
                match result {
                    Ok(entry) => {
                        if entry.file_type().map_or(false, |ft| ft.is_file()) {
                            let file_path = entry.path();
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
                                            // Format: path:line_num:line_content
                                            results.push(format!(
                                                " {}:{}:{}",
                                                file_path.display(),
                                                line_num + 1,
                                                line
                                            ));
                                            // TODO: Handle max_results
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
            // TODO: Check if search errors should set is_error = true
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

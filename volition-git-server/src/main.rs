// volition-servers/git/src/main.rs
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
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use git2::{Repository, StatusOptions};

// Define the server struct
#[derive(Debug, Clone)]
struct GitServer {
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    tools: Arc<HashMap<String, Tool>>,
}

impl GitServer {
    fn new() -> Self {
        let mut tools = HashMap::new();
        tools.insert(
            "git_diff".to_string(),
            Tool {
                name: "git_diff".into(),
                description: Some("Shows git diff for the repository.".into()),
                input_schema: Arc::new(Map::new()), // TODO: Schema for path, staged, etc.
            },
        );
        tools.insert(
            "git_status".to_string(),
            Tool {
                name: "git_status".into(),
                description: Some("Shows git status for the repository.".into()),
                input_schema: Arc::new(Map::new()), // TODO: Schema for path?
            },
        );
        // TODO: Add git_add, git_commit?

        Self {
            peer: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
        }
    }

    // Helper to open repo at optional path or current dir
    fn open_repo(&self, args_map: &Map<String, Value>) -> Result<Repository, McpError> {
        let path_str = args_map.get("path").and_then(Value::as_str);
        let repo_path = path_str.map(Path::new).unwrap_or_else(|| Path::new("."));
        Repository::open(repo_path)
            .map_err(|e| McpError::internal_error(format!("Failed to open repository at '{}': {}", repo_path.display(), e), None))
    }

    // Helper to handle git diff
    async fn handle_git_diff(&self, args_map: Map<String, Value>) -> Result<CallToolResult, McpError> {
        let repo = self.open_repo(&args_map)?;
        let mut diff_opts = git2::DiffOptions::new();
        // TODO: Handle staged, specific files from args_map

        // Diff HEAD against working directory
        let diff = repo.diff_index_to_workdir(None, Some(&mut diff_opts))
            .map_err(|e| McpError::internal_error(format!("Failed to generate diff: {}", e), None))?;

        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' | '-' | ' ' => line.origin().to_string(),
                _ => " ".to_string(), // Default prefix for context lines, etc.
            };
            diff_text.push_str(&prefix);
            diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or("<invalid utf8>"));
            true // Continue processing lines
        }).map_err(|e| McpError::internal_error(format!("Failed to format diff: {}", e), None))?;

        let raw_content = RawContent::Text(RawTextContent { text: diff_text });
        let annotated = Annotated { raw: raw_content, annotations: None };
        Ok(CallToolResult { content: vec![annotated], is_error: Some(false) })
    }

    // Helper to handle git status
    async fn handle_git_status(&self, args_map: Map<String, Value>) -> Result<CallToolResult, McpError> {
        let repo = self.open_repo(&args_map)?;
        let mut status_opts = StatusOptions::new();
        status_opts.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut status_opts))
            .map_err(|e| McpError::internal_error(format!("Failed to get status: {}", e), None))?;

        let mut status_text = String::new();
        if statuses.is_empty() {
            status_text.push_str("No changes detected.");
        } else {
            for entry in statuses.iter() {
                let path = entry.path().unwrap_or("<invalid path>");
                let status = entry.status();
                status_text.push_str(&format!("{:?}: {}\n", status, path));
            }
        }

        let raw_content = RawContent::Text(RawTextContent { text: status_text });
        let annotated = Annotated { raw: raw_content, annotations: None };
        Ok(CallToolResult { content: vec![annotated], is_error: Some(false) })
    }

    // Helper to handle tool calls
    fn handle_tool_call(&self, params: CallToolRequestParam) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        let args_map = params.arguments.unwrap_or_default(); // Use default empty map if no args
        match params.name.as_ref() {
            "git_diff" => Box::pin(self.handle_git_diff(args_map)),
            "git_status" => Box::pin(self.handle_git_status(args_map)),
            _ => Box::pin(async { Err(McpError::method_not_found::<CallToolRequestMethod>()) })
        }
    }
}

impl Service<RoleServer> for GitServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: Some(true) }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "volition-git-server".into(),
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
                    self_clone.handle_tool_call(params).await.map(ServerResult::CallToolResult)
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
    let server = GitServer::new();
    let transport = io::stdio();
    let ct = CancellationToken::new();

    println!("Starting git MCP server...");

    server.serve_with_ct(transport, ct.clone()).await
        .map_err(|e| anyhow!("Server failed: {}", e))?;

    println!("Git MCP server stopped.");

    Ok(())
}

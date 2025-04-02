// volition-servers/git/src/main.rs
use anyhow::Result;
use git2::{Repository, StatusOptions};
use rmcp::{Error as McpError, model::*, service::*, transport::io};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Command; // Added for git commit
use std::sync::{Arc, Mutex};
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
    let map = match schema {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    Arc::new(map)
}

// Define the server struct
#[derive(Debug, Clone)]
struct GitServer {
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    tools: Arc<HashMap<String, Tool>>,
}

impl GitServer {
    fn new() -> Self {
        let mut tools = HashMap::new();
        let path_schema_prop = (
            "path",
            json!({ "type": "string", "description": "Optional path to the repository (defaults to current directory)." }),
        );
        let message_schema_prop = (
            "message",
            json!({ "type": "string", "description": "Commit message." }),
        );

        // --- Git Diff ---
        let diff_schema = create_schema_object(
            vec![
                path_schema_prop.clone(),
                // TODO: Add staged (bool), paths (array[string])?
            ],
            vec![], // No required args for basic diff
        );
        tools.insert(
            "git_diff".to_string(),
            Tool {
                name: "git_diff".into(),
                description: "Shows git diff for the repository.".into(),
                input_schema: diff_schema,
            },
        );

        // --- Git Status ---
        let status_schema = create_schema_object(
            vec![path_schema_prop.clone()],
            vec![], // No required args for status
        );
        tools.insert(
            "git_status".to_string(),
            Tool {
                name: "git_status".into(),
                description: "Shows git status for the repository.".into(),
                input_schema: status_schema,
            },
        );

        // --- Git Commit ---
        let commit_schema = create_schema_object(
            vec![
                path_schema_prop.clone(), // Optional path
                message_schema_prop,      // Required message
            ],
            vec!["message"], // message is required
        );
        tools.insert(
            "git_commit".to_string(),
            Tool {
                name: "git_commit".into(),
                description: "Creates a git commit with the provided message.".into(),
                input_schema: commit_schema,
            },
        );

        Self {
            peer: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
        }
    }

    fn open_repo(&self, args_map: &Map<String, Value>) -> Result<Repository, McpError> {
        let path_str = args_map.get("path").and_then(Value::as_str);
        let repo_path = path_str.map(Path::new).unwrap_or_else(|| Path::new("."));
        Repository::open(repo_path).map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to open repository at '{}': {}",
                    repo_path.display(),
                    e
                ),
                None,
            )
        })
    }

    async fn handle_git_diff(
        &self,
        args_map: Map<String, Value>,
    ) -> Result<CallToolResult, McpError> {
        let repo = self.open_repo(&args_map)?;
        let mut diff_opts = git2::DiffOptions::new();
        let diff = repo
            .diff_index_to_workdir(None, Some(&mut diff_opts))
            .map_err(|e| {
                McpError::internal_error(format!("Failed to generate diff: {}", e), None)
            })?;
        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' | '-' | ' ' => line.origin().to_string(),
                _ => " ".to_string(),
            };
            diff_text.push_str(&prefix);
            diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or("<invalid utf8>"));
            true
        })
        .map_err(|e| McpError::internal_error(format!("Failed to format diff: {}", e), None))?;
        let raw_content = RawContent::Text(RawTextContent { text: diff_text });
        let annotated = Annotated {
            raw: raw_content,
            annotations: None,
        };
        Ok(CallToolResult {
            content: vec![annotated],
            is_error: Some(false),
        })
    }

    async fn handle_git_status(
        &self,
        args_map: Map<String, Value>,
    ) -> Result<CallToolResult, McpError> {
        let repo = self.open_repo(&args_map)?;
        let mut status_opts = StatusOptions::new();
        status_opts
            .include_untracked(true)
            .recurse_untracked_dirs(true);
        let statuses = repo
            .statuses(Some(&mut status_opts))
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
        let annotated = Annotated {
            raw: raw_content,
            annotations: None,
        };
        Ok(CallToolResult {
            content: vec![annotated],
            is_error: Some(false),
        })
    }

    // --- New handle_git_commit function ---
    async fn handle_git_commit(
        &self,
        args_map: Map<String, Value>,
    ) -> Result<CallToolResult, McpError> {
        let message = args_map
            .get("message")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::invalid_params("Missing required argument: message", None))?;

        let path_str = args_map.get("path").and_then(Value::as_str);
        let repo_path = path_str.map(Path::new); // Option<&Path>

        let mut command = Command::new("git");
        command.arg("commit").arg("-m").arg(message);

        if let Some(dir) = repo_path {
            // Check if path exists and is a directory before setting current_dir
            if dir.exists() && dir.is_dir() {
                command.current_dir(dir);
            } else if let Some(parent_dir) = dir.parent() {
                // If path is a file, maybe try the parent? Or just default to CWD?
                // For now, let's default to CWD if path isn't a valid directory.
                if parent_dir.exists() && parent_dir.is_dir() {
                    command.current_dir(parent_dir);
                    eprintln!(
                        "Warning: Provided path '{}' is not a directory. Running git commit in parent '{}'.",
                        dir.display(),
                        parent_dir.display()
                    );
                } else {
                    eprintln!(
                        "Warning: Provided path '{}' is not a valid directory or its parent does not exist. Running git commit in current working directory.",
                        dir.display()
                    );
                }
            } else {
                eprintln!(
                    "Warning: Provided path '{}' is not a valid directory. Running git commit in current working directory.",
                    dir.display()
                );
            }
        }

        let output = command.output().map_err(|e| {
            McpError::internal_error(format!("Failed to execute git commit: {}", e), None)
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1); // Use -1 if no exit code

        let result_text = format!(
            "Exit Code: {}
--- STDOUT ---
{}
--- STDERR ---
{}",
            exit_code, stdout, stderr
        );

        let raw_content = RawContent::Text(RawTextContent { text: result_text });
        let annotated = Annotated {
            raw: raw_content,
            annotations: None,
        };

        Ok(CallToolResult {
            content: vec![annotated],
            is_error: Some(!output.status.success()),
        })
    }

    fn handle_tool_call(
        &self,
        params: CallToolRequestParam,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        let args_map = params.arguments.unwrap_or_default();
        match params.name.as_ref() {
            "git_diff" => Box::pin(self.handle_git_diff(args_map)),
            "git_status" => Box::pin(self.handle_git_status(args_map)),
            "git_commit" => Box::pin(self.handle_git_commit(args_map)), // Added commit handler
            _ => Box::pin(async { Err(McpError::method_not_found::<CallToolRequestMethod>()) }),
        }
    }
}

impl Service<RoleServer> for GitServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
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
async fn main() -> Result<()> {
    let server = GitServer::new();
    let transport = io::stdio();
    let ct = CancellationToken::new();

    // Print startup message to stderr
    eprintln!("Starting git MCP server...");

    // Run the server loop. This might return if the client disconnects.
    if let Err(e) = server.serve_with_ct(transport, ct.clone()).await {
        eprintln!("Server loop failed: {}", e); // Log error to stderr
        // Decide if the error is fatal or if we should wait for cancellation anyway
        // For now, we'll proceed to wait for cancellation.
    }

    // Keep the process alive until cancellation is requested.
    // This handles cases where serve_with_ct returns because the client
    // disconnected after initialization, preventing premature exit.
    ct.cancelled().await;

    // Print stopped message to stderr
    eprintln!("Git MCP server stopped.");

    Ok(())
}

// volition-servers/git/src/main.rs
use anyhow::Result;
// Remove unused git2 imports if we only use std::process::Command
// use git2::{Repository, StatusOptions}; // Keep Repository for path validation? Maybe not needed.
use rmcp::{Error as McpError, model::*, service::*, transport::io};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Command; // Used for executing git
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

// Helper to create JSON schema object (unchanged)
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

// Define the server struct (unchanged)
#[derive(Debug, Clone)]
struct GitServer {
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    tools: Arc<HashMap<String, Tool>>,
}

impl GitServer {
    fn new() -> Self {
        let mut tools = HashMap::new();

        // --- Unified Git Tool ---
        let git_schema = create_schema_object(
            vec![
                (
                    "subcommand",
                    json!({ "type": "string", "description": "The git subcommand to execute (e.g., 'status', 'diff', 'log')." }),
                ),
                (
                    "args",
                     json!({
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional arguments for the git subcommand.",
                        "default": [] // Explicitly define default as empty array
                    }),
                ),
                 (
                    "path",
                    json!({ "type": "string", "description": "Optional path to the repository (defaults to current directory)." }),
                ),
            ],
            vec!["subcommand"], // Only subcommand is required
        );
        tools.insert(
            "git".to_string(),
            Tool {
                name: "git".into(),
                description: "Executes a git subcommand with optional arguments and path, subject to a deny list.".into(),
                input_schema: git_schema,
            },
        );

        Self {
            peer: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
        }
    }

    // --- New unified handle_git_command function ---
    async fn handle_git_command(
        &self,
        args_map: Map<String, Value>,
    ) -> Result<CallToolResult, McpError> {
        let subcommand = args_map
            .get("subcommand")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::invalid_params("Missing required argument: subcommand", None))?;

        let args_val = args_map.get("args").cloned().unwrap_or(json!([])); // Default to empty array if missing
        let args: Vec<String> = serde_json::from_value(args_val)
            .map_err(|e| McpError::invalid_params(format!("Invalid format for 'args': {}", e), None))?;

        let path_str = args_map.get("path").and_then(Value::as_str);
        let repo_path = path_str.map(Path::new); // Option<&Path>

        // --- Deny List ---
        let deny_list: Vec<&str> = vec![
            "push", "pull", "fetch", "merge", "rebase", "reset", "clean", "rm", "mv",
            "checkout", // Can discard changes or switch branches unsafely
            "branch",   // Deny base command due to destructive options like -D
            "tag",      // Deny base command due to destructive options like -d
            "filter-branch", // Highly destructive history rewriting
            "config", // Can alter repo/global settings
            "remote", // Deny adding/removing/modifying remotes
            "clone", // Interacts with remotes, potential for large downloads/overwrites
            // Consider denying aliases or other potentially problematic commands?
        ];

        // Check the *first* part of the subcommand against the deny list
        // This prevents things like `git branch -D mybranch` if `branch` is denied.
        let command_base = subcommand.split_whitespace().next().unwrap_or(subcommand);

        if deny_list.contains(&command_base.to_lowercase().as_str()) {
            return Err(McpError::invalid_request(
                format!("Execution of git subcommand '{}' is denied for security reasons.", command_base),
                None,
            ));
        }

        // --- Execute Command ---
        let mut command = Command::new("git");
        command.arg(subcommand); // Pass the full subcommand string first
        command.args(&args); // Add the separate arguments array

        if let Some(dir) = repo_path {
             // Basic check: does the path exist?
             if dir.exists() {
                 // If it's a directory, use it directly
                 if dir.is_dir() {
                     command.current_dir(dir);
                 }
                 // If it's a file, try its parent directory
                 else if let Some(parent_dir) = dir.parent() {
                     if parent_dir.is_dir() { // Check if parent is a directory
                        command.current_dir(parent_dir);
                        eprintln!(
                            "Warning: Provided path '{}' is a file. Running git command in parent directory '{}'.",
                            dir.display(),
                            parent_dir.display()
                        );
                     } else {
                         eprintln!(
                            "Warning: Parent directory of '{}' does not exist or is not a directory. Running git command in current working directory.",
                            dir.display()
                        );
                     }
                 }
                 // If it's neither file nor dir (symlink?) or parent fails, default to CWD
                 else {
                      eprintln!(
                        "Warning: Could not determine a valid directory from path '{}'. Running git command in current working directory.",
                        dir.display()
                    );
                 }
             } else {
                 eprintln!(
                    "Warning: Provided path '{}' does not exist. Running git command in current working directory.",
                    dir.display()
                );
             }
        }

        let output = command.output().map_err(|e| {
            McpError::internal_error(format!("Failed to execute git command: {}", e), None)
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1); // Use -1 if no exit code

        let result_text = format!(
            "Exit Code: {}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}\n", // Added newline at the end
            exit_code, stdout, stderr
        );

        let raw_content = RawContent::Text(RawTextContent { text: result_text });
        let annotated = Annotated {
            raw: raw_content,
            annotations: None,
        };

        Ok(CallToolResult {
            content: vec![annotated],
            is_error: Some(!output.status.success()), // Report error if exit code != 0
        })
    }


    // Updated handle_tool_call to dispatch the new unified command
    fn handle_tool_call(
        &self,
        params: CallToolRequestParam,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        let args_map = params.arguments.unwrap_or_default();
        match params.name.as_ref() {
            // --- NEW: Route "git" tool name to the unified handler ---
            "git" => Box::pin(self.handle_git_command(args_map)),
            _ => Box::pin(async { Err(McpError::method_not_found::<CallToolRequestMethod>()) }),
        }
    }
}

// --- Service implementation (mostly unchanged) ---
impl Service<RoleServer> for GitServer {
    fn get_info(&self) -> ServerInfo { // Unchanged
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

    fn get_peer(&self) -> Option<Peer<RoleServer>> { // Unchanged
        self.peer.lock().unwrap().clone()
    }

    fn set_peer(&mut self, peer: Peer<RoleServer>) { // Unchanged
        *self.peer.lock().unwrap() = Some(peer);
    }

    #[allow(refining_impl_trait)] // Allow Pin<Box<dyn Future>> where trait uses impl Future
    fn handle_request( // Unchanged logic, relies on handle_tool_call
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
    fn handle_notification( // Unchanged
        &self,
        _notification: ClientNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), McpError>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

// --- main function (unchanged) ---
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

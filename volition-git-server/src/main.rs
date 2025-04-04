// volition-servers/git/src/main.rs
use anyhow::Result;
use clap::Parser; // Added clap
use rmcp::{Error as McpError, model::*, service::*, transport::io};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

// --- Default Allow List ---
const DEFAULT_ALLOWED_COMMANDS: &[&str] = &[
    "status", "diff", "log", "show", "commit", "add", "shortlog", "describe", "ls-files",
    // Read-only branch/tag commands are likely safe
    "branch --list", "branch -vv",
    "tag --list", "tag -l",
    // Restore is useful but needs care. Allow specific safe forms?
    // "restore --staged", "restore" (only with file args?) - Needs more thought, leave out for now.
];

// --- CLI Arguments Definition ---
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Comma-separated list of allowed git subcommands (overrides default).
    #[arg(long)]
    allowed_commands: Option<String>,
}


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

// Define the server struct (add allowed_commands)
#[derive(Debug, Clone)]
struct GitServer {
    peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
    tools: Arc<HashMap<String, Tool>>,
    allowed_commands: Arc<Vec<String>>, // Added allow list
}

impl GitServer {
    // --- Updated Constructor ---
    fn new(allowed_commands: Vec<String>) -> Self {
        let mut tools = HashMap::new();

        // --- Unified Git Tool Schema (unchanged) ---
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
                        "default": []
                    }),
                ),
                 (
                    "path",
                    json!({ "type": "string", "description": "Optional path to the repository (defaults to current directory)." }),
                ),
            ],
            vec!["subcommand"],
        );
        tools.insert(
            "git".to_string(),
            Tool {
                name: "git".into(),
                description: "Executes an allowed git subcommand with optional arguments and path.".into(), // Updated description
                input_schema: git_schema,
            },
        );

        Self {
            peer: Arc::new(Mutex::new(None)),
            tools: Arc::new(tools),
            allowed_commands: Arc::new(allowed_commands), // Store the provided list
        }
    }

    // --- Updated handle_git_command function (uses allow list) ---
    async fn handle_git_command(
        &self,
        args_map: Map<String, Value>,
    ) -> Result<CallToolResult, McpError> {
        let subcommand_full = args_map
            .get("subcommand")
            .and_then(Value::as_str)
            .ok_or_else(|| McpError::invalid_params("Missing required argument: subcommand", None))?;

        let args_val = args_map.get("args").cloned().unwrap_or(json!([]));
        let args: Vec<String> = serde_json::from_value(args_val)
            .map_err(|e| McpError::invalid_params(format!("Invalid format for 'args': {}", e), None))?;

        let path_str = args_map.get("path").and_then(Value::as_str);
        let repo_path = path_str.map(Path::new);

        // --- Allow List Check ---
        // Check if the *full* subcommand string provided is in the allow list.
        // This is safer than just checking the base command, as it prevents
        // disallowed flags/options (e.g., if "branch" is allowed, but "branch -D" is not).
        // We compare case-insensitively.
        if !self.allowed_commands.iter().any(|allowed| allowed.eq_ignore_ascii_case(subcommand_full)) {
             // Let's also try checking just the base command for simpler cases like "log", "status"
             let command_base = subcommand_full.split_whitespace().next().unwrap_or(subcommand_full);
             if !self.allowed_commands.iter().any(|allowed| allowed.eq_ignore_ascii_case(command_base)) {
                 return Err(McpError::invalid_request(
                    format!("Execution of git subcommand '{}' is not allowed.", subcommand_full),
                    None,
                 ));
             }
             // If the base command *is* allowed, but the full string wasn't, issue a warning maybe?
             // For now, let's allow if the base command is present. More specific rules could be added.
             // Consider if "git commit -m msg" should require "commit -m" in allow list or just "commit".
             // Sticking with "base command must be allowed" for now.
        }


        // --- Execute Command ---
        let mut command = Command::new("git");
        command.arg(subcommand_full); // Pass the full subcommand string first
        command.args(&args); // Add the separate arguments array

        // Path handling (unchanged from previous version)
        if let Some(dir) = repo_path {
             if dir.exists() {
                 if dir.is_dir() {
                     command.current_dir(dir);
                 }
                 else if let Some(parent_dir) = dir.parent() {
                     if parent_dir.is_dir() {
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

        // Output handling (unchanged from previous version)
        let output = command.output().map_err(|e| {
            McpError::internal_error(format!("Failed to execute git command: {}", e), None)
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        let result_text = format!(
            "Exit Code: {}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}\n",
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


    // Updated handle_tool_call (unchanged logic, just calls the updated handler)
    fn handle_tool_call(
        &self,
        params: CallToolRequestParam,
    ) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        let args_map = params.arguments.unwrap_or_default();
        match params.name.as_ref() {
            "git" => Box::pin(self.handle_git_command(args_map)),
            _ => Box::pin(async { Err(McpError::method_not_found::<CallToolRequestMethod>()) }),
        }
    }
}

// --- Service implementation (unchanged) ---
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
                ClientRequest::CallToolRequest(Request { params, .. }) => self_clone
                    .handle_tool_call(params)
                    .await
                    .map(ServerResult::CallToolResult),
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

// --- Updated main function ---
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse(); // Parse CLI arguments

    // Determine the final list of allowed commands
    let final_allowed_commands: Vec<String> = cli.allowed_commands
        .map(|cmds| cmds.split(',').map(String::from).collect()) // Parse comma-separated string
        .unwrap_or_else(|| DEFAULT_ALLOWED_COMMANDS.iter().map(|&s| s.to_string()).collect()); // Use default if not provided

    eprintln!("Using allowed commands: {:?}", final_allowed_commands); // Log the list being used

    // Create server instance with the determined allow list
    let server = GitServer::new(final_allowed_commands);
    let transport = io::stdio();
    let ct = CancellationToken::new();

    eprintln!("Starting git MCP server...");

    if let Err(e) = server.serve_with_ct(transport, ct.clone()).await {
        eprintln!("Server loop failed: {}", e);
    }

    ct.cancelled().await;
    eprintln!("Git MCP server stopped.");
    Ok(())
}

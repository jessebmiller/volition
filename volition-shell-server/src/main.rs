// volition-servers/shell/src/main.rs
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
// Removed: use duct;

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

    async fn execute_shell_command(command: &str, workdir: Option<&str>) -> Result<(Vec<Annotated<RawContent>>, bool), McpError> {
        // *** FIX: Explicitly use sh -c for shell interpretation ***
        let mut cmd_expr = duct::cmd!("/bin/sh", "-c", command); // Explicitly use /bin/sh
        if let Some(dir) = workdir {
            cmd_expr = cmd_expr.dir(dir);
        }

        // Run the command
        let output_result = cmd_expr.stdout_capture().stderr_capture().unchecked().run();

        let (content_vec, is_error) = match output_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                let result_text = format!(
                    "Exit Code: {}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
                    exit_code, stdout, stderr
                );
                let raw_content = RawContent::Text(RawTextContent { text: result_text });
                let annotated = Annotated { raw: raw_content, annotations: None };
                (vec![annotated], !output.status.success())
            }
            Err(e) => {
                // This error should now only occur if the shell itself fails or the command truly doesn't exist after shell parsing
                let error_text = format!("Failed to execute command '{}': {}", command, e);
                // Log the error to stderr for tests (kept original log)
                eprintln!("Execute Error: {}", error_text);
                let raw_content = RawContent::Text(RawTextContent { text: error_text });
                let annotated = Annotated { raw: raw_content, annotations: None };
                (vec![annotated], true)
            }
        };
        Ok((content_vec, is_error))
    }


    fn handle_shell_call(&self, params: CallToolRequestParam) -> Pin<Box<dyn Future<Output = Result<CallToolResult, McpError>> + Send + '_>> {
        Box::pin(async move {
            let args_map: Map<String, Value> = params.arguments
                .ok_or_else(|| McpError::invalid_params("Missing arguments", None))?;
            let command = args_map.get("command").and_then(Value::as_str)
                .ok_or_else(|| McpError::invalid_params("Missing 'command' argument", None))?;
            let workdir = args_map.get("workdir").and_then(Value::as_str);

            // Fixed: Added <RawContent> generic to Annotated
            let (content_vec, is_error): (Vec<Annotated<RawContent>>, bool) = Self::execute_shell_command(command, workdir).await?;

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
                 // Added case for InitializeRequest
                 // Assuming InitializeRequest directly holds params based on rmcp server code analysis
                ClientRequest::InitializeRequest(_params) => { // Mark params as unused
                    // Note: params (InitializeRequestParam) contains client info/capabilities, ignored for now.
                    eprintln!("Received InitializeRequest (handled in handle_request - should not happen with current rmcp)"); // Added for debugging
                    // *** FIX: Use fully qualified trait syntax ***
                    let server_info = rmcp::Service::get_info(&self_clone);
                    Ok(ServerResult::InitializeResult(InitializeResult {
                        protocol_version: server_info.protocol_version, // Use negotiated or server's version
                        capabilities: server_info.capabilities,
                        server_info: server_info.server_info,
                        instructions: server_info.instructions,
                    }))
                }
                // Assuming ListToolsRequest *is* wrapped in Request based on original code
                ClientRequest::ListToolsRequest(Request { .. }) => {
                    Ok(ServerResult::ListToolsResult(ListToolsResult {
                        tools: self_clone.tools.values().cloned().collect(),
                        next_cursor: None,
                    }))
                }
                // Assuming CallToolRequest *is* wrapped in Request based on original code
                ClientRequest::CallToolRequest(Request { params, .. }) => {
                    if params.name == "shell" {
                        self_clone.handle_shell_call(params).await.map(ServerResult::CallToolResult)
                    } else {
                        // Return specific error for unsupported tool
                        Err(McpError::method_not_found::<CallToolRequestMethod>())
                    }
                }
                 // Fallback for any other *unhandled* request types
                 // Kept original fallback for now.
                _ => Err(McpError::method_not_found::<InitializeResultMethod>()),
            }
        })
    }

    #[allow(refining_impl_trait)]
    fn handle_notification(
        &self,
        _notification: ClientNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), McpError>> + Send + '_>> {
        Box::pin(async { Ok(()) }) // Basic handler, might need more logic for specific notifications
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> { // Return Box<dyn Error>
    let server = ShellServer::new();
    let transport = io::stdio();
    let ct = CancellationToken::new();

    // Fixed typo in log message
    eprintln!("Starting shell MCP server...");

    if let Err(e) = server.serve_with_ct(transport, ct.clone()).await {
         eprintln!("Server loop failed: {}", e);
    }

    ct.cancelled().await;

    eprintln!("Shell MCP server stopped.");

    Ok(())
}


// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    // Fixed: Added <RawContent> generic to Annotated
    fn get_text_from_result(result: Result<(Vec<Annotated<RawContent>>, bool), McpError>) -> (String, bool) {
        match result {
            Ok((content_vec, is_error)) => {
                let text = content_vec.first().map_or(String::new(), |annotated| {
                    match &annotated.raw {
                        RawContent::Text(t) => t.text.clone(),
                        _ => String::new(),
                    }
                });
                (text, is_error)
            }
            Err(e) => (format!("MCP Error: {}", e), true),
        }
    }

    #[tokio::test]
    async fn test_echo_absolute_path() -> Result<()> {
        let command = "/bin/echo hello world";
        let (output, is_error) = get_text_from_result(ShellServer::execute_shell_command(command, None).await);
        println!("Test Output ({}): {}", command, output); // Print for debugging
        assert!(!is_error, "Command '{}' should succeed", command);
        assert!(output.contains("hello world"), "Output should contain 'hello world'");
        assert!(output.contains("Exit Code: 0"), "Exit code should be 0");
        Ok(())
    }

    #[tokio::test]
    async fn test_git_version_absolute_path() -> Result<()> {
        let command = "/usr/bin/git --version";
         // Check if git exists before running the test
         if !std::path::Path::new("/usr/bin/git").exists() {
             println!("Skipping test_git_version_absolute_path: /usr/bin/git not found.");
             return Ok(());
         }
        let (output, is_error) = get_text_from_result(ShellServer::execute_shell_command(command, None).await);
        println!("Test Output ({}): {}", command, output); // Print for debugging
        assert!(!is_error, "Command '{}' should succeed", command);
        assert!(output.contains("git version"), "Output should contain 'git version'");
         assert!(output.contains("Exit Code: 0"), "Exit code should be 0");
        Ok(())
    }

    #[tokio::test]
    async fn test_git_version_path() -> Result<()> {
        let command = "git --version"; // Relies on PATH
         // Check if git seems available via PATH first
         let path_check = duct::cmd!("which", "git").stdout_capture().run(); // Keep using duct::cmd for simple check
         if path_check.is_err() || !path_check.unwrap().status.success() {
             println!("Skipping test_git_version_path: 'git' not found in PATH.");
             return Ok(());
         }

        let (output, is_error) = get_text_from_result(ShellServer::execute_shell_command(command, None).await);
        println!("Test Output ({}): {}", command, output); // Print for debugging
        assert!(!is_error, "Command '{}' should succeed if git is in PATH", command);
        assert!(output.contains("git version"), "Output should contain 'git version'");
         assert!(output.contains("Exit Code: 0"), "Exit code should be 0");
        Ok(())
    }

    #[tokio::test]
    async fn test_command_not_found() -> Result<()> {
        let command = "this_command_should_not_exist_qwertyuiop";
        let (output, is_error) = get_text_from_result(ShellServer::execute_shell_command(command, None).await);
        println!("Test Output ({}): {}", command, output); // Print for debugging
        // The command run should fail internally, but execute_shell_command should return Ok
        assert!(is_error, "Execution should result in an error status");
        // The error message might now come from the shell (e.g., "sh: ... not found")
        // assert!(output.contains("Failed to execute command"), "Output should indicate execution failure"); // This might not be true anymore
        assert!(output.contains("not found") || output.contains("No such file"), "Error message should indicate command not found by shell");
        Ok(())
    }

    // Removed test_command_parsing_failure as duct handles parsing

    #[tokio::test]
    async fn test_command_with_args() -> Result<()> {
        let command = "/bin/ls -l"; // Command with arguments
        let (output, is_error) = get_text_from_result(ShellServer::execute_shell_command(command, None).await);
        println!("Test Output ({}): {}", command, output); // Print for debugging
        assert!(!is_error, "Command '{}' should succeed", command);
        assert!(output.contains("total"), "Output should contain typical ls -l output"); // Check for a common string in ls output
        assert!(output.contains("Exit Code: 0"), "Exit code should be 0");
        Ok(())
    }

}

// volition-cli/src/tools/provider.rs

use anyhow::{anyhow, Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

// use reqwest::Client; // Removed unused import

use volition_agent_core::{async_trait, models::tools::*, ToolProvider};

// Import the CLI tool *wrapper* functions
use super::{cargo, file, filesystem, git, search, shell, user_input};

pub struct CliToolProvider {
    // No fields needed currently.
    // TODO: Consider adding back http_client if any CLI tool wrappers
    //       (e.g., for web search, hypothetical future tool) require it.
    // _http_client: Client,
}

impl CliToolProvider {
    // Updated constructor - no client needed for now
    pub fn new(/* http_client: Client */) -> Self {
        Self { /* _http_client: http_client */ }
    }

    // --- Parameter Definition Helpers ---
    fn string_param(description: &str) -> ToolParameter {
        ToolParameter {
            param_type: ToolParameterType::String,
            description: description.to_string(),
            enum_values: None,
            items: None,
        }
    }
    fn bool_param(description: &str) -> ToolParameter {
        ToolParameter {
            param_type: ToolParameterType::Boolean,
            description: description.to_string(),
            enum_values: None,
            items: None,
        }
    }
    fn int_param(description: &str) -> ToolParameter {
        ToolParameter {
            param_type: ToolParameterType::Integer,
            description: description.to_string(),
            enum_values: None,
            items: None,
        }
    }
    fn string_array_param(description: &str) -> ToolParameter {
        ToolParameter {
            param_type: ToolParameterType::Array,
            description: description.to_string(),
            enum_values: None,
            items: Some(Box::new(ToolParameter {
                param_type: ToolParameterType::String,
                description: "A single string item ".to_string(),
                enum_values: None,
                items: None,
            })),
        }
    }
}
// --- End Parameter Definition Helpers ---

#[async_trait]
impl ToolProvider for CliToolProvider {
    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "shell".to_string(),
                description: "Run a shell command and get the output ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([(
                        "command".to_string(),
                        Self::string_param("The shell command to run "),
                    )]),
                    required: vec!["command".to_string()],
                },
            },
            ToolDefinition {
                name: "read_file".to_string(),
                description: "Read the contents of a file ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([(
                        "path".to_string(),
                        Self::string_param("Path to the file to read "),
                    )]),
                    required: vec!["path".to_string()],
                },
            },
            ToolDefinition {
                name: "write_file".to_string(),
                description: "Write content to a file ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "path".to_string(),
                            Self::string_param("Path to the file to write "),
                        ),
                        (
                            "content".to_string(),
                            Self::string_param("Content to write to the file "),
                        ),
                    ]),
                    required: vec!["path".to_string(), "content".to_string()],
                },
            },
            ToolDefinition {
                name: "search_text".to_string(),
                description: "Search for text patterns in files, returning matching lines with context. Requires \'ripgrep\' (rg) to be installed. ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "pattern".to_string(),
                            Self::string_param("Text or regex pattern to search for "),
                        ),
                        (
                            "path".to_string(),
                            Self::string_param(
                                "Directory or file path to search in (defaults to current directory) ",
                            ),
                        ),
                        (
                            "file_glob".to_string(),
                            Self::string_param(
                                "Glob pattern to filter files (e.g., \'*.rs\', \'*.md\', defaults to \'*\') - Use forward slashes ('/') as path separators in globs, even on Windows. ",
                            ),
                        ),
                        (
                            "case_sensitive".to_string(),
                            Self::bool_param("Perform case-sensitive search (defaults to false) "),
                        ),
                        (
                            "context_lines".to_string(),
                            Self::int_param(
                                "Number of context lines before and after each match (defaults to 1) ",
                            ),
                        ),
                        (
                            "max_results".to_string(),
                            Self::int_param(
                                "Maximum number of matching lines to return (defaults to 50) ",
                            ),
                        ),
                    ]),
                    required: vec!["pattern".to_string()],
                },
            },
            ToolDefinition {
                name: "find_rust_definition".to_string(),
                description: "Find where a Rust symbol (function, struct, enum, trait, etc.) is defined in the codebase. Searches *.rs files. ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "symbol".to_string(),
                            Self::string_param(
                                "Rust symbol name to search for (function, struct, enum, trait, macro, etc.) ",
                            ),
                        ),
                        (
                            "path".to_string(),
                            Self::string_param(
                                "Directory path to search in (defaults to current directory) ",
                            ),
                        ),
                    ]),
                    required: vec!["symbol".to_string()],
                },
            },
            ToolDefinition {
                name: "user_input".to_string(),
                description: "Ask the user for input when a choice needs to be made ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "prompt".to_string(),
                            Self::string_param("The question or prompt to show the user "),
                        ),
                        (
                            "options".to_string(),
                            Self::string_array_param(
                                "Optional list of specific options to present to the user ",
                            ),
                        ),
                    ]),
                    required: vec!["prompt".to_string()],
                },
            },
            ToolDefinition {
                name: "git_command".to_string(),
                description: "Run a safe git command. Denied commands: push, reset, rebase, checkout, branch -D, etc. ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "command".to_string(),
                            Self::string_param(
                                "The git subcommand to run (e.g., \"status\", \"diff\", \"add\", \"commit\", \"log\") ",
                            ),
                        ),
                        (
                            "args".to_string(),
                            Self::string_array_param(
                                "Arguments for the git subcommand (e.g., [\"--porcelain\"], [\"--staged\"], [\"src/main.rs\"], [\"-m\", \"My message\"]) ",
                            ),
                        ),
                    ]),
                    required: vec!["command".to_string()],
                },
            },
            ToolDefinition {
                name: "cargo_command".to_string(),
                description: "Run a safe cargo command. Denied commands: publish, install, login, owner, etc. ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "command".to_string(),
                            Self::string_param(
                                "The cargo subcommand to run (e.g., \"build\", \"test\", \"check\", \"fmt\", \"run\") ",
                            ),
                        ),
                        (
                            "args".to_string(),
                            Self::string_array_param(
                                "Arguments for the cargo subcommand (e.g., [\"--release\"], [\"my_test\", \"--\", \"--nocapture\"]) ",
                            ),
                        ),
                    ]),
                    required: vec!["command".to_string()],
                },
            },
            ToolDefinition {
                name: "list_directory".to_string(),
                description: "List files and directories at a given path, respecting .gitignore. Output is raw text, one path per line. ".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "path".to_string(),
                            Self::string_param("The directory path to explore. "),
                        ),
                        (
                            "depth".to_string(),
                            Self::int_param(
                                "Maximum depth to recurse (1 lists immediate contents, 2 includes subdirs, etc.). Defaults to 1. Use 0 to list only the directory itself (if not hidden/ignored). ",
                            ),
                        ),
                        (
                            "show_hidden".to_string(),
                            Self::bool_param(
                                "Include hidden files/directories (starting with \'.\'). Defaults to false. ",
                            ),
                        ),
                    ]),
                    required: vec!["path".to_string()],
                },
            },
        ]
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        input: ToolInput,
        working_dir: &Path,
    ) -> Result<String> {
        tracing::info!(
            tool_name = tool_name,
            args = ?input.arguments,
            "Executing tool via CliToolProvider"
        );
        let args = input.arguments;

        // Call the appropriate CLI wrapper function which includes safety checks / interaction
        // These wrappers then call the core tool implementations
        match tool_name {
            "shell" => {
                let command: String = get_required_arg(&args, "command")?;
                shell::run_shell_command(&command, working_dir).await
            }
            "read_file" => {
                let path: String = get_required_arg(&args, "path")?;
                file::read_file(&path, working_dir).await
            }
            "write_file" => {
                let path: String = get_required_arg(&args, "path")?;
                let content: String = get_required_arg(&args, "content")?;
                file::write_file(&path, &content, working_dir).await
            }
            "search_text" => {
                let pattern: String = get_required_arg(&args, "pattern")?;
                let path: Option<String> = get_optional_arg(&args, "path")?;
                let file_glob: Option<String> = get_optional_arg(&args, "file_glob")?;
                let case_sensitive: Option<bool> = get_optional_arg(&args, "case_sensitive")?;
                let context_lines: Option<u32> = get_optional_arg(&args, "context_lines")?;
                let max_results: Option<usize> = get_optional_arg(&args, "max_results")?;
                search::run_search_text(
                    &pattern,
                    path.as_deref(),
                    file_glob.as_deref(),
                    case_sensitive,
                    context_lines,
                    max_results,
                    working_dir,
                )
                .await
            }
            "find_rust_definition" => {
                let symbol: String = get_required_arg(&args, "symbol")?;
                let path: Option<String> = get_optional_arg(&args, "path")?;
                search::run_find_rust_definition(&symbol, path.as_deref(), working_dir).await
            }
            "user_input" => {
                let prompt: String = get_required_arg(&args, "prompt")?;
                let options: Option<Vec<String>> = get_optional_arg(&args, "options")?;
                user_input::get_user_input(&prompt, options)
            }
            "cargo_command" => {
                let command: String = get_required_arg(&args, "command")?;
                let cmd_args: Option<Vec<String>> = get_optional_arg(&args, "args")?;
                cargo::run_cargo_command(&command, cmd_args.as_deref().unwrap_or(&[]), working_dir)
                    .await
            }
            "git_command" => {
                let command: String = get_required_arg(&args, "command")?;
                let cmd_args: Option<Vec<String>> = get_optional_arg(&args, "args")?;
                git::run_git_command(&command, cmd_args.as_deref().unwrap_or(&[]), working_dir)
                    .await
            }
            "list_directory" => {
                let path: String = get_required_arg(&args, "path")?;
                let depth: Option<usize> = get_optional_arg(&args, "depth")?;
                let show_hidden: Option<bool> = get_optional_arg(&args, "show_hidden")?;
                filesystem::run_list_directory_contents(
                    &path,
                    depth,
                    show_hidden.unwrap_or(false),
                    working_dir,
                )
            }
            unknown => {
                tracing::error!(tool_name = unknown, "Unknown tool requested");
                Err(anyhow!("Unknown tool requested by AI: {}", unknown))
            }
        }
    }
}

// --- Argument Extraction Helpers ---
fn get_required_arg<T>(args: &HashMap<String, JsonValue>, key: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let value = args
        .get(key)
        .ok_or_else(|| anyhow!("Missing required argument: '{}'", key))?;
    serde_json::from_value(value.clone()).with_context(|| {
        format!(
            "Invalid type or value for argument '{}'. Expected {}.",
            key,
            std::any::type_name::<T>()
        )
    })
}

fn get_optional_arg<T>(args: &HashMap<String, JsonValue>, key: &str) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    match args.get(key) {
        Some(value) => {
            if value.is_null() {
                Ok(None)
            } else {
                serde_json::from_value(value.clone())
                    .map(Some)
                    .with_context(|| {
                        format!(
                            "Invalid type or value for optional argument '{}'. Expected {}.",
                            key,
                            std::any::type_name::<T>()
                        )
                    })
            }
        }
        None => Ok(None),
    }
}
// --- End Argument Extraction Helpers ---

// volition-cli/src/tools/provider.rs
use anyhow::{anyhow, Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use volition_core::tools::fs::{list_directory_contents, read_file as read_file_core};
use volition_core::{async_trait, models::tools::*, ToolProvider};

// Remove lsp imports
use super::{cargo, file, git, search, shell, user_input};

#[derive(Debug)]
enum CliToolArguments {
    Shell {
        command: String,
    },
    ReadFile {
        path: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    SearchText {
        pattern: String,
        path: Option<String>,
        file_glob: Option<String>,
        case_sensitive: Option<bool>,
        context_lines: Option<u32>,
        max_results: Option<usize>,
    },
    FindRustDefinition {
        symbol: String,
        path: Option<String>,
    },
    UserInput {
        prompt: String,
        options: Option<Vec<String>>,
    },
    CargoCommand {
        command: String,
        args: Option<Vec<String>>,
    },
    // Keep internal enum variant name, just change parsing/definition
    GitCommand {
        command: String, // This field will hold the 'subcommand' value after parsing
        args: Option<Vec<String>>,
    },
    ListDirectory {
        path: String,
        depth: Option<usize>,
        show_hidden: Option<bool>,
    },
    // Remove LSP variants
}

impl fmt::Display for CliToolArguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliToolArguments::Shell { command } => write!(f, "command: '{}'", command),
            CliToolArguments::ReadFile { path } => write!(f, "path: {}", path),
            CliToolArguments::WriteFile { path, content } => {
                write!(f, "path: {}, content_len: {}", path, content.len())
            }
            CliToolArguments::SearchText {
                pattern,
                path,
                file_glob,
                ..
            } => {
                write!(f, "pattern: '{}'", pattern)?;
                if let Some(p) = path {
                    write!(f, ", path: {}", p)?;
                }
                if let Some(g) = file_glob {
                    write!(f, ", glob: {}", g)?;
                }
                Ok(())
            }
            CliToolArguments::FindRustDefinition { symbol, path } => {
                write!(f, "symbol: {}", symbol)?;
                if let Some(p) = path {
                    write!(f, ", path: {}", p)?;
                }
                Ok(())
            }
            CliToolArguments::UserInput { prompt, options } => {
                write!(f, "prompt: '{}'", prompt)?;
                if let Some(opts) = options {
                    write!(f, ", options: [{}]", opts.join(", "))?;
                }
                Ok(())
            }
            CliToolArguments::CargoCommand { command, args } => {
                write!(f, "command: {}", command)?;
                if let Some(a) = args {
                    write!(f, ", args: {:?}", a)?;
                }
                Ok(())
            }
            // Display format remains the same internally
            CliToolArguments::GitCommand { command, args } => {
                write!(f, "subcommand: {}", command)?;
                if let Some(a) = args {
                    write!(f, ", args: {:?}", a)?;
                }
                Ok(())
            }
            CliToolArguments::ListDirectory {
                path,
                depth,
                show_hidden,
            } => {
                write!(f, "path: {}", path)?;
                if let Some(d) = depth {
                    write!(f, ", depth: {}", d)?;
                }
                if let Some(h) = show_hidden {
                    write!(f, ", show_hidden: {}", h)?;
                }
                Ok(())
            }
            // Remove LSP display arms
        }
    }
}

fn parse_tool_arguments(
    tool_name: &str,
    args: &HashMap<String, JsonValue>,
) -> Result<CliToolArguments> {
    // Helper functions remain the same
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

    match tool_name {
        "shell" => Ok(CliToolArguments::Shell {
            command: get_required_arg(args, "command")?,
        }),
        "read_file" => Ok(CliToolArguments::ReadFile {
            path: get_required_arg(args, "path")?,
        }),
        "write_file" => Ok(CliToolArguments::WriteFile {
            path: get_required_arg(args, "path")?,
            content: get_required_arg(args, "content")?,
        }),
        "search_text" => Ok(CliToolArguments::SearchText {
            pattern: get_required_arg(args, "pattern")?,
            path: get_optional_arg(args, "path")?,
            file_glob: get_optional_arg(args, "file_glob")?,
            case_sensitive: get_optional_arg(args, "case_sensitive")?,
            context_lines: get_optional_arg(args, "context_lines")?,
            max_results: get_optional_arg(args, "max_results")?,
        }),
        "find_rust_definition" => Ok(CliToolArguments::FindRustDefinition {
            symbol: get_required_arg(args, "symbol")?,
            path: get_optional_arg(args, "path")?,
        }),
        "user_input" => Ok(CliToolArguments::UserInput {
            prompt: get_required_arg(args, "prompt")?,
            options: get_optional_arg(args, "options")?,
        }),
        "cargo_command" => Ok(CliToolArguments::CargoCommand {
            command: get_required_arg(args, "command")?,
            args: get_optional_arg(args, "args")?,
        }),
        // Changed tool name from "git_command" to "git"
        "git" => Ok(CliToolArguments::GitCommand {
            // Changed argument name from "command" to "subcommand"
            command: get_required_arg(args, "subcommand")?,
            args: get_optional_arg(args, "args")?,
        }),
        "list_directory" => Ok(CliToolArguments::ListDirectory {
            path: get_required_arg(args, "path")?,
            depth: get_optional_arg(args, "depth")?,
            show_hidden: get_optional_arg(args, "show_hidden")?,
        }),
        // Remove LSP parsing arms
        unknown => Err(anyhow!("Unknown tool name: {}", unknown)),
    }
}

pub struct CliToolProvider {}

impl CliToolProvider {
    pub fn new() -> Self {
        Self {}
    }

    // --- Parameter definition helpers (remove unused ones if desired) ---
    fn string_param(description: &str) -> ToolParameter {
        ToolParameter {
            param_type: ToolParameterType::String,
            description: description.to_string(),
            enum_values: None,
            items: None,
            properties: None, // Added properties/required fields here based on core model
            required: None,
        }
    }
    fn bool_param(description: &str) -> ToolParameter {
        ToolParameter {
            param_type: ToolParameterType::Boolean,
            description: description.to_string(),
            enum_values: None,
            items: None,
            properties: None,
            required: None,
        }
    }
    fn int_param(description: &str) -> ToolParameter {
        ToolParameter {
            param_type: ToolParameterType::Integer,
            description: description.to_string(),
            enum_values: None,
            items: None,
            properties: None,
            required: None,
        }
    }
    fn string_array_param(description: &str) -> ToolParameter {
        ToolParameter {
            param_type: ToolParameterType::Array,
            description: description.to_string(),
            enum_values: None,
            items: Some(Box::new(ToolParameter {
                param_type: ToolParameterType::String,
                description: "A single string item".to_string(),
                enum_values: None,
                items: None,
                properties: None,
                required: None,
            })),
            properties: None,
            required: None,
        }
    }
    // Remove object_param and array_param helpers if no longer used
    // fn object_param(...) -> ToolParameter { ... }
    // fn array_param(...) -> ToolParameter { ... }
}

#[async_trait]
impl ToolProvider for CliToolProvider {
    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        // Remove LSP parameter definitions (lsp_position_param, etc.)

        vec![ // Changed from `let mut definitions = vec![...]; definitions.extend(...)`
            // --- Existing Tool Definitions ---
            ToolDefinition {
                name: "shell".to_string(),
                description: "Run a shell command and get the output".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([("command".to_string(), Self::string_param("The shell command to run"))]),
                    required: vec!["command".to_string()],
                },
            },
            ToolDefinition {
                name: "read_file".to_string(),
                description: "Read the contents of a file".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([("path".to_string(), Self::string_param("Path to the file to read"))]),
                    required: vec!["path".to_string()],
                },
            },
            ToolDefinition {
                name: "write_file".to_string(),
                description: "Write content to a file".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        ("path".to_string(), Self::string_param("Path to the file to write")),
                        ("content".to_string(), Self::string_param("Content to write to the file")),
                    ]),
                    required: vec!["path".to_string(), "content".to_string()],
                },
            },
            ToolDefinition {
                name: "search_text".to_string(),
                description: "Search for text patterns in files, returning matching lines with context. Requires 'ripgrep' (rg) to be installed.".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        ("pattern".to_string(), Self::string_param("Text or regex pattern to search for")),
                        ("path".to_string(), Self::string_param("Directory or file path to search in (defaults to current directory)")),
                        ("file_glob".to_string(), Self::string_param("Glob pattern to filter files (e.g., \"*.rs\", \"*.md\", defaults to \"*\") - Use forward slashes ('/') as path separators in globs, even on Windows.")),
                        ("case_sensitive".to_string(), Self::bool_param("Perform case-sensitive search (defaults to false)")),
                        ("context_lines".to_string(), Self::int_param("Number of context lines before and after each match (defaults to 1)")),
                        ("max_results".to_string(), Self::int_param("Maximum number of matching lines to return (defaults to 50)")),
                    ]),
                    required: vec!["pattern".to_string()],
                },
            },
            ToolDefinition {
                name: "find_rust_definition".to_string(),
                description: "Find where a Rust symbol (function, struct, enum, trait, etc.) is defined in the codebase. Searches *.rs files.".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        ("symbol".to_string(), Self::string_param("Rust symbol name to search for (function, struct, enum, trait, macro, etc.)")),
                        ("path".to_string(), Self::string_param("Directory path to search in (defaults to current directory)")),
                    ]),
                    required: vec!["symbol".to_string()],
                },
            },
            ToolDefinition {
                name: "user_input".to_string(),
                description: "Ask the user for input when a choice needs to be made".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        ("prompt".to_string(), Self::string_param("The question or prompt to show the user")),
                        ("options".to_string(), Self::string_array_param("Optional list of specific options to present to the user")),
                    ]),
                    required: vec!["prompt".to_string()],
                },
            },
            ToolDefinition {
                name: "git".to_string(),
                description: "Executes an allowed git subcommand with optional arguments and path. Denied commands: push, reset, rebase, checkout, branch -D, etc.".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "subcommand".to_string(),
                             Self::string_param("The git subcommand to execute (e.g., 'status', 'diff', 'log')"),
                        ),
                        (
                            "args".to_string(),
                             Self::string_array_param("Optional arguments for the git subcommand (e.g., [\"--porcelain\"], [\"--staged\"], [\"src/main.rs\"], [\"-m\", \"My message\"])"),
                        ),
                    ]),
                    required: vec!["subcommand".to_string()],
                },
            },
            ToolDefinition {
                name: "cargo_command".to_string(),
                description: "Run a safe cargo command. Denied commands: publish, install, login, owner, etc.".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "command".to_string(),
                             Self::string_param("The cargo subcommand to run (e.g., \"build\", \"test\", \"check\", \"fmt\", \"run\")"),
                        ),
                        (
                            "args".to_string(),
                             Self::string_array_param("Arguments for the cargo subcommand (e.g., [\"--release\"], [\"my_test\", \"--\", \"--nocapture\"])"),
                        ),
                    ]),
                    required: vec!["command".to_string()],
                },
            },
            ToolDefinition {
                name: "list_directory".to_string(),
                description: "List files and directories at a given path, respecting .gitignore. Output is raw text, one path per line.".to_string(),
                parameters: ToolParametersDefinition {
                    param_type: "object".to_string(),
                    properties: HashMap::from([
                        ("path".to_string(), Self::string_param("The directory path to explore.")),
                        ("depth".to_string(), Self::int_param("Maximum depth to recurse (1 lists immediate contents, 2 includes subdirs, etc.). Defaults to 1. Use 0 to list only the directory itself (if not hidden/ignored).")),
                        ("show_hidden".to_string(), Self::bool_param("Include hidden files/directories (starting with '.'). Defaults to false.")),
                    ]),
                    required: vec!["path".to_string()],
                },
            },
            // Remove LSP tool definitions
        ]
        // Remove definitions.extend(...)
    }


    async fn execute_tool(
        &self,
        tool_name: &str,
        input: ToolInput,
        working_dir: &Path,
    ) -> Result<String> {
        let parsed_args = parse_tool_arguments(tool_name, &input.arguments)
            .with_context(|| format!("Failed to parse arguments for tool '{}'", tool_name))?;

        tracing::info!(
            tool_name = tool_name,
            args = %parsed_args, // Use Display format (%)
            "Executing tool via CliToolProvider"
        );

        match parsed_args {
            CliToolArguments::Shell { command } => {
                shell::run_shell_command(&command, working_dir).await
            }
            CliToolArguments::ReadFile { path } => read_file_core(&path, working_dir).await,
            CliToolArguments::WriteFile { path, content } => {
                file::write_file(&path, &content, working_dir).await
            }
            CliToolArguments::SearchText {
                pattern,
                path,
                file_glob,
                case_sensitive,
                context_lines,
                max_results,
            } => {
                search::search_text(
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
            CliToolArguments::FindRustDefinition { symbol, path } => {
                search::find_rust_definition(&symbol, path.as_deref(), working_dir).await
            }
            CliToolArguments::UserInput { prompt, options } => {
                user_input::get_user_input(&prompt, options)
            }
            CliToolArguments::CargoCommand { command, args } => {
                cargo::run_cargo_command(&command, args.as_deref().unwrap_or(&[]), working_dir)
                    .await
            }
            CliToolArguments::GitCommand { command, args } => {
                git::run_git_command(&command, args.as_deref().unwrap_or(&[]), working_dir).await
            }
            CliToolArguments::ListDirectory {
                path,
                depth,
                show_hidden,
            } => list_directory_contents(&path, depth, show_hidden.unwrap_or(false), working_dir),
            // Remove LSP execution arms
        }
    }
}

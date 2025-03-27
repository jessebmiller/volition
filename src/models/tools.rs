// src/models/tools.rs
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolFunction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShellArgs {
    pub command: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadFileArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WriteFileArgs {
    pub path: String,
    pub content: String,
}

// Renamed from SearchCodeArgs to SearchTextArgs and updated fields
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchTextArgs {
    pub pattern: String,
    pub path: Option<String>,
    pub file_glob: Option<String>, // Changed from file_pattern to file_glob
    pub case_sensitive: Option<bool>,
    pub context_lines: Option<u32>, // Added context_lines
    pub max_results: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FindDefinitionArgs {
    pub symbol: String,
    pub language: Option<String>,
    pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInputArgs {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

// --- Unified Cargo Tool Struct ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CargoCommandArgs {
    // The cargo subcommand (e.g., "build", "test", "check", "fmt")
    pub command: String,
    // Arguments for the subcommand (e.g., ["--release"], ["--", "--nocapture"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

// --- Unified Git Tool Struct ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitCommandArgs {
    // The git subcommand (e.g., "status", "diff", "add", "commit")
    pub command: String,
    // Arguments for the subcommand (e.g., ["--porcelain"], ["--staged"], ["src/main.rs"], ["-m", "My message"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

// --- List Directory Tool Struct --- Added
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListDirectoryArgs {
    pub path: String,
    // Default depth = 1, using Option<usize> and serde default
    #[serde(default = "default_depth")]
    pub depth: Option<usize>,
    // Default show_hidden = false
    #[serde(default)]
    pub show_hidden: bool,
}

// Function to provide the default value for depth
fn default_depth() -> Option<usize> {
    Some(1)
}

pub struct Tools;

impl Tools {
    // Returns the standard OpenAI format definition for the shell tool.
    pub fn shell_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "shell",
                "description": "Run a shell command and get the output",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to run"
                        }
                    },
                    "required": ["command"]
                }
            }
        })
    }

    pub fn read_file_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        })
    }

    pub fn write_file_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        })
    }

    // Updated definition for search_text
    pub fn search_text_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "search_text", // Renamed from search_code
                "description": "Search for text patterns in files, returning matching lines with context. Requires 'ripgrep' (rg) to be installed.", // Updated description
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Text or regex pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory or file path to search in (defaults to current directory)"
                        },
                        "file_glob": { // Renamed from file_pattern
                            "type": "string",
                            "description": "Glob pattern to filter files (e.g., '*.rs', '*.md', defaults to '*') - Use forward slashes ('/') as path separators in globs, even on Windows."
                        },
                        "case_sensitive": {
                            "type": "boolean",
                            "description": "Perform case-sensitive search (defaults to false)"
                        },
                        "context_lines": { // Added
                            "type": "integer",
                            "description": "Number of context lines before and after each match (defaults to 1)"
                        },
                        "max_results": { // Note: This now applies to lines, not files
                            "type": "integer",
                            "description": "Maximum number of matching lines to return (defaults to 50)"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        })
    }

    pub fn find_definition_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "find_definition",
                "description": "Find where a symbol is defined in the codebase",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Symbol name to search for (function, class, variable, etc.)"
                        },
                        "language": {
                            "type": "string",
                            "description": "Programming language to consider (affects search patterns)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory path to search in (defaults to current directory)"
                        }
                    },
                    "required": ["symbol"]
                }
            }
        })
    }

    pub fn user_input_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "user_input",
                "description": "Ask the user for input when a choice needs to be made",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "The question or prompt to show the user"
                        },
                        "options": {
                            "type": "array",
                            "description": "Optional list of specific options to present to the user",
                            "items": {
                                "type": "string"
                            }
                        }
                    },
                    "required": ["prompt"]
                }
            }
        })
    }

    // --- Unified Cargo Tool Definition ---
    // Removed #[allow(dead_code)]
    pub fn cargo_command_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "cargo_command", // Use a generic name
                "description": "Run a safe cargo command. Denied commands: publish, install, login, owner, etc.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The cargo subcommand to run (e.g., 'build', 'test', 'check', 'fmt', 'run')"
                        },
                        "args": {
                            "type": "array",
                            "description": "Arguments for the cargo subcommand (e.g., ['--release'], ['my_test', '--', '--nocapture'])",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["command"]
                }
            }
        })
    }

    // --- Unified Git Tool Definition ---
    // Removed #[allow(dead_code)]
    pub fn git_command_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "git_command", // Use a generic name
                "description": "Run a safe git command. Denied commands: push, reset, rebase, checkout, branch -D, etc.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The git subcommand to run (e.g., 'status', 'diff', 'add', 'commit', 'log')"
                        },
                        "args": {
                            "type": "array",
                            "description": "Arguments for the git subcommand (e.g., ['--porcelain'], ['--staged'], ['src/main.rs'], ['-m', 'My message'])",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["command"]
                }
            }
        })
    }

    // --- List Directory Tool Definition --- Added
    // Removed #[allow(dead_code)]
    pub fn list_directory_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "list_directory",
                "description": "List files and directories at a given path, respecting .gitignore. Output is raw text, one path per line.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The directory path to explore."
                        },
                        "depth": {
                            "type": "integer",
                            "description": "Maximum depth to recurse (1 lists immediate contents, 2 includes subdirs, etc.). Defaults to 1. Use 0 to list only the directory itself (if not hidden/ignored)."
                        },
                        "show_hidden": {
                            "type": "boolean",
                            "description": "Include hidden files/directories (starting with '.'). Defaults to false."
                        }
                    },
                    "required": ["path"]
                }
            }
        })
    }
} // End impl Tools

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
    #[allow(dead_code)] // Allow dead code because this is used externally (e.g., by AI configuration)
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
    #[allow(dead_code)] // Allow dead code because this is used externally (e.g., by AI configuration)
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
} // End impl Tools

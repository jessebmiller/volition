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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchTextArgs {
    pub pattern: String,
    pub path: Option<String>,
    pub file_glob: Option<String>,
    pub case_sensitive: Option<bool>,
    pub context_lines: Option<u32>,
    pub max_results: Option<usize>,
}

// Renamed from FindDefinitionArgs, removed language field
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FindRustDefinitionArgs {
    pub symbol: String,
    // Removed language field
    pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInputArgs {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CargoCommandArgs {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitCommandArgs {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListDirectoryArgs {
    pub path: String,
    #[serde(default = "default_depth")]
    pub depth: Option<usize>,
    #[serde(default)]
    pub show_hidden: bool,
}

fn default_depth() -> Option<usize> {
    Some(1)
}

pub struct Tools;

impl Tools {
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

    pub fn search_text_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "search_text",
                "description": "Search for text patterns in files, returning matching lines with context. Requires 'ripgrep' (rg) to be installed.",
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
                        "file_glob": {
                            "type": "string",
                            "description": "Glob pattern to filter files (e.g., '*.rs', '*.md', defaults to '*') - Use forward slashes ('/') as path separators in globs, even on Windows."
                        },
                        "case_sensitive": {
                            "type": "boolean",
                            "description": "Perform case-sensitive search (defaults to false)"
                        },
                        "context_lines": {
                            "type": "integer",
                            "description": "Number of context lines before and after each match (defaults to 1)"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of matching lines to return (defaults to 50)"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        })
    }

    // Renamed function, updated schema for find_rust_definition
    pub fn find_rust_definition_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "find_rust_definition", // Renamed tool
                "description": "Find where a Rust symbol (function, struct, enum, trait, etc.) is defined in the codebase. Searches *.rs files.", // Updated description
                "parameters": {
                    "type": "object",
                    "properties": {
                        "symbol": {
                            "type": "string",
                            "description": "Rust symbol name to search for (function, struct, enum, trait, macro, etc.)" // Updated description
                        },
                        // Removed language parameter
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

    pub fn cargo_command_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "cargo_command",
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

    pub fn git_command_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "git_command",
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

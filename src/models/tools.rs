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
pub struct SearchCodeArgs {
    pub pattern: String,
    pub path: Option<String>,
    pub file_pattern: Option<String>,
    pub case_sensitive: Option<bool>,
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

    pub fn search_code_definition() -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "search_code",
                "description": "Search for patterns in code files",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Pattern to search for (regex supported)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory path to search in (defaults to current directory)"
                        },
                        "file_pattern": {
                            "type": "string",
                            "description": "File pattern to include (e.g., '*.rs', '*.{js,ts}')"
                        },
                        "case_sensitive": {
                            "type": "boolean",
                            "description": "Whether to perform case-sensitive search (defaults to false)"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of results to return (defaults to 100)"
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
}

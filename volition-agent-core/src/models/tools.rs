// volition-agent-core/src/models/tools.rs
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

// --- Structs for AI Tool Interaction ---

/// Represents a tool call requested by the AI model.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String, // Usually "function"
    pub function: ToolFunction,
}

/// Represents the function call details within a ToolCall.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolFunction {
    pub name: String,
    /// Arguments are expected to be a JSON string by the AI model
    pub arguments: String,
}

// --- Generic Structs for Tool Definition and Input (Core Library) ---

/// Defines the schema for a tool that can be presented to the AI.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ToolParametersDefinition,
}

/// Defines the parameters structure for a tool.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolParametersDefinition {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: HashMap<String, ToolParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

/// Defines a single parameter within a tool's schema.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolParameter {
    #[serde(rename = "type")]
    pub param_type: ToolParameterType,
    pub description: String,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ToolParameter>>,
}

/// Represents the type of a tool parameter.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolParameterType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
}

/// Represents the input arguments provided for a tool execution at runtime.
/// Uses a HashMap to store arguments generically.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ToolInput {
    pub arguments: HashMap<String, JsonValue>,
}

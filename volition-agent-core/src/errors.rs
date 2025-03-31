// volition-agent-core/src/errors.rs
use thiserror::Error;

/// Errors that can occur during Agent execution.
#[derive(Error, Debug)]
pub enum AgentError {
    /// Error related to configuration loading or validation.
    #[error("Configuration Error: {0}")]
    Config(String),

    /// Error during interaction with the AI model API.
    #[error("API Error: {0}")]
    Api(#[source] anyhow::Error),

    /// Error originating from within an agent strategy.
    #[error("Strategy Error: {0}")]
    Strategy(String),

    /// Error related to tool definition or execution (Old system).
    #[error("Tool Error: {0}")]
    Tool(String),
    
    /// Error related to MCP connection or tool call (New system).
    #[error("MCP Error: {0}")]
    Mcp(#[source] anyhow::Error), // Added Mcp variant

    /// Error during task delegation (if implemented).
    #[error("Delegation Error: {0}")]
    Delegation(String),

    /// Error during user interaction.
    #[error("User Interaction Error: {0}")]
    Ui(#[source] anyhow::Error),
}

// Helper implementations (optional)
impl AgentError {
    pub fn config(msg: impl Into<String>) -> Self {
        AgentError::Config(msg.into())
    }
    // Keep other helpers if needed
}

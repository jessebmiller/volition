use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("API call failed: {0}")]
    Api(#[from] anyhow::Error),
    #[error("Tool execution failed for '{tool_name}': {source}")]
    ToolExecution {
        tool_name: String,
        source: anyhow::Error,
    },
    #[error("Tool argument parsing failed for '{tool_name}': {source}")]
    ToolArgumentParsing {
        tool_name: String,
        source: serde_json::Error,
    },
    #[error("Strategy error: {0}")]
    Strategy(String),
    #[error("Delegation failed: {0}")]
    Delegation(String),
    #[error("User interaction failed: {0}")]
    UserInteraction(anyhow::Error),
    #[error("Agent stopped: {0}")]
    Stopped(String),
    #[error("Other error: {0}")]
    Other(String),
}

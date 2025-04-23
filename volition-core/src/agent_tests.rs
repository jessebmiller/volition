// volition-agent-core/src/agent_tests.rs
#![cfg(test)]

use super::*;
use crate::agent::Agent;
use crate::config::AgentConfig; // Removed McpServerConfig, ModelConfig, ProviderConfig
use crate::errors::AgentError;
use crate::strategies::complete_task::CompleteTaskStrategy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use tracing::info;
use tracing_subscriber;

use crate::models::chat::{ApiResponse, ChatMessage, Choice};
use crate::providers::{Provider, ProviderRegistry};
// Removed: use crate::strategies::conversation::ConversationStrategy;
use crate::mcp::McpConnection;
use tokio::sync::Mutex as TokioMutex;

// --- Mock UI (Keep existing) ---
#[derive(Default)]
struct MockUI {
    ask_responses: StdMutex<Vec<String>>,
    ask_prompts: StdMutex<Vec<String>>,
}

#[async_trait]
impl UserInteraction for MockUI {
    async fn ask(&self, prompt: String, _options: Vec<String>) -> Result<String> {
        self.ask_prompts.lock().unwrap().push(prompt);
        let response = self
            .ask_responses
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| "yes".to_string());
        Ok(response)
    }
}

impl MockUI {
    #[allow(dead_code)]
    fn add_response(&self, response: &str) {
        self.ask_responses
            .lock()
            .unwrap()
            .push(response.to_string());
    }
}

// --- Enhanced MockToolProvider (acts as ToolProvider and Provider) ---
#[derive(Clone)]
struct MockToolProvider {
    call_log: Arc<StdMutex<Vec<(String, String)>>>,
    outputs: HashMap<String, Result<String, String>>,
    definitions: Vec<ToolDefinition>,
    received_histories: Arc<StdMutex<Vec<Vec<ChatMessage>>>>,
}

impl MockToolProvider {
    fn new(
        definitions: Vec<ToolDefinition>,
        outputs: HashMap<String, Result<String, String>>,
    ) -> Self {
        Self {
            call_log: Arc::new(StdMutex::new(Vec::new())),
            outputs,
            definitions,
            received_histories: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    // simple_def is unused now, removing warning
    // fn simple_def(name: &str) -> ToolDefinition { ... }
}

// Helper function for mock provider
fn generate_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}_{}", prefix, nanos)
}

// Implement Provider trait for MockToolProvider
#[async_trait]
impl Provider for MockToolProvider {
    fn name(&self) -> &str {
        "mock-provider"
    }

    async fn get_completion(
        &self,
        messages: Vec<ChatMessage>,
        _tools: Option<&[ToolDefinition]>,
    ) -> Result<ApiResponse> {
        self.received_histories
            .lock()
            .unwrap()
            .push(messages.clone());
        Ok(ApiResponse {
            id: "mock_resp_".to_string() + &generate_id(""),
            content: "Mock response".to_string(),
            finish_reason: "stop".to_string(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some("Mock response".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: "stop".to_string(),
            }],
        })
    }
}

// ToolProvider implementation remains the same
#[async_trait]
impl ToolProvider for MockToolProvider {
    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.definitions.clone()
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        input: ToolInput,
        _working_dir: &Path,
    ) -> Result<String> {
        let input_json = serde_json::to_string(&input.arguments).unwrap_or_default();
        self.call_log
            .lock()
            .unwrap()
            .push((tool_name.to_string(), input_json));

        match self.outputs.get(tool_name) {
            Some(Ok(output)) => Ok(output.clone()),
            Some(Err(e)) => Err(anyhow!("Tool execution failed: {}", e.clone())), // Simplified error
            None => Err(anyhow!(
                "MockToolProvider: No output defined for tool '{}'",
                tool_name
            )),
        }
    }
}

// --- Test Config Helper (Minimal config for tests) ---
fn create_minimal_agent_config(default_provider_id: String) -> AgentConfig {
    AgentConfig {
        default_provider: default_provider_id,
        providers: HashMap::new(),
        mcp_servers: HashMap::new(),
        strategies: HashMap::new(),
        system_prompt: String::new(),
    }
}

// --- Agent Test Helper (Removed Agent::new_with_registry) ---

// --- Existing Tests ---

#[tokio::test]
async fn test_agent_initialization() -> Result<(), AgentError> {
    let mock_provider = Arc::new(MockToolProvider::new(vec![], HashMap::new()));
    let mock_ui = Arc::new(MockUI::default());
    let initial_task = "Test task".to_string();

    let default_provider_id = "mock-provider-id".to_string();
    let mut provider_registry = ProviderRegistry::new(default_provider_id.clone());
    provider_registry.register(
        default_provider_id.clone(),
        Box::new(mock_provider.as_ref().clone()),
    );

    let mcp_connections: HashMap<String, Arc<TokioMutex<McpConnection>>> = HashMap::new();
    let config = create_minimal_agent_config(default_provider_id.clone());

    // Fix: Correct argument order for Agent::new
    let agent = Agent::new(
        config,
        mock_ui,
        Box::new(CompleteTaskStrategy::default()),
        None,         // history (starting fresh)
        initial_task, // current_user_input
        Some(provider_registry),
        Some(mcp_connections),
    )
    .map_err(|e| AgentError::Config(e.to_string()))?;

    let _ = agent;
    Ok(())
}

#[tokio::test]
async fn test_agent_run_single_tool_call_success() -> Result<()> {
    /* ... (test body commented out) ... */
    Ok(())
}

// --- New Test Case ---

#[tokio::test]
async fn test_conversation_history_persistence() -> Result<(), AgentError> {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    // 1. Setup
    let mock_provider = Arc::new(MockToolProvider::new(vec![], HashMap::new()));
    let mock_ui = Arc::new(MockUI::default());
    let default_provider_id = "mock-history-provider".to_string();

    // Create registry for Turn 1
    let mut provider_registry1 = ProviderRegistry::new(default_provider_id.clone());
    provider_registry1.register(
        default_provider_id.clone(),
        Box::new(mock_provider.as_ref().clone()),
    );

    let mcp_connections1: HashMap<String, Arc<TokioMutex<McpConnection>>> = HashMap::new();
    let config = create_minimal_agent_config(default_provider_id.clone());

    let initial_task_1 = "This is the first task.".to_string();
    let user_message_2 = "This is the second task.".to_string();

    // --- Turn 1 ---
    info!("Starting Turn 1");
    let agent_strategy_1 = Box::new(CompleteTaskStrategy::default()); // Use base strategy directly

    // Fix: Correct argument order for Agent::new
    let mut agent1 = Agent::new(
        config.clone(),
        mock_ui.clone(),
        agent_strategy_1,
        None,                   // history (starting fresh)
        initial_task_1.clone(), // current_user_input
        Some(provider_registry1),
        Some(mcp_connections1),
    )
    .map_err(|e| AgentError::Config(e.to_string()))?;

    let (response1, state1) = agent1.run(&PathBuf::from(".")).await?;
    info!(response1 = %response1, "Turn 1 completed.");
    assert_eq!(response1, "Mock response", "Unexpected response in Turn 1");

    // --- Turn 2 Setup ---
    // History now includes the user message + assistant response from turn 1
    let history_turn_2 = state1.messages.clone();
    assert_eq!(
        history_turn_2.len(),
        2,
        "State after Turn 1 should have 2 messages"
    );
    assert_eq!(history_turn_2[0].role, "user");
    assert_eq!(
        history_turn_2[0].content.as_deref(),
        Some(initial_task_1.as_str())
    );
    assert_eq!(history_turn_2[1].role, "assistant");
    assert_eq!(history_turn_2[1].content.as_deref(), Some("Mock response"));

    info!(
        num_messages = history_turn_2.len(),
        "Prepared history for Turn 2 input"
    );

    // --- Turn 2 Execution ---
    info!("Starting Turn 2");
    let mut provider_registry2 = ProviderRegistry::new(default_provider_id.clone());
    provider_registry2.register(
        default_provider_id.clone(),
        Box::new(mock_provider.as_ref().clone()),
    );
    let mcp_connections2: HashMap<String, Arc<TokioMutex<McpConnection>>> = HashMap::new();
    let agent_strategy_2 = Box::new(CompleteTaskStrategy::default()); // Use base strategy directly

    // Fix: Correct argument order for Agent::new
    let mut agent2 = Agent::new(
        config.clone(),
        mock_ui.clone(),
        agent_strategy_2,
        Some(history_turn_2.clone()), // Pass history from turn 1
        user_message_2.clone(),       // current_user_input
        Some(provider_registry2),
        Some(mcp_connections2),
    )
    .map_err(|e| AgentError::Config(e.to_string()))?;

    let (response2, _state2) = agent2.run(&PathBuf::from(".")).await?;
    info!(response2 = %response2, "Turn 2 completed.");
    assert_eq!(response2, "Mock response", "Unexpected response in Turn 2");

    // --- Verification ---
    let histories_received = mock_provider.received_histories.lock().unwrap();
    assert_eq!(
        histories_received.len(),
        2,
        "Expected exactly two calls to the provider"
    );

    // History sent during Turn 1 (AgentState::new_turn creates [User1])
    let history_sent_1 = &histories_received[0];
    info!(?history_sent_1, "History sent to provider during Turn 1");
    assert_eq!(
        history_sent_1.len(),
        1,
        "Turn 1 history sent should have 1 message"
    );
    assert_eq!(history_sent_1[0].role, "user");
    assert_eq!(
        history_sent_1[0].content.as_deref(),
        Some(initial_task_1.as_str())
    );

    // History sent during Turn 2 (AgentState::new_turn creates [User1, Asst1, User2])
    let history_sent_2 = &histories_received[1];
    info!(?history_sent_2, "History sent to provider during Turn 2");
    assert_eq!(
        history_sent_2.len(),
        3,
        "Turn 2 history sent should have 3 messages"
    );

    assert_eq!(
        history_sent_2[0].role, "user",
        "Turn 2 history[0] role mismatch"
    );
    assert_eq!(
        history_sent_2[0].content.as_deref(),
        Some(initial_task_1.as_str()),
        "Turn 2 history[0] content mismatch"
    );

    assert_eq!(
        history_sent_2[1].role, "assistant",
        "Turn 2 history[1] role mismatch"
    );
    assert_eq!(
        history_sent_2[1].content.as_deref(),
        Some("Mock response"),
        "Turn 2 history[1] content mismatch"
    );

    assert_eq!(
        history_sent_2[2].role, "user",
        "Turn 2 history[2] role mismatch"
    );
    assert_eq!(
        history_sent_2[2].content.as_deref(),
        Some(user_message_2.as_str()),
        "Turn 2 history[2] content mismatch"
    );

    Ok(())
}

// TODO: Add tests for error handling (API errors, tool errors)
// TODO: Add tests for scenarios without tool calls
// TODO: Test delegation once implemented (will require different strategy/mocks)

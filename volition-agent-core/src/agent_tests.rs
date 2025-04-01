// volition-agent-core/src/agent_tests.rs
#![cfg(test)]

use super::*;
use crate::agent::Agent; // Added import
use crate::config::RuntimeConfig; // Added import
// use crate::errors::AgentError; // Removed unused import
use crate::strategies::complete_task::CompleteTaskStrategy;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result}; // Added anyhow macro import
use httpmock::prelude::*;
use serde_json::json;
use tracing::debug; // Import debug for logging
use tracing_subscriber;

#[derive(Default)]
struct MockUI {
    ask_responses: Mutex<Vec<String>>,
    ask_prompts: Mutex<Vec<String>>,
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
            .unwrap_or_else(|| "yes".to_string()); // Default to "yes" for tests
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

#[derive(Clone)]
struct MockToolProvider {
    call_log: Arc<Mutex<Vec<(String, String)>>>,
    outputs: HashMap<String, Result<String, String>>,
    definitions: Vec<ToolDefinition>,
}

impl MockToolProvider {
    fn new(
        definitions: Vec<ToolDefinition>,
        outputs: HashMap<String, Result<String, String>>,
    ) -> Self {
        Self {
            call_log: Arc::new(Mutex::new(Vec::new())),
            outputs,
            definitions,
        }
    }

    fn simple_def(name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: format!("Mock tool {}", name),
            parameters: ToolParametersDefinition {
                param_type: "object".to_string(),
                properties: HashMap::from([(
                    "arg".to_string(),
                    ToolParameter {
                        param_type: ToolParameterType::String,
                        description: "An argument".to_string(),
                        enum_values: None,
                        items: None,
                    },
                )]),
                required: vec![],
            },
        }
    }
}

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
            Some(Err(e)) => Err(anyhow!("{}", e.clone())), // Use imported anyhow!
            None => Err(anyhow!(
                // Use imported anyhow!
                "MockToolProvider: No output defined for tool '{}'",
                tool_name
            )),
        }
    }
}

const TEST_ENDPOINT_PATH: &str = "/test/completions";

fn create_test_config(mock_server_base_url: &str) -> RuntimeConfig {
    let mock_endpoint = format!("{}{}", mock_server_base_url, TEST_ENDPOINT_PATH);
    let mut models = HashMap::new();
    models.insert(
        "test-model-key".to_string(),
        ModelConfig {
            model_name: "test-model".to_string(),
            endpoint: Some(mock_endpoint), // Wrapped in Some()
            parameters: Some(toml::Value::Table(Default::default())), // Wrapped in Some()
        },
    );
    RuntimeConfig {
        system_prompt: "Test System Prompt".to_string(),
        selected_model: "test-model-key".to_string(),
        models,
        api_key: "test-api-key".to_string(),
    }
}

#[tokio::test]
async fn test_agent_initialization() {
    let config = create_test_config("http://unused");
    let mock_provider = Arc::new(MockToolProvider::new(vec![], HashMap::new()));
    let mock_ui = Arc::new(MockUI::default());
    let initial_task = "Test task".to_string();

    let agent_result = Agent::new(
        config,
        mock_provider,
        mock_ui,
        Box::new(CompleteTaskStrategy), // Removed ::new()
        initial_task,
    );
    assert!(agent_result.is_ok());
}

#[tokio::test]
async fn test_agent_run_single_tool_call_success() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    let server = MockServer::start_async().await;
    let mock_base_url = server.base_url();

    let tool_name = "get_weather";
    let tool_defs = vec![MockToolProvider::simple_def(tool_name)];
    let mut tool_outputs = HashMap::new();
    let tool_output_content = "The weather is sunny.".to_string();
    tool_outputs.insert(tool_name.to_string(), Ok(tool_output_content.clone()));
    let mock_provider = Arc::new(MockToolProvider::new(tool_defs.clone(), tool_outputs));
    let mock_ui = Arc::new(MockUI::default());

    let config = create_test_config(&mock_base_url);
    let initial_task = "What is the weather?".to_string();

    let mut agent = Agent::new(
        config.clone(),
        mock_provider.clone(),
        mock_ui,
        Box::new(CompleteTaskStrategy), // Removed ::new()
        initial_task.clone(),
    )?;

    let tool_call_id = "call_123";
    let tool_args = json!({ "arg": "today" });

    // Mock 1: Initial user message -> Tool Call Request
    let expected_messages_1 = json!([
        { "role": "user", "content": initial_task },
    ]);
    let model_name = config.selected_model_config().unwrap().model_name.clone(); // Clone model name
    let expected_body_1 = json!({
        "model": model_name,
        "messages": expected_messages_1,
        "tools": [ { "type": "function", "function": tool_defs[0] } ]
    });
    let mock_response_1 = json!({
        "id": "resp1",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": tool_call_id,
                    "type": "function",
                    "function": { "name": tool_name, "arguments": tool_args.to_string() }
                }]
            },
            "finish_reason": "tool_use"
        }]
    });
    let api_mock_1 = server
        .mock_async(|when, then| {
            when.method(POST)
                .path(TEST_ENDPOINT_PATH)
                .json_body(expected_body_1.clone());
            then.status(200).json_body(mock_response_1);
        })
        .await;

    // Mock 2: Tool Result -> Final Answer
    let final_answer = "The weather today is sunny.".to_string();
    let mock_response_2 = json!({
        "id": "resp2",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": final_answer,
                "tool_calls": null
            },
            "finish_reason": "stop_sequence"
        }]
    });
    // Adjusted expected messages: remove "content": null from assistant message
    let expected_messages_2 = json!([
        { "role": "user", "content": initial_task },
        {
            "role": "assistant",
            // "content": null, // Removed this line
            "tool_calls": [{
                "id": tool_call_id,
                "type": "function",
                "function": { "name": tool_name, "arguments": tool_args.to_string() }
            }]
        },
        {
            "role": "tool",
            "content": tool_output_content,
            "tool_call_id": tool_call_id
        }
    ]);
    let expected_body_2 = json!({
        "model": model_name, // Use cloned model name
        "messages": expected_messages_2,
        "tools": [ { "type": "function", "function": tool_defs[0] } ]
    });
    let api_mock_2 = server
        .mock_async(|when, then| {
            when.method(POST)
                .path(TEST_ENDPOINT_PATH)
                .json_body(expected_body_2.clone());
            then.status(200).json_body(mock_response_2);
        })
        .await;

    let working_dir = PathBuf::from(".");
    debug!("Running agent...");

    let agent_result = agent.run(&working_dir).await;
    debug!("Agent run finished. Result: {:?}", agent_result);

    debug!("Checking mock 1 hits...");
    api_mock_1.assert_hits(1);
    debug!("Checking mock 2 hits...");
    api_mock_2.assert_hits(1);

    assert!(
        agent_result.is_ok(),
        "Agent run failed: {:?}",
        agent_result.err()
    );
    let final_message = agent_result.unwrap();

    let calls = mock_provider.call_log.lock().unwrap();
    assert_eq!(calls.len(), 1, "Expected exactly one tool call");
    assert_eq!(calls[0].0, tool_name, "Tool name mismatch");
    let logged_args: serde_json::Value =
        serde_json::from_str(&calls[0].1).expect("Failed to parse logged tool arguments");
    assert_eq!(logged_args, tool_args, "Tool arguments mismatch");

    assert_eq!(final_message, final_answer, "Final message mismatch");

    Ok(())
}

// TODO: Add tests for error handling (API errors, tool errors)
// TODO: Add tests for scenarios without tool calls
// TODO: Test delegation once implemented (will require different strategy/mocks)

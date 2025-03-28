// volition-agent-core/src/agent_tests.rs
#![cfg(test)]

use super::*;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use httpmock::prelude::*;
use serde_json::json;
use tracing_subscriber;

// --- Mock Tool Provider ---

#[derive(Clone)]
struct MockToolProvider {
    call_log: Arc<Mutex<Vec<(String, String)>>>,
    outputs: HashMap<String, Result<String, String>>,
    definitions: Vec<ToolDefinition>,
}

impl MockToolProvider {
    fn new(definitions: Vec<ToolDefinition>, outputs: HashMap<String, Result<String, String>>) -> Self {
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

    async fn execute_tool(&self, tool_name: &str, input: ToolInput, _working_dir: &Path) -> Result<String> {
        let input_json = serde_json::to_string(&input.arguments).unwrap_or_default();
        self.call_log.lock().unwrap().push((tool_name.to_string(), input_json));

        match self.outputs.get(tool_name) {
            Some(Ok(output)) => Ok(output.clone()),
            Some(Err(e)) => Err(anyhow!("{}", e.clone())),
            None => Err(anyhow!("MockToolProvider: No output defined for tool '{}'", tool_name)),
        }
    }
}

// --- Test Helpers ---

const TEST_ENDPOINT_PATH: &str = "/test/completions";

// Updated test helper: Removed project_root
fn create_test_config(mock_server_base_url: &str) -> RuntimeConfig {
    let mock_endpoint = format!("{}{}", mock_server_base_url, TEST_ENDPOINT_PATH);
    let mut models = HashMap::new();
    models.insert(
        "test-model-key".to_string(),
        ModelConfig {
            model_name: "test-model".to_string(),
            endpoint: mock_endpoint,
            parameters: toml::Value::Table(Default::default()),
        },
    );
    RuntimeConfig {
        system_prompt: "Test System Prompt".to_string(),
        selected_model: "test-model-key".to_string(),
        models,
        api_key: "test-api-key".to_string(),
        // project_root: PathBuf::from("."), // Removed field
    }
}

// --- Agent Tests ---

#[tokio::test]
async fn test_agent_initialization() {
    let config = create_test_config("http://unused");
    let mock_provider = Arc::new(MockToolProvider::new(vec![], HashMap::new()));
    let agent_result = Agent::new(config, mock_provider);
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

    let config = create_test_config(&mock_base_url);
    let agent = Agent::new(config.clone(), mock_provider.clone())?;

    let goal = "What is the weather?";
    let tool_call_id = "call_123";
    let tool_args = json!({ "arg": "today" });

    // --- Mock 1 Setup (Initial Request) ---
    let expected_messages_1 = json!([
        { "role": "system", "content": config.system_prompt },
        { "role": "user", "content": goal },
    ]);
    let expected_body_1 = json!({
        "model": config.selected_model_config().unwrap().model_name,
        "messages": expected_messages_1,
        "tools": [
            { "type": "function", "function": tool_defs[0] }
        ]
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
                    "function": {
                        "name": tool_name,
                        "arguments": tool_args.to_string()
                    }
                }]
            },
            "finish_reason": "tool_calls"
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

    // --- Mock 2 Setup (Request with Tool Result) ---
    let final_answer = "The weather today is sunny.";
    let mock_response_2 = json!({
        "id": "resp2",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": final_answer,
                "tool_calls": null
            },
            "finish_reason": "stop"
        }]
    });
    let expected_messages_2 = json!([
        { "role": "system", "content": config.system_prompt },
        { "role": "user", "content": goal },
        {
            "role": "assistant",
            "tool_calls": [{
                "id": tool_call_id,
                "type": "function",
                "function": {
                    "name": tool_name,
                    "arguments": tool_args.to_string()
                }
            }]
        },
        {
            "role": "tool",
            "content": tool_output_content,
            "tool_call_id": tool_call_id
        }
    ]);
    let expected_body_2 = json!({
        "model": config.selected_model_config().unwrap().model_name,
        "messages": expected_messages_2,
        "tools": [
            { "type": "function", "function": tool_defs[0] }
        ]
    });
    let api_mock_2 = server
        .mock_async(|when, then| {
            when.method(POST)
                .path(TEST_ENDPOINT_PATH)
                .json_body(expected_body_2.clone());
            then.status(200).json_body(mock_response_2);
        })
        .await;

    // --- Run Agent ---
    let working_dir = PathBuf::from(".");
    println!("Running agent...");
    let agent_output_result = agent.run(goal, &working_dir).await;
    println!("Agent run finished. Result: {:?}", agent_output_result);

    // --- Assertions ---
    println!("Checking mock 1 hits...");
    api_mock_1.assert_hits(1);
    println!("Checking mock 2 hits...");
    api_mock_2.assert_hits(1);

    assert!(
        agent_output_result.is_ok(),
        "Agent run failed: {:?}",
        agent_output_result.err()
    );
    let agent_output = agent_output_result.unwrap();

    let calls = mock_provider.call_log.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, tool_name);
    assert_eq!(calls[0].1, tool_args.to_string());

    assert!(agent_output.suggested_summary.is_some());
    assert_eq!(agent_output.applied_tool_results.len(), 1);
    let tool_result = &agent_output.applied_tool_results[0];
    assert_eq!(tool_result.tool_call_id, tool_call_id);
    assert_eq!(tool_result.tool_name, tool_name);
    assert_eq!(tool_result.input, tool_args);
    assert_eq!(tool_result.output, tool_output_content);
    assert_eq!(tool_result.status, ToolExecutionStatus::Success);
    assert_eq!(
        agent_output.final_state_description,
        Some(final_answer.to_string())
    );

    Ok(())
}

// TODO: More tests

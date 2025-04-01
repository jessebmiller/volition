// volition-agent-core/src/strategies/complete_task.rs
// Removed unused Agent import
use crate::errors::AgentError;
// Removed unused ChatMessage import
use crate::models::chat::ApiResponse;
use crate::strategies::{NextStep, Strategy};
use crate::UserInteraction;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::info;

#[derive(Default)]
pub struct CompleteTaskStrategy;

#[async_trait]
impl<UI: UserInteraction + 'static> Strategy<UI> for CompleteTaskStrategy {
    fn name(&self) -> &'static str {
        "CompleteTask"
    }

    fn initialize_interaction(
        &mut self,
        state: &mut crate::AgentState,
    ) -> Result<NextStep, AgentError> {
        info!("Initializing CompleteTask strategy.");
        Ok(NextStep::CallApi(state.clone()))
    }

    fn process_api_response(
        &mut self,
        state: &mut crate::AgentState,
        response: ApiResponse,
    ) -> Result<NextStep, AgentError> {
        info!("Processing API response for CompleteTask.");
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(AgentError::Api(anyhow!("No choices returned from API")))?;

        state.add_message(choice.message.clone());

        if let Some(tool_calls) = choice.message.tool_calls {
            state.set_tool_calls(tool_calls);
            Ok(NextStep::CallTools(state.clone()))
        } else {
            let final_content = choice
                .message
                .content
                .unwrap_or_else(|| "Task completed.".to_string());
            Ok(NextStep::Completed(final_content))
        }
    }

    fn process_tool_results(
        &mut self,
        state: &mut crate::AgentState,
        results: Vec<crate::ToolResult>,
    ) -> Result<NextStep, AgentError> {
        info!("Processing tool results for CompleteTask.");
        state.add_tool_results(results);
        Ok(NextStep::CallApi(state.clone()))
    }

    fn process_delegation_result(
        &mut self,
        _state: &mut crate::AgentState,
        _result: crate::DelegationResult,
    ) -> Result<NextStep, AgentError> {
        Err(AgentError::Strategy(
            "Delegation not supported by CompleteTaskStrategy".to_string(),
        ))
    }
}

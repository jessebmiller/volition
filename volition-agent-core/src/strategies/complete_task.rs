use super::{DelegationResult, NextStep, Strategy};
use crate::errors::AgentError;
use crate::{AgentState, ApiResponse, ToolResult};
use anyhow::anyhow; // Import anyhow for error creation

pub struct CompleteTaskStrategy;

impl CompleteTaskStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CompleteTaskStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for CompleteTaskStrategy {
    fn name(&self) -> &'static str {
        "CompleteTask"
    }

    fn initialize_interaction(&self, state: &mut AgentState) -> Result<NextStep, AgentError> {
        // For CompleteTask, the initial step is usually to call the API
        // with the user's request already in the state.
        Ok(NextStep::CallApi(state.clone())) // Assuming AgentState is cloneable
    }

    fn process_api_response(
        &self,
        state: &mut AgentState,
        response: ApiResponse,
    ) -> Result<NextStep, AgentError> {
        // Get the first choice, or return error if none
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AgentError::Api(anyhow!("API response contained no choices")))?;

        // Add the assistant's response message to the state
        let assistant_message = choice.message.clone(); // Clone for state update
        state.add_message(assistant_message.clone()); // Clone again for potential use below

        // Check for tool calls in the message
        // Use `ref` to borrow the vector instead of moving it
        if let Some(ref tool_calls) = assistant_message.tool_calls {
            if !tool_calls.is_empty() {
                // Clone the tool_calls when setting them in the state
                state.set_tool_calls(tool_calls.clone());
                return Ok(NextStep::CallTools(state.clone()));
            }
        }

        // If no tool calls, check the finish reason
        match choice.finish_reason.as_str() {
            "tool_use" | "tool_calls" => {
                // This case handles the scenario where finish_reason indicates tool use,
                // even if the tool_calls vector might be empty (which would be unusual).
                // We still need to handle the possibility of assistant_message.tool_calls being None here.
                state.set_tool_calls(assistant_message.tool_calls.clone().unwrap_or_default());
                Ok(NextStep::CallTools(state.clone()))
            }
            "stop" | "stop_sequence" | "max_tokens" | "end_turn" => {
                // Task is considered complete.
                // Return the content of the assistant message.
                Ok(NextStep::Completed(
                    assistant_message.content.unwrap_or_default(),
                ))
            }
            other_reason => {
                // Unknown finish reason.
                if !other_reason.is_empty() {
                    tracing::warn!(
                        "Unknown API finish reason: '{}'. Assuming completion.",
                        other_reason
                    );
                } else {
                    tracing::warn!("Empty API finish reason. Assuming completion.");
                }
                Ok(NextStep::Completed(
                    assistant_message.content.unwrap_or_default(),
                ))
            }
        }
    }

    fn process_tool_results(
        &self,
        state: &mut AgentState,
        results: Vec<ToolResult>,
    ) -> Result<NextStep, AgentError> {
        // Add tool results to the state
        state.add_tool_results(results); // This adds tool messages
                                         // After getting tool results, call the API again to let the LLM process them.
        Ok(NextStep::CallApi(state.clone()))
    }

    fn process_delegation_result(
        &self,
        _state: &mut AgentState,
        _result: DelegationResult,
    ) -> Result<NextStep, AgentError> {
        // CompleteTaskStrategy does not initiate delegation, so receiving
        // a delegation result is unexpected.
        Err(AgentError::Strategy(format!(
            "{} received unexpected delegation result",
            self.name()
        )))
    }
}

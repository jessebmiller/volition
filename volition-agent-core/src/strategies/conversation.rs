use super::{DelegationInput, DelegationResult, NextStep, Strategy};
use crate::{AgentState, ApiResponse, ToolResult};
use crate::errors::AgentError;
use anyhow::anyhow;

pub struct ConversationStrategy {
    inner_strategy: Box<dyn Strategy + Send + Sync>,
    conversation_state: Option<AgentState>,
    end_current_task: bool,
}

impl ConversationStrategy {
    pub fn new(inner_strategy: Box<dyn Strategy + Send + Sync>) -> Self {
        Self {
            inner_strategy,
            conversation_state: None,
            end_current_task: false,
        }
    }

    pub fn with_state(
        inner_strategy: Box<dyn Strategy + Send + Sync>,
        existing_state: AgentState,
    ) -> Self {
        Self {
            inner_strategy,
            conversation_state: Some(existing_state),
            end_current_task: false,
        }
    }

    pub fn get_conversation_state(&self) -> Option<&AgentState> {
        self.conversation_state.as_ref()
    }

    pub fn get_conversation_state_mut(&mut self) -> Option<&mut AgentState> {
        self.conversation_state.as_mut()
    }

    pub fn take_conversation_state(&mut self) -> Option<AgentState> {
        self.conversation_state.take()
    }
}

impl Strategy for ConversationStrategy {
    fn name(&self) -> &'static str {
        "Conversation"
    }

    fn initialize_interaction(&self, state: &mut AgentState) -> Result<NextStep, AgentError> {
        // If we have existing conversation state, merge it with the new state
        if let Some(existing_state) = &self.conversation_state {
            // Create a new state that has all previous messages plus the new user message
            let user_message = state.messages.last().cloned().ok_or_else(|| {
                AgentError::Strategy("State contains no initial message".to_string())
            })?;

            // Use existing conversation messages but add the new user message
            state.messages = existing_state.messages.clone();
            state.add_message(user_message);
        }

        // Delegate to the inner strategy
        self.inner_strategy.initialize_interaction(state)
    }

    fn process_api_response(
        &self,
        state: &mut AgentState,
        response: ApiResponse,
    ) -> Result<NextStep, AgentError> {
        // Delegate to the inner strategy
        self.inner_strategy.process_api_response(state, response)
    }

    fn process_tool_results(
        &self,
        state: &mut AgentState,
        results: Vec<ToolResult>,
    ) -> Result<NextStep, AgentError> {
        // Delegate to the inner strategy
        self.inner_strategy.process_tool_results(state, results)
    }

    fn process_delegation_result(
        &self,
        state: &mut AgentState,
        result: DelegationResult,
    ) -> Result<NextStep, AgentError> {
        // Delegate to the inner strategy
        self.inner_strategy.process_delegation_result(state, result)
    }
}
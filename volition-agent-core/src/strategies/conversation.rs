// volition-agent-core/src/strategies/conversation.rs
use super::{DelegationResult, NextStep, Strategy};
use crate::agent::Agent;
use crate::errors::AgentError;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::UserInteraction;
use anyhow::Result;
use async_trait::async_trait;

// Add generic parameter <UI>
pub struct ConversationStrategy<UI: UserInteraction + 'static> {
    conversation_history: Vec<ChatMessage>,
    inner_strategy: Box<dyn Strategy<UI> + Send + Sync>,
    end_current_task: bool,
}

impl<UI: UserInteraction + 'static> ConversationStrategy<UI> {
    pub fn new(inner_strategy: Box<dyn Strategy<UI> + Send + Sync>) -> Self {
        Self {
            conversation_history: Vec::new(),
            inner_strategy,
            end_current_task: false,
        }
    }

    pub fn with_history(
        inner_strategy: Box<dyn Strategy<UI> + Send + Sync>,
        history: Vec<ChatMessage>,
    ) -> Self {
        Self {
            conversation_history: history,
            inner_strategy,
            end_current_task: false,
        }
    }
}

#[async_trait]
impl<UI: UserInteraction + 'static> Strategy<UI> for ConversationStrategy<UI> {
    fn name(&self) -> &'static str {
        "Conversation"
    }

    fn initialize_interaction(
        &mut self,
        state: &mut crate::AgentState,
    ) -> Result<NextStep, AgentError> {
        let current_messages = std::mem::take(&mut state.messages);
        state.messages = self.conversation_history.clone();
        state.messages.extend(current_messages);
        self.end_current_task = false;
        self.inner_strategy.initialize_interaction(state)
    }

    fn process_api_response(
        &mut self,
        state: &mut crate::AgentState,
        response: ApiResponse,
    ) -> Result<NextStep, AgentError> {
        let next_step = self.inner_strategy.process_api_response(state, response)?;
        self.update_history_and_check_completion(state, &next_step);
        Ok(next_step)
    }

    fn process_tool_results(
        &mut self,
        state: &mut crate::AgentState,
        results: Vec<crate::ToolResult>,
    ) -> Result<NextStep, AgentError> {
        let next_step = self.inner_strategy.process_tool_results(state, results)?;
        self.update_history_and_check_completion(state, &next_step);
        Ok(next_step)
    }

    fn process_delegation_result(
        &mut self,
        state: &mut crate::AgentState,
        result: DelegationResult,
    ) -> Result<NextStep, AgentError> {
        let next_step = self.inner_strategy.process_delegation_result(state, result)?;
        self.update_history_and_check_completion(state, &next_step);
        Ok(next_step)
    }
}

impl<UI: UserInteraction + 'static> ConversationStrategy<UI> {
    fn update_history_and_check_completion(
        &mut self,
        state: &crate::AgentState,
        next_step: &NextStep,
    ) {
        self.conversation_history = state.messages.clone();
        if let NextStep::Completed(_) = next_step {
            self.end_current_task = true;
        }
    }

    pub fn get_history(&self) -> &Vec<ChatMessage> {
        &self.conversation_history
    }
}

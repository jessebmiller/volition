// volition-agent-core/src/strategies/conversation.rs
use super::{DelegationResult, NextStep, Strategy};
use crate::errors::AgentError;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::UserInteraction;
use anyhow::Result;
use async_trait::async_trait;
use std::fmt;

pub struct ConversationStrategy<UI: UserInteraction + 'static> {
    conversation_history: Vec<ChatMessage>,
    inner_strategy: Box<dyn Strategy<UI> + Send + Sync>,
}

// Manual Debug implementation
impl<UI: UserInteraction + 'static> fmt::Debug for ConversationStrategy<UI> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConversationStrategy")
         .field("conversation_history", &self.conversation_history)
         .field("inner_strategy", &self.inner_strategy.name())
         .finish()
    }
}

impl<UI: UserInteraction + 'static> ConversationStrategy<UI> {
    pub fn new(inner_strategy: Box<dyn Strategy<UI> + Send + Sync>) -> Self {
        Self {
            conversation_history: Vec::new(),
            inner_strategy,
        }
    }

    pub fn with_history(
        inner_strategy: Box<dyn Strategy<UI> + Send + Sync>,
        history: Vec<ChatMessage>,
    ) -> Self {
        Self {
            conversation_history: history,
            inner_strategy,
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
        // If conversation_history is not empty, we are continuing a conversation.
        // Replace the state's messages entirely with the stored history.
        if !self.conversation_history.is_empty() {
            state.messages = self.conversation_history.clone();
        } else {
            // If history is empty (first turn), keep the initial message(s)
            // already present in the state (from AgentState::new).
            // We'll update self.conversation_history later in update_history.
        }
        self.inner_strategy.initialize_interaction(state)
    }

    fn process_api_response(
        &mut self,
        state: &mut crate::AgentState,
        response: ApiResponse,
    ) -> Result<NextStep, AgentError> {
        let next_step = self.inner_strategy.process_api_response(state, response)?;
        self.update_history(state);
        Ok(next_step)
    }

    fn process_tool_results(
        &mut self,
        state: &mut crate::AgentState,
        results: Vec<crate::ToolResult>,
    ) -> Result<NextStep, AgentError> {
        let next_step = self.inner_strategy.process_tool_results(state, results)?;
        self.update_history(state);
        Ok(next_step)
    }

    fn process_delegation_result(
        &mut self,
        state: &mut crate::AgentState,
        result: DelegationResult,
    ) -> Result<NextStep, AgentError> {
        let next_step = self.inner_strategy.process_delegation_result(state, result)?;
        self.update_history(state);
        Ok(next_step)
    }
}

impl<UI: UserInteraction + 'static> ConversationStrategy<UI> {
    // Simplified history update
    fn update_history(&mut self, state: &crate::AgentState) {
        self.conversation_history = state.messages.clone();
    }

    pub fn get_history(&self) -> &Vec<ChatMessage> {
        &self.conversation_history
    }
}

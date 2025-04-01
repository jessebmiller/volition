// volition-agent-core/src/strategies/plan_execute.rs
use super::{DelegationResult, NextStep, Strategy, StrategyConfig};
use crate::errors::AgentError;
use crate::models::chat::{ApiResponse, ChatMessage};
use crate::UserInteraction;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::{debug, info, instrument};

#[derive(Debug, PartialEq, Clone, Copy)]
enum PlanExecutePhase {
    Planning,
    Execution,
    Completed,
}

pub struct PlanExecuteStrategy {
    config: StrategyConfig,
    phase: PlanExecutePhase,
    plan: Option<String>,
}

impl PlanExecuteStrategy {
    pub fn new(config: StrategyConfig) -> Self {
        Self {
            config,
            phase: PlanExecutePhase::Planning,
            plan: None,
        }
    }
}

#[async_trait]
impl<UI: UserInteraction + 'static> Strategy<UI> for PlanExecuteStrategy {
    fn name(&self) -> &'static str {
        "PlanExecute"
    }

    #[instrument(skip(self, agent_state), name = "PlanExecute::initialize")]
    fn initialize_interaction(&mut self, agent_state: &mut crate::AgentState) -> Result<NextStep, AgentError> {
        info!(phase = ?self.phase, "Initializing PlanExecute strategy.");
        self.phase = PlanExecutePhase::Planning;
        let _planning_provider = self.config.planning_provider.as_deref()
            .ok_or_else(|| AgentError::Strategy("Missing planning_provider in strategy config".to_string()))?;

        // Get the last user message as the current task, assuming ConversationStrategy placed it there.
        let current_task = agent_state.messages.last()
            .filter(|m| m.role == "user") // Ensure it's a user message
            .and_then(|m| m.content.as_ref())
            .ok_or_else(|| AgentError::Strategy("Current user task message not found in state".to_string()))?;

        let planning_messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some("You are a planning assistant. Create a concise, step-by-step plan to accomplish the user's task. Output ONLY the plan steps.".to_string()),
                ..Default::default()
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(format!("Create a plan for this task: {}", current_task)), // Rephrase slightly
                ..Default::default()
            },
        ];

        // Append planning context instead of overwriting
        agent_state.messages.extend(planning_messages);
        agent_state.pending_tool_calls.clear();
        Ok(NextStep::CallApi(agent_state.clone()))
    }

    #[instrument(skip(self, agent_state, api_response), name = "PlanExecute::process_api")]
    fn process_api_response(
        &mut self,
        agent_state: &mut crate::AgentState,
        api_response: ApiResponse,
    ) -> Result<NextStep, AgentError> {
        info!(phase = ?self.phase, "Processing API response.");
        let response_message = api_response.choices.first()
            .ok_or_else(|| AgentError::Api(anyhow!("API response was empty")))?
            .message.clone();

        agent_state.add_message(response_message.clone());

        match self.phase {
            PlanExecutePhase::Planning => {
                let plan_content = response_message.content
                    .ok_or_else(|| AgentError::Api(anyhow!("Planning response content was empty")))?;
                info!(plan = %plan_content, "Generated plan.");
                self.plan = Some(plan_content.clone());
                self.phase = PlanExecutePhase::Execution;

                let _execution_provider = self.config.execution_provider.as_deref()
                    .ok_or_else(|| AgentError::Strategy("Missing execution_provider in strategy config".to_string()))?;

                let execution_messages = vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: Some("You are an execution assistant. Execute the given plan step-by-step using the available tools (MCP servers). Request tool calls as needed.".to_string()),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: Some(format!("Execute this plan:\n---\n{}\n---", plan_content)),
                        ..Default::default()
                    },
                ];

                // Append execution context instead of overwriting
                agent_state.messages.extend(execution_messages);
                agent_state.pending_tool_calls.clear();
                Ok(NextStep::CallApi(agent_state.clone()))
            }
            PlanExecutePhase::Execution => {
                if let Some(tool_calls) = response_message.tool_calls {
                    debug!(count = tool_calls.len(), "AI requested tool calls.");
                    agent_state.set_tool_calls(tool_calls);
                    Ok(NextStep::CallTools(agent_state.clone()))
                } else {
                    info!("Execution phase completed.");
                    self.phase = PlanExecutePhase::Completed;
                    let final_content = response_message.content.unwrap_or_else(|| "Execution complete.".to_string());
                    Ok(NextStep::Completed(final_content))
                }
            }
            PlanExecutePhase::Completed => {
                Err(AgentError::Strategy("Received API response after completion".to_string()))
            }
        }
    }

    #[instrument(skip(self, agent_state, tool_results), name = "PlanExecute::process_tools")]
    fn process_tool_results(
        &mut self,
        agent_state: &mut crate::AgentState,
        tool_results: Vec<crate::ToolResult>,
    ) -> Result<NextStep, AgentError> {
        info!(phase = ?self.phase, count = tool_results.len(), "Processing tool results.");
        if self.phase != PlanExecutePhase::Execution {
            return Err(AgentError::Strategy("Received tool results outside of execution phase".to_string()));
        }
        agent_state.add_tool_results(tool_results);
        Ok(NextStep::CallApi(agent_state.clone()))
    }

     fn process_delegation_result(
        &mut self,
        _agent_state: &mut crate::AgentState,
        _delegation_result: DelegationResult,
    ) -> Result<NextStep, AgentError> {
        Err(AgentError::Strategy("Delegation not supported by PlanExecuteStrategy".to_string()))
    }
}

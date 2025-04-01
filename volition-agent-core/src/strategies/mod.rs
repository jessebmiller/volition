// volition-agent-core/src/strategies/mod.rs
use crate::errors::AgentError;
use crate::{AgentState, ApiResponse, DelegationResult, ToolResult, UserInteraction};

pub mod complete_task;
// Removed: mod conversation;
pub mod plan_execute;

// Removed: pub use conversation::ConversationStrategy;
pub use plan_execute::PlanExecuteStrategy;
pub use crate::config::StrategyConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyType {
    CompleteTask,
    PlanReviseExecute,
    // Removed: Conversation,
    PlanExecute,
}

#[derive(Debug, Clone)]
pub struct DelegationInput {
    pub task_description: String,
}

#[derive(Debug, Clone)]
pub struct DelegationOutput {
    pub result: String,
}

#[derive(Debug)]
pub enum NextStep {
    CallApi(AgentState),
    CallTools(AgentState),
    DelegateTask(DelegationInput),
    Completed(String),
}

pub trait Strategy<UI: UserInteraction + 'static>: Send + Sync {
    fn name(&self) -> &'static str;

    fn initialize_interaction(&mut self, agent_state: &mut AgentState) -> Result<NextStep, AgentError>;

    fn process_api_response(
        &mut self,
        agent_state: &mut AgentState,
        response: ApiResponse,
    ) -> Result<NextStep, AgentError>;

    fn process_tool_results(
        &mut self,
        agent_state: &mut AgentState,
        results: Vec<ToolResult>,
    ) -> Result<NextStep, AgentError>;

    fn process_delegation_result(
        &mut self,
        agent_state: &mut AgentState,
        result: DelegationResult,
    ) -> Result<NextStep, AgentError>;
}

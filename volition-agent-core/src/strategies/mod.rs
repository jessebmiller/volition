use crate::errors::AgentError;
use crate::{AgentState, ApiResponse, DelegationResult, ToolResult};

pub mod complete_task;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyType {
    CompleteTask,
    PlanReviseExecute, // Conceptual for now
}

#[derive(Debug, Clone)]
pub struct DelegationInput {
    pub task_description: String,
    // Potentially add context, constraints, etc.
}

#[derive(Debug, Clone)]
pub struct DelegationOutput {
    pub result: String,
    // Potentially add artifacts, logs, etc.
}

#[derive(Debug)]
pub enum NextStep {
    CallApi(AgentState),
    CallTools(AgentState),
    DelegateTask(DelegationInput),
    Completed(String),
}

pub trait Strategy: Send + Sync {
    fn name(&self) -> &'static str;

    fn initialize_interaction(&self, state: &mut AgentState) -> Result<NextStep, AgentError>;

    fn process_api_response(
        &self,
        state: &mut AgentState,
        response: ApiResponse,
    ) -> Result<NextStep, AgentError>;

    fn process_tool_results(
        &self,
        state: &mut AgentState,
        results: Vec<ToolResult>,
    ) -> Result<NextStep, AgentError>;

    fn process_delegation_result(
        &self,
        state: &mut AgentState,
        result: DelegationResult,
    ) -> Result<NextStep, AgentError>;
}

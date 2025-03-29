# Agent Strategy Abstraction Plan

## Goal

Refactor the agent's core logic to use a pluggable `Strategy`
pattern. This will allow different approaches to task execution (e.g.,
simple request-response, plan-revise-execute) without modifying the
core agent orchestration loop.

## Core Concepts

*   **`StrategyType` Enum:** Identifies specific strategy implementations (e.g., `CompleteTask`, `PlanReviseExecute`).
*   **`DelegationInput` Struct:** Data needed to start a delegated strategy (e.g., `initial_messages: Vec<ChatMessage>`, `goal: Option<String>`).
*   **`DelegationOutput` Struct:** Results returned by a completed delegated strategy (e.g., `final_messages: Vec<ChatMessage>`, `final_result: Option<String>`).

## Core Architecture

1.  **`Strategy` Trait:** Defines the interface for different agent strategies. It's responsible for deciding the next step based on the current state and the results of the previous step (API response, tool execution, or delegation result).
    *   Location: `volition-agent-core/src/strategies/mod.rs`
    *   Key Methods:
        *   `initialize_interaction(&mut self, /* context */) -> Result<Vec<ChatMessage>>`: Provides the initial messages for the first API call. Takes `&mut self` to allow state initialization.
        *   `process_api_response(&mut self, messages: &[ChatMessage], response: &ApiResponse) -> Result<NextStep>`: Processes the latest API response (including potential tool calls) and decides what to do next.
        *   `process_tool_results(&mut self, messages: &mut Vec<ChatMessage>, tool_results: Vec<ToolResult>) -> Result<NextStep>`: Processes the results of executed tools and decides what to do next. Can modify the message history directly.
        *   `process_delegation_result(&mut self, output: DelegationOutput) -> Result<NextStep>`: Processes the results received after a delegated strategy completes and decides the next step for the current strategy.

2.  **`NextStep` Enum:** Represents the possible outcomes a `Strategy` can decide upon, instructing the orchestrator what action to perform next.
    *   Location: `volition-agent-core/src/strategies/mod.rs`
    *   Variants:
        *   `GetChatCompletion(Vec<ChatMessage>)`: Instructs the orchestrator to call the API with the provided messages.
        *   `ExecuteTools(Vec<ToolCall>)`: Instructs the orchestrator to execute the specific tool calls *requested* by the API in the previous response.
        *   `Delegate { strategy_type: StrategyType, input: DelegationInput }`: Instructs the orchestrator to run a sub-strategy.
        *   `Complete { final_messages: Vec<ChatMessage>, final_result: Option<String> }`: Indicates the interaction (or delegated sub-task) is complete and provides the final state and result.
        *   `ReportError(anyhow::Error)`: Signals an unrecoverable error.

3.  **`Agent` Struct (Orchestrator):**
    *   Holds the top-level `strategy: Box<dyn Strategy + Send + Sync>`.
    *   The `Agent::run` method orchestrates the interaction flow:
        1. Calls `strategy.initialize_interaction` to get initial messages.
        2. Enters a main loop processing `NextStep` results:
            *   `GetChatCompletion`: Calls API, appends response, calls `strategy.process_api_response`.
            *   `ExecuteTools`: Executes tools, appends results, calls `strategy.process_tool_results`.
            *   `Delegate`:
                a. Stores the current strategy state (conceptually).
                b. Creates an instance of the specified `sub_strategy`.
                c. Runs the `sub_strategy` using its `initialize_interaction`, `process_api_response`, `process_tool_results` methods and handling *its* `NextStep` results (potentially recursively delegating).
                d. When the `sub_strategy` returns `NextStep::Complete { ... }`: Extracts the `DelegationOutput` (final messages, result). Note: Need to clearly define how output is extracted/structured.
                e. Calls `original_strategy.process_delegation_result(output)` on the stored strategy to continue its flow.
            *   `Complete`: Exits the main loop, returns the final result.
            *   `ReportError`: Returns the error.

## Strategy Implementations

### 1. `CompleteTaskStrategy` (Formerly `DefaultStrategy`)

*   **Purpose:** Handles a task by iteratively calling the API and executing tools until a final answer is reached. Can be used as the top-level strategy or delegated to.
*   **Logic:**
    *   `initialize_interaction`: Creates standard system prompt + user message based on input.
    *   `process_api_response`: Checks the API response. If `tool_calls` are present, returns `NextStep::ExecuteTools(calls)`. Otherwise, returns `NextStep::Complete { final_messages, final_result: content }`.
    *   `process_tool_results`: Formats tool results into messages, appends them, and returns `NextStep::GetChatCompletion(updated_messages)` to send results back to the API.
    *   `process_delegation_result`: Likely returns an error, as this strategy doesn't delegate.

### 2. `PlanReviseExecuteStrategy` (Conceptual)

*   **Purpose:** Implements a more complex flow involving planning, evaluation, execution (potentially delegated), and potential revision.
*   **Internal State:** Maintains its current phase (e.g., `NeedsPlan`, `EvaluatingPlan`, `ExecutingStep`, `RevisingPlan`) and the plan itself.
*   **Tools:** Relies on the API using specific tools provided to it (e.g., `submit_plan`, `submit_evaluation`).
*   **Logic:**
    *   `initialize_interaction`: Asks the API to create a plan. Sets state to `NeedsPlan`. Returns `NextStep::GetChatCompletion(...)`.
    *   `process_api_response`: Handles responses based on state (e.g., receiving the plan via `submit_plan` tool call). Returns `NextStep::ExecuteTools` to confirm tool use or `NextStep::GetChatCompletion` for evaluation/revision prompts.
    *   `process_tool_results`: Processes results of `submit_plan` or `submit_evaluation`. Transitions state (e.g., `NeedsPlan` -> `EvaluatingPlan`, `EvaluatingPlan` -> `ExecutingStep` or `RevisingPlan`). If transitioning to `ExecutingStep`:
        *   Determines the goal and context for the first step.
        *   Creates `DelegationInput` for the step.
        *   Returns `NextStep::Delegate { strategy_type: StrategyType::CompleteTask, input }`.
    *   `process_delegation_result`: Called when a delegated `CompleteTaskStrategy` finishes an execution step.
        Question: If the strategy is not waiting for a delegation result should it return an error?
        *   Receives `DelegationOutput` (messages, result of the step).
        *   Updates internal state (marks step complete, stores result).
        *   Decides next action:
            *   Prepare next step and return `NextStep::Delegate { ... }`.
            *   If plan complete, return `NextStep::Complete { ... }`.
            *   If step failed/needs revision, transition state to `RevisingPlan` and return `NextStep::GetChatCompletion(...)` for revision prompt.

This architecture separates the interaction mechanics (orchestrator) from the decision-making logic (strategy), allowing for flexible and complex agent behaviors, including strategy composition via delegation.

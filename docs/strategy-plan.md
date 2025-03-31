# Agent Strategy Abstraction Plan

## Goal

Refactor the agent's core logic to use a pluggable `Strategy` pattern. This will allow different approaches to task execution (e.g., simple request-response, plan-revise-execute) without modifying the core agent orchestration loop.

## Core Architecture

1.  **`Strategy` Trait:** Defines the interface for different agent strategies. It's responsible for deciding the next step based on the current state and the results of the previous step (API response or tool execution).
    *   Location: `volition-agent-core/src/strategies/mod.rs`
    *   Key Methods:
        *   `initialize_interaction(...) -> Vec<ChatMessage>`: Provides the initial messages for the first API call.
        *   `process_api_response(...) -> Result<NextStep>`: Processes the latest API response (including potential tool calls) and decides what to do next.
        *   `process_tool_results(...) -> Result<NextStep>`: Processes the results of executed tools and decides what to do next.

2.  **`NextStep` Enum:** Represents the possible outcomes a `Strategy` can decide upon.
    *   Location: `volition-agent-core/src/strategies/mod.rs`
    *   Variants:
        *   `QueryApi(Vec<ChatMessage>)`: Instructs the orchestrator to call the API with the provided messages.
        *   `ExecuteTools(Vec<ToolCall>)`: Instructs the orchestrator to execute the specific tool calls *requested* by the API in the previous response.
        *   `Complete(Option<String>)`: Indicates the interaction is complete and provides the final assistant message content.
        *   `ReportError(anyhow::Error)`: Signals an unrecoverable error.

3.  **`Agent` Struct (Orchestrator):**
    *   Holds a `strategy: Box<dyn Strategy + Send + Sync>`.\
    *   The `Agent::run` method orchestrates the interaction flow:
        1. Calls `strategy.initialize_interaction` to get initial messages.
        2. Enters a loop:
            a. Calls the API with the current message history.
            b. Appends the assistant response to history.
            c. Calls `strategy.process_api_response`.
            d. Based on the `NextStep`:
                *   `Complete`: Exits loop, returns result.
                *   `ReportError`: Returns error.
                *   `ExecuteTools`: Executes the requested tools, appends results to history, calls `strategy.process_tool_results`.
                *   `QueryApi` (from `process_tool_results`): Continues the loop for the next API call.
                *   (Other decisions handled appropriately based on context).

## Strategy Implementations

### 1. `DefaultStrategy`

*   **Purpose:** Mimics the original, direct interaction flow.
*   **Logic:**
    *   `initialize_interaction`: Creates standard system prompt + user message.
    *   `process_api_response`: Checks the API response. If `tool_calls` are present, returns `ExecuteTools(calls)`. Otherwise, returns `Complete(content)`.
    *   `process_tool_results`: Formats tool results into messages, appends them to history, and returns `QueryApi(updated_history)` to send results back to the API.

### 2. `PlanReviseExecuteStrategy` (Conceptual)

*   **Purpose:** Implements a more complex flow involving planning, evaluation, execution, and potential revision.
*   **Internal State:** Maintains its current phase (e.g., `NeedsPlan`, `EvaluatingPlan`, `ExecutingStep`, `RevisingPlan`).
*   **Tools:** Relies on the API using specific tools provided to it, such as:
    *   `submit_plan(plan: String)`: Used by the API to provide the generated plan.
    *   `submit_evaluation(score: f64, reasoning: String)`: Used by the API to evaluate a plan.
*   **Logic:**
    *   `initialize_interaction`: Asks the API to create a plan for the user's request and use `submit_plan`. Sets state to `NeedsPlan`.
    *   `process_api_response`:
        *   If state is `NeedsPlan` and `submit_plan` is called: Stores plan, returns `ExecuteTools(submit_plan_call)`.
        *   If state is `EvaluatingPlan` and `submit_evaluation` is called: Stores evaluation, returns `ExecuteTools(submit_evaluation_call)`.
        *   If state is `ExecutingStep`: Checks for work-related tool calls (e.g., `read_file`) or text indicating step completion. Returns `ExecuteTools(work_calls)` or `QueryApi(prompt_for_next_step)`.
        *   Handles other states and unexpected API responses appropriately.
    *   `process_tool_results`:
        *   If state was `NeedsPlan` (after `submit_plan`): Prepares evaluation prompt, provides `submit_evaluation` tool, sets state to `EvaluatingPlan`, returns `QueryApi(eval_prompt)`.
        *   If state was `EvaluatingPlan` (after `submit_evaluation`): Checks score. If good, sets state to `ExecutingStep(0)`, prepares prompt for first step execution. If bad, sets state to `RevisingPlan`, prepares revision prompt. Returns `QueryApi(...)`.
        *   If state was `ExecutingStep` (after work tool): Prepares prompt including tool results, asking API for next action/tool call. Returns `QueryApi(...)`.
        *   Handles other state transitions.

This architecture separates the interaction mechanics (orchestrator) from the decision-making logic (strategy), allowing for flexible and complex agent behaviors.

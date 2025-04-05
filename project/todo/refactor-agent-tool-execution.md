# Task: Extract Agent Tool Execution Logic

**Context:**
The logic within the `NextStep::CallTools` arm of the `match` statement in `Agent::run()` (or the proposed `handle_call_tools` helper function) is responsible for iterating through pending tool calls, mapping tool names to MCP servers, calling `call_mcp_tool`, handling errors, formatting results, and logging. This logic is complex and long (~80 lines).

**Goal:**
Extract the tool execution loop into a dedicated, well-defined function to simplify the `CallTools` handling logic and improve testability/modularity.

**Proposed Steps:**
1.  Identify the complete block of code responsible for executing tool calls within the `NextStep::CallTools` arm (or the `handle_call_tools` helper function).
2.  Create a new private async function within `impl<UI: UserInteraction + 'static> Agent<UI>`: `async fn execute_mcp_tool_calls(&self, tool_calls: &[crate::models::chat::ToolCall]) -> Result<Vec<crate::ToolResult>, AgentError>`.
3.  Move the tool execution loop logic into this new function. This includes:
    *   Iterating through `tool_calls`.
    *   Parsing arguments (`serde_json::from_str`).
    *   Mapping the tool name to an `server_id` (hardcoded mapping logic).
    *   Calling `self.call_mcp_tool(server_id, tool_name, args).await`.
    *   Handling the `Ok`/`Err` result from the tool call.
    *   Formatting the output `Value` into a string for the `ToolResult`.
    *   Creating and collecting `crate::ToolResult` structs.
    *   Logging individual tool execution start/results (the `println!` calls).
4.  Update the `NextStep::CallTools` arm (or `handle_call_tools` helper) to:
    *   Get the `tool_calls_to_execute` from `self.state.pending_tool_calls`.
    *   Call `self.execute_mcp_tool_calls(&tool_calls_to_execute).await`.
    *   Pass the returned `Vec<crate::ToolResult>` to `self.strategy.process_tool_results`.
5.  Ensure the new function has access to `self` to call `call_mcp_tool`.
6.  Run `cargo check`, `cargo clippy`, and `cargo test` within `volition-core`.

**Affected Files:**
*   `volition-core/src/agent.rs`

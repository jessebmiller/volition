# Task: Refactor Agent Run Method

**Context:**
The `Agent::run()` method in `volition-core/src/agent.rs` contains the main interaction loop, orchestrated by a large `match next_step { ... }` block. The logic within each arm of the match (`CallApi`, `CallTools`, `DelegateTask`, `Completed`) is substantial, making the `run` method long (~150 lines) and less readable.

**Goal:**
Improve the readability and maintainability of `Agent::run()` by extracting the logic for handling each `NextStep` variant into separate private helper functions.

**Proposed Steps:**
1.  Identify the distinct blocks of logic within each arm of the `match next_step` statement in `Agent::run()`.
2.  Create private helper functions within `impl<UI: UserInteraction + 'static> Agent<UI>` for each significant arm:
    *   `async fn handle_call_api(&mut self) -> Result<NextStep, AgentError>`: Contains the logic currently in the `NextStep::CallApi` arm (listing MCP tools, formatting definitions, calling `get_completion`, calling `strategy.process_api_response`).
    *   `async fn handle_call_tools(&mut self) -> Result<NextStep, AgentError>`: Contains the logic currently in the `NextStep::CallTools` arm (getting pending calls, executing them (potentially calling another helper - see separate TODO), processing results with `strategy.process_tool_results`).
    *   `async fn handle_delegate_task(&mut self, delegation_input: DelegationInput) -> Result<NextStep, AgentError>`: Contains the logic for `NextStep::DelegateTask`.
3.  Modify the `Agent::run()` method's loop to call these helper functions based on the `next_step` value received from the strategy.
    *   The loop will become much simpler, primarily focused on getting `next_step` from the strategy and dispatching to the appropriate `handle_...` function.
4.  Ensure state (`self.state`) is passed correctly to and updated by the helper functions as needed (likely by taking `&mut self`).
5.  Run `cargo check`, `cargo clippy`, and `cargo test` within `volition-core`.

**Affected Files:**
*   `volition-core/src/agent.rs`

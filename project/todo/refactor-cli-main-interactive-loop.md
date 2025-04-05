# Task: Refactor CLI Interactive Loop

**Context:**
The `run_interactive` function in `volition-cli/src/main.rs` handles the entire interactive REPL session and is quite long (~120 lines). It includes rustyline setup, the main input loop, command parsing (`exit`, `new`), agent execution, result handling, and history saving.

**Goal:**
Break down `run_interactive` into smaller, more focused functions to improve readability and maintainability.

**Proposed Steps:**
1.  **Extract Rustyline Setup:** Create a helper function `setup_rustyline() -> Result<DefaultEditor>` that encapsulates the `Config::builder()` logic and history file loading/saving setup.
2.  **Extract Agent Turn Logic:** The core block within the loop (~40-50 lines) that takes user input, creates the agent, manages the spinner, runs the agent, handles the `Ok`/`Err` result, prints output, and saves history is very similar to `run_single_turn`. Create a function like `execute_agent_turn(user_input: String, history: &mut ConversationHistory, config: &AgentConfig, project_root: &Path, ui_handler: Arc<CliUserInteraction>) -> Result<()>`.
    *   This function would handle creating the agent, running it, processing the output (printing/formatting), and updating/saving the history.
    *   `run_interactive` would call this function within the loop.
    *   Consider if `run_single_turn` can also be refactored to use this common function.
3.  **Refine Loop Structure:** After extraction, the main loop in `run_interactive` should primarily handle:
    *   Calling `rl.readline()`.
    *   Parsing top-level commands (`exit`, `quit`, `new`).
    *   Calling `execute_agent_turn` for regular input.
    *   Handling `ReadlineError` variants (`Interrupted`, `Eof`).
4.  Move the extracted functions within `main.rs` or potentially to a new `cli/src/interactive.rs` module if they become numerous.
5.  Run `cargo check` and `cargo test` within `volition-cli`.

**Affected Files:**
*   `volition-cli/src/main.rs`
*   Potentially `volition-cli/src/interactive.rs` (new)

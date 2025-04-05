# Task: Move CLI History Command Handlers

**Context:**
The functions `handle_list_conversations`, `handle_view_conversation`, and `handle_delete_conversation` are currently defined within `volition-cli/src/main.rs`. They implement the logic for the `list`, `view`, and `delete` subcommands, primarily interacting with the history functions defined in `cli/src/history.rs`.

**Goal:**
Move these command handler functions out of `main.rs` to improve organization and separation of concerns.

**Proposed Steps:**
1.  Identify the handler functions: `handle_list_conversations`, `handle_view_conversation`, `handle_delete_conversation` in `main.rs`.
2.  Move these functions into the existing `volition-cli/src/history.rs` module, as they directly operate on conversation history.
    *   Alternatively, create a new module like `volition-cli/src/commands.rs` or `volition-cli/src/handlers.rs` if more command handlers are expected in the future.
3.  Make the moved functions public (`pub fn ...`).
4.  Update `main.rs` to call the functions from their new location (e.g., `history::handle_list_conversations(...)`).
5.  Ensure all necessary imports (`uuid`, `anyhow`, `colored`, `chrono`, `dialoguer`, etc., and functions from `history.rs` itself like `list_histories`, `load_history`, `delete_history`, `get_history_preview`) are present in `history.rs` (or the new module).
6.  Update `volition-cli/src/history.rs` (or the new module file) to declare necessary `use` statements.
7.  Run `cargo check` and `cargo test` within `volition-cli`.

**Affected Files:**
*   `volition-cli/src/main.rs`
*   `volition-cli/src/history.rs` (or a new command/handler module)

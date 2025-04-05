# Task: Consolidate CLI UI Helper Logic

**Context:**
The `volition-cli/src/main.rs` file contains UI-related helper logic scattered in different places:
*   `print_welcome_message` function.
*   Duplicated spinner setup and management code (using `indicatif`) within `run_single_turn` and `run_interactive`.

**Goal:**
Consolidate these UI elements to reduce duplication and improve organization.

**Proposed Steps:**
1.  **Welcome Message:** Move the `print_welcome_message` function to the `volition-cli/src/rendering.rs` module (or a dedicated `ui.rs` if preferred).
    *   Make it public (`pub fn ...`).
    *   Update call sites in `main.rs`.
2.  **Spinner Logic:** Create a helper function or struct to manage the spinner.
    *   **Option A (Function):** Create a function like `async fn run_with_spinner<F, T>(message: &str, future: F) -> Result<T>` where `F: Future<Output = Result<T>>`. This function would setup/start the spinner, await the future, stop the spinner, and return the result.
    *   **Option B (Struct):** Create a struct `Spinner { pb: ProgressBar }` with methods like `start(&str)`, `finish()`, `set_message(&str)`. The calling code would manage the spinner lifecycle around the async operation.
    *   Place this helper in `rendering.rs` or a new `ui.rs` module.
3.  Update `run_single_turn` and `run_interactive` to use the new spinner helper, removing the duplicated `ProgressBar::new_spinner()...` code.
4.  Ensure necessary imports (`indicatif`, `colored`, `uuid`) are handled correctly in the new locations.
5.  Run `cargo check` and `cargo test` within `volition-cli`.

**Affected Files:**
*   `volition-cli/src/main.rs`
*   `volition-cli/src/rendering.rs` (or a new `ui.rs` module)

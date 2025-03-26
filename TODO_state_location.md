# TODO: State File Location

## Issue: Unsuitable State File Path

The conversation recovery state is currently saved to `.conversation_state.json` in the application's current working directory (defined by `RECOVERY_FILE_PATH` in `src/main.rs`).

This location is problematic because:
*   The current directory might be temporary or read-only.
*   It can clash with source code files if run from the project root.
*   It's not a standard or predictable location for application state.

## Recommendation

Modify `src/main.rs` to save the state file to a more appropriate and robust location. Options include:

1.  **User-Specific Directory (Recommended):**
    *   Use a crate like `directories` or `dirs` to find the user's home directory or application data directory.
    *   Create a dedicated hidden subdirectory (e.g., `~/.volition/` or `%APPDATA%\Volition\`).
    *   Store the state file within this directory (e.g., `~/.volition/conversation_state.json`).

2.  **Configurable Directory:**
    *   Add a configuration option (e.g., `state_directory` in `config.toml`) to specify where state files should be stored.
    *   Fall back to a default location (like option 1) if not specified.

**Implementation Notes:**
*   Ensure the chosen directory is created if it doesn't exist when attempting to save the state.
*   Update the logic in `src/main.rs` that reads, writes, and deletes the state file to use the new path determination logic.

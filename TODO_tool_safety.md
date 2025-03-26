# TODO: Tool Safety Improvements

## Issue: Lack of User Confirmation

The `shell` and `write_file` tools currently execute potentially destructive actions (running arbitrary shell commands, writing/overwriting files) without asking the user for confirmation. This is a critical safety risk.

## Recommendation

Modify the following files to incorporate a user confirmation step using the `user_input` tool before proceeding with the action:

1.  **`src/tools/shell.rs`**:
    *   Before executing the command using `std::process::Command`.
    *   Prompt the user with the command to be executed and ask for confirmation (Yes/No).
    *   Only proceed if the user explicitly confirms.

2.  **`src/tools/file.rs` (specifically the `write_file` function)**:
    *   Before calling `fs::write`.
    *   Prompt the user with the target file path and potentially a summary of the content (or its size) and ask for confirmation (Yes/No).
    *   Only proceed if the user explicitly confirms. Consider adding an extra warning if the file already exists.

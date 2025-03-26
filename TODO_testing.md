# TODO: Implement Testing for Production Readiness

The application currently lacks automated tests, which is critical for production stability and maintainability.

## High Priority Recommendations:

1.  **Implement Unit Tests:**
    *   **`config.rs`:**
        *   Test `load_config()` with valid `config.toml` and `API_KEY` environment variable.
        *   Test `load_config()` when `config.toml` is missing or invalid.
        *   Test `load_config()` when `API_KEY` is missing or empty.
        *   Test `load_config()` validation logic (mismatched service/model, missing model).
        *   Test `get_config_path()` logic (mocking `dirs::home_dir` if necessary).
    *   **`api.rs`:**
        *   Test `build_openai_request()` for correct JSON structure, including parameters and tool inclusion/exclusion based on service.
        *   Consider mocking `reqwest::Client` to test `chat_with_endpoint` and `chat_with_api` logic (URL selection, header creation, retry logic, response parsing) without making real API calls. Test different service types (`openai`, `ollama`) and endpoint overrides.
    *   **`tools.rs`:**
        *   Test argument parsing for each tool function (e.g., `parse_shell_args`, `parse_read_file_args`).
        *   Test the core logic of each tool function where possible (mocking filesystem or shell interactions).
    *   **`models/*.rs`:**
        *   Add tests for any custom logic or validation within data models.

2.  **Implement Integration Tests (in `tests/` directory):**
    *   **Basic Conversation Flow:** Test the `handle_conversation` loop with mocked API responses (including tool calls and user input).
    *   **State Recovery:**
        *   Test saving state to `.conversation_state.json`.
        *   Test resuming a conversation from an existing state file.
        *   Test handling of corrupted or invalid state files.
        *   Test user opting out of resuming.
    *   **Tool Execution Flow:** Test the end-to-end flow of receiving a tool call, parsing arguments, executing the (mocked) tool, and sending the result back to the API.
    *   **CLI Argument Parsing:** Test `Cli::parse()` with various valid and invalid command-line arguments.

## Tools & Techniques:

*   Use Rust's built-in testing framework (`#[test]`).
*   Use mocking libraries like `mockall` or `reqwest_mock` for external dependencies (API calls, filesystem).
*   Use `assert_cmd` and `predicates` crates for testing the compiled binary's behavior via the command line.
*   Organize tests clearly within modules (`#[cfg(test)] mod tests { ... }`) or in the `tests/` directory.

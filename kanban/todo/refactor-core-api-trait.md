# Task: Refactor Core API Interaction with Trait-Based Strategy

**Context:**
The `volition-core/src/api.rs` file, particularly the `call_chat_completion_api` function, currently handles the logic for interacting with multiple distinct AI chat completion APIs (Google Gemini and OpenAI-compatible) within a single large function. This involves conditional logic (`if is_google_api { ... } else { ... }`) for payload construction, authentication, parameter mapping, and response parsing, making the function long (600+ lines) and difficult to maintain or extend for new API providers.

**Goal:**
Refactor the API interaction logic using a trait-based approach to separate the implementation details for each API provider. This will improve modularity, testability, maintainability, and make it easier to add support for new APIs in the future.

**Proposed Steps:**

1.  **Define the Trait:**
    *   Create a new trait, e.g., `ChatApiProvider`, within the `volition-core/src/api/` directory (create the directory and `mod.rs` if they don't exist).
    *   Define the necessary methods for the trait. This might include:
        *   `fn build_payload(&self, model_name: &str, messages: Vec<ChatMessage>, tools: Option<&[ToolDefinition]>, parameters: Option<&toml::Value>) -> Result<serde_json::Value>;`
        *   `fn parse_response(&self, response_body: &str) -> Result<ApiResponse>;`
        *   Optional: `fn build_headers(&self, api_key: &str) -> Result<reqwest::header::HeaderMap>;` (or handle headers within the main function)
        *   Optional: `fn adapt_endpoint(&self, endpoint: Url, api_key: &str) -> Result<Url>;` (for handling things like API key in query params)

2.  **Create Provider Implementations:**
    *   Create separate modules/files for each provider, e.g., `volition-core/src/api/gemini.rs` and `volition-core/src/api/openai.rs`.
    *   Define structs within these modules (e.g., `GeminiProvider`, `OpenAIProvider`).
    *   Implement the `ChatApiProvider` trait for each struct.
    *   Move the specific payload construction logic from `call_chat_completion_api` into the `build_payload` implementation for each provider.
    *   Move the specific response parsing logic (including error handling specific to the provider, like Gemini's block reasons) into the `parse_response` implementation for each provider.

3.  **Refactor `call_chat_completion_api`:**
    *   Modify the function signature or create a new top-level function if needed.
    *   Keep the logic for parsing the endpoint URL and detecting the API type (e.g., based on host).
    *   Based on the detected type, instantiate or select the appropriate `ChatApiProvider` implementation (e.g., `let provider: Box<dyn ChatApiProvider> = Box::new(GeminiProvider);`).
    *   Use the trait methods:
        *   Call `provider.build_payload(...)` to get the request body.
        *   Keep the common `reqwest` HTTP client logic for sending the request (potentially using headers/endpoint modifications from the trait methods if added).
        *   Handle common HTTP errors (connection issues, non-success status codes before parsing).
        *   If the request is successful, call `provider.parse_response(...)` with the response body text.
    *   The main function becomes much shorter, acting primarily as an orchestrator.

4.  **Update Module Structure:**
    *   Ensure `volition-core/src/lib.rs` declares the `api` module (`pub mod api;`).
    *   Ensure `volition-core/src/api/mod.rs` declares the provider modules (`mod gemini; mod openai; pub trait ChatApiProvider; ...`).
    *   Update `use` statements throughout the affected files.

5.  **Testing:**
    *   Run `cargo check`, `cargo clippy`, and `cargo test` within `volition-core`.
    *   Consider adding unit tests for the individual provider implementations if possible (mocking might be needed).

**Affected Files:**
*   `volition-core/src/api.rs` (will be significantly refactored, potentially renamed or replaced by `api/mod.rs`)
*   `volition-core/src/api/mod.rs` (new or modified)
*   `volition-core/src/api/gemini.rs` (new)
*   `volition-core/src/api/openai.rs` (new)
*   `volition-core/src/lib.rs` (module declaration)
*   Any files that directly call `call_chat_completion_api` might need minor updates depending on signature changes.


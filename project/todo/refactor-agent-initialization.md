# Task: Extract Agent Initialization Logic

**Context:**
The `Agent::new()` function in `volition-core/src/agent.rs` initializes several components based on the provided `AgentConfig`, including the `ProviderRegistry` and the `mcp_connections` HashMap. The logic for setting up these components involves iterating through config sections, reading environment variables (for providers), and constructing instances, making `Agent::new()` quite lengthy.

**Goal:**
Simplify the `Agent::new()` function by extracting the initialization logic for the provider registry and MCP connections into separate helper functions.

**Proposed Steps:**
1.  **Extract Provider Registry Initialization:**
    *   Identify the block of code within `Agent::new()` that creates and populates the `ProviderRegistry`.
    *   Create a new private function: `fn initialize_provider_registry(config: &AgentConfig, http_client: &reqwest::Client) -> Result<ProviderRegistry>`.
    *   Move the provider registry creation, iteration over `config.providers`, environment variable reading, provider instantiation (e.g., `GeminiProvider::new`, `OllamaProvider::new`), and registration logic into this function.
    *   Update `Agent::new()` to call this helper function.
2.  **Extract MCP Connections Initialization:**
    *   Identify the block of code within `Agent::new()` that creates and populates the `mcp_connections` HashMap.
    *   Create a new private function: `fn initialize_mcp_connections(config: &AgentConfig) -> Result<HashMap<String, Arc<Mutex<McpConnection>>>>`.
    *   Move the HashMap creation, iteration over `config.mcp_servers`, `McpConnection::new` calls, and insertion logic into this function.
    *   Update `Agent::new()` to call this helper function.
3.  Ensure the helper functions are defined appropriately (e.g., as free functions within `agent.rs` or private methods if they need access to `self`, although in this case they likely don't).
4.  Ensure all necessary imports are available to the helper functions.
5.  Run `cargo check`, `cargo clippy`, and `cargo test` within `volition-core`.

**Affected Files:**
*   `volition-core/src/agent.rs`

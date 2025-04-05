# Task: Refactor Agent Test Setup

**Context:**
The test functions within `volition-core/src/agent_tests.rs` (e.g., `test_agent_initialization`, `test_conversation_history_persistence`) repeat similar setup logic for creating mock components (`MockUI`, `MockToolProvider`), `ProviderRegistry`, `AgentConfig`, and finally the `Agent` instance itself.

**Goal:**
Reduce boilerplate code and improve maintainability within the test functions by extracting the common agent setup logic into a reusable helper function.

**Proposed Steps:**

1.  **Identify Common Setup:** Analyze the setup steps performed at the beginning of each test function in `agent_tests.rs`.
2.  **Create Helper Function:**
    *   Define a new helper function within the test module, e.g.:
      ```rust
      async fn setup_test_agent(
          initial_task: String,
          history: Option<Vec<ChatMessage>>,
          mock_provider: Arc<MockToolProvider>, // Reuse provider if needed across turns
          mock_ui: Arc<MockUI> // Reuse UI if needed
      ) -> Result<Agent<MockUI>, AgentError> {
          // Logic to create default_provider_id, ProviderRegistry, 
          // AgentConfig, mcp_connections (empty HashMap), etc.
          let default_provider_id = format!("mock-provider-{}", generate_id(""));
          let mut registry = ProviderRegistry::new(default_provider_id.clone());
          registry.register(default_provider_id.clone(), Box::new(mock_provider.as_ref().clone()));
          let config = create_minimal_agent_config(default_provider_id);
          let mcp_connections = HashMap::new();
          let strategy = Box::new(CompleteTaskStrategy::default()); // Or make strategy configurable

          Agent::new(
              config,
              mock_ui,
              strategy,
              history,
              initial_task,
              Some(registry),
              Some(mcp_connections),
          )
          .map_err(|e| AgentError::Config(e.to_string()))
      }
      ```
    *   Adjust parameters as needed (e.g., if the strategy needs to be varied).
3.  **Update Test Functions:** Modify existing test functions to call the `setup_test_agent` helper function instead of repeating the setup logic inline.
4.  **Testing:** Run `cargo test` within `volition-core` to ensure all tests still pass after refactoring.

**Benefits:**
*   Reduces code duplication in test setup.
*   Makes test functions shorter and easier to read, focusing on the specific scenario and assertions.
*   Improves maintainability; changes to the agent setup process only need to be made in the helper function.

**Affected Files:**
*   `volition-core/src/agent_tests.rs`

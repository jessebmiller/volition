Refined Plan Document (v4 - Generic Output):

# Refactoring Plan: Extracting `volition-agent-core` Library

This document outlines the plan to refactor the `volition` CLI tool into a Cargo workspace, extracting a reusable `volition-agent-core` library.

**Goal:** Create a library (`volition-agent-core`) containing the core AI interaction, configuration handling, and tool execution interface, making it usable by different frontends (`volition-cli`, `pr-machine`, etc.).

**Core Principles:**

*   **Separation of Concerns:** The core library handles AI communication and orchestrates tool use but does not implement specific tools or UI.
*   **Flexibility:** Consumers provide tool implementations via a `ToolProvider` trait.
*   **Testability:** The core library can be tested independently using mock tool providers.
*   **Generic Output:** The core library returns a general summary of its execution and tool results, allowing consumers to interpret this into specific actions or formats (like commits or PR descriptions).

## Phase 1: Project Setup (Workspace)

1.  **Convert to Workspace:** Modify the root `Cargo.toml` to define a workspace with members `volition-cli` and `volition-agent-core`. Define shared dependencies under `[workspace.dependencies]`.
2.  **Create `volition-cli`:**
    *   Create `volition-cli/` directory.
    *   Move existing `src/` into `volition-cli/src/`.
    *   Move existing `Cargo.toml` into `volition-cli/Cargo.toml`.
    *   Update `volition-cli/Cargo.toml`:
        *   Set `name = "volition-cli"`.
        *   Ensure `[[bin]] name = "volition"`.
        *   Add path dependency: `volition-agent-core = { path = "../volition-agent-core" }`.
        *   Update dependencies to use `workspace = true`.
3.  **Create `volition-agent-core`:**
    *   Create `volition-agent-core/` directory.
    *   Create `volition-agent-core/src/lib.rs`.
    *   Create `volition-agent-core/Cargo.toml`:
        *   Set `name = "volition-agent-core"`.
        *   Define it as a `[lib]` crate.
        *   Add necessary dependencies using `workspace = true`. Include `async-trait`.

## Phase 2: Move Core Logic & Define Tooling Interfaces

1.  **Relocate Core Code:** Move non-CLI-specific modules and types (e.g., `api.rs`, AI interaction logic, non-CLI `models/`, core configuration structs) from `volition-cli/src/` to `volition-agent-core/src/`.
2.  **Define Tool Interfaces (in `volition-agent-core`):**
    *   `models::tools::{ToolDefinition, ToolInput}`: Structs/enums defining tool schemas for the AI and runtime input arguments.
    *   `trait ToolProvider`: Define the `#[async_trait::async_trait] pub trait ToolProvider: Send + Sync` with methods:
        *   `fn get_tool_definitions(&self) -> Vec<ToolDefinition>;`
        *   `async fn execute_tool(&self, tool_name: &str, input: ToolInput, working_dir: &Path) -> Result<String>;` (Returns tool output string).
3.  **Keep Implementations:** Tool implementation logic (using `duct`, `fs`, `reqwest`, interactive `user_input`) remains in `volition-cli/src/tools/` for now.

## Phase 3: Define Core Agent API

1.  **Define Core Structs (in `volition-agent-core`):**
    *   `AgentConfig`: Holds configuration (API keys, model names).
    *   `AgentOutput`: Represents the generic result of an agent run. Contains AI summary, tool execution results, etc. (Replaces `ProposedChange`).
        ```rust
        // Example structure
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        pub struct AgentOutput {
            pub suggested_summary: Option<String>,
            pub applied_tool_results: Vec<ToolExecutionResult>,
            pub final_state_description: Option<String>, // e.g., Final AI message
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct ToolExecutionResult {
           pub tool_name: String,
           // Consider storing input in a structured way, ToolInput might need Serialize/Deserialize
           pub input: serde_json::Value,
           pub output: String,
           pub status: ToolExecutionStatus, // e.g., Success, Failure
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub enum ToolExecutionStatus { Success, Failure }
        ```
    *   `Agent`: The main entry point.
2.  **Implement `Agent`:**
    *   `Agent::new(config: AgentConfig, tool_provider: Arc<dyn ToolProvider>) -> Result<Self>`.
    *   Primary method (e.g., `agent.run(goal: &str, working_dir: &Path) -> Result<AgentOutput>`) orchestrates the process:
        *   Gets tool definitions via `tool_provider`.
        *   Communicates with the AI model.
        *   Delegates tool execution via `tool_provider.execute_tool(...)`.
        *   Collects results into `AgentOutput`.

## Phase 4: Adapt `volition-cli`

1.  **Implement `CliToolProvider` (in `volition-cli`):**
    *   Create `struct CliToolProvider` implementing `volition_agent_core::ToolProvider`.
    *   `get_tool_definitions`: Return definitions for all CLI tools (git, cargo, file, search, user_input, etc.).
    *   `execute_tool`: Dispatch to implementation functions (currently in `src/tools/*`), handle interactive `user_input`.
2.  **Update `main.rs`:**
    *   Load `AgentConfig`.
    *   Instantiate `CliToolProvider`.
    *   Instantiate `Agent`.
    *   Call `agent.run(...)`.
    *   Interpret the returned `AgentOutput`: Display summary, show tool results, potentially ask for confirmation based on results, perform final actions (like committing changes if files were modified by tools).
3.  **Cleanup:** Remove code/modules moved to `volition-agent-core`.

## Phase 5: Testing & Refinement

1.  **Core Tests:** Add unit/integration tests in `volition-agent-core` using a `MockToolProvider`.
2.  **CLI Tests:** Update/ensure `volition-cli` tests pass.
3.  **Quality Checks:** Run `cargo check/fmt/clippy/test --workspace` frequently.
4.  **Documentation:** Add Rustdoc comments to `volition-agent-core`'s public API. Update READMEs.

## Phase 6: Standard Tools (Optional Future Enhancement)

*   Consider extracting common, non-interactive tool implementations (e.g., `GitTool`, `CargoTool`, `FileSystemTool` using `std::fs`) into `volition-agent-core::tools::*`.
*   Consumers like `CliToolProvider` could then compose these standard tools alongside their environment-specific ones (like `CliUserInputTool`).
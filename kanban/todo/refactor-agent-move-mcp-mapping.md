# Task: Move MCP Schema Mapping Function

**Context:**
The `volition-core/src/agent.rs` file contains the function `mcp_schema_to_tool_params`, which is responsible for converting the JSON schema definition provided by an MCP tool into the `ToolParametersDefinition` struct used internally and by AI APIs. This function is purely for data mapping and transformation (~80 lines) and resides alongside the core agent logic.

**Goal:**
Improve code organization by moving this utility/mapping function out of `agent.rs` into a more appropriate location.

**Proposed Steps:**
1.  Identify the `mcp_schema_to_tool_params` function in `agent.rs`.
2.  Create a new module dedicated to MCP-related utilities or mapping, for example:
    *   `volition-core/src/mcp/mapping.rs` (and `volition-core/src/mcp/mod.rs` if it doesn't exist).
    *   Or potentially `volition-core/src/utils/mcp_mapping.rs`.
3.  Move the `mcp_schema_to_tool_params` function into the new module file.
4.  Make the function public (`pub fn ...`) so it can be called from `agent.rs`.
5.  Update the call site within `agent.rs` (inside the `NextStep::CallApi` arm or `handle_call_api` helper) to use the new path (e.g., `crate::mcp::mapping::mcp_schema_to_tool_params(...)`).
6.  Ensure all necessary imports (`serde_json`, `std::collections::HashMap`, internal tool model types) are added to the new module file.
7.  Update the parent module (`mcp/mod.rs` or `utils/mod.rs`) to declare the new mapping module (`pub mod mapping;`).
8.  Update `lib.rs` if the top-level module (`mcp` or `utils`) is new.
9.  Run `cargo check`, `cargo clippy`, and `cargo test` within `volition-core`.

**Affected Files:**
*   `volition-core/src/agent.rs`
*   `volition-core/src/mcp/mapping.rs` (new)
*   `volition-core/src/mcp/mod.rs` (new or modified)
*   `volition-core/src/lib.rs` (potentially modified)

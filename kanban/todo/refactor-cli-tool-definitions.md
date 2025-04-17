# Task: Refactor CLI Tool Definitions

**Context:**
The `CliToolProvider::get_tool_definitions` function in `volition-cli/src/tools/provider.rs` is responsible for creating the list of `ToolDefinition` structs for all available CLI tools. It currently does this by manually constructing each `ToolDefinition` within the function body, leading to a large (~150 lines) and repetitive function.

**Goal:**
Refactor `get_tool_definitions` to reduce repetition and make adding/modifying tool definitions easier by separating the tool metadata from the construction logic.

**Proposed Steps:**

1.  **Define Tool Metadata:**
    *   Create a static data structure (e.g., a `const` array or `lazy_static` map) within `provider.rs` to hold the core metadata for each tool.
    *   This structure could contain tuples or small structs holding the tool's name, description, and a representation of its parameters (e.g., a list of parameter names, types, descriptions, and whether they are required).
    *   Example structure element: `("shell", "Run a shell command...", &[("command", ParamType::String, "The command...", true)])`

2.  **Refactor `get_tool_definitions`:**
    *   Modify the function to iterate over the static metadata structure defined in step 1.
    *   For each entry in the metadata, dynamically construct the `ToolDefinition` and its `ToolParametersDefinition`.
    *   Use the existing helper functions (`Self::string_param`, etc.) or create new ones based on the parameter type defined in the metadata structure.
    *   This significantly shortens the function body, making it primarily a loop that transforms the static metadata into the required `Vec<ToolDefinition>`.

3.  **Maintain Clarity:** Ensure the chosen metadata structure and the generation logic remain clear and easy to understand.

4.  **Testing:** Run `cargo check`, `cargo clippy`, and `cargo test` within `volition-cli` to ensure the generated definitions are correct and the provider still functions as expected.

**Benefits:**
*   Reduces boilerplate code within `get_tool_definitions`.
*   Makes adding, removing, or modifying tool definitions simpler by editing the static metadata structure.
*   Separates the definition data from the construction logic.

**Affected Files:**
*   `volition-cli/src/tools/provider.rs`

# Plan: Specialize Volition CLI for Rust Development

**Goal:** Transform the general-purpose AI coding assistant into a specialized tool focused on enhancing Rust development workflows.

**Rationale:**

*   **Focused Expertise:** Provide more relevant and accurate Rust-specific assistance.
*   **Tailored Tools:** Optimize existing tools and add new ones for the Rust ecosystem.
*   **Stronger Assumptions:** Leverage the Rust context (project structure, build system) for better AI understanding.
*   **Deeper Integration Potential:** Enable future integrations with Rust-specific tools like LSP servers (`rust-analyzer`).
*   **Clearer Product Identity:** Define a distinct purpose and target audience.

**Proposed Steps:**

1.  **Update System Prompt (`Volition.toml`):**
    *   Modify the core prompt to explicitly state the assistant's specialization in Rust.
    *   Emphasize knowledge of Rust syntax, idioms, standard library, common crates (tokio, serde, anyhow, etc.), error handling patterns, lifetimes, borrowing, and `cargo`.
    *   Instruct the AI to assume it's operating within a Rust/Cargo project unless told otherwise.

2.  **Refine Existing Tools:**
    *   `search_text`: Default `file_glob` to `*.rs` or relevant Rust files. Add common Rust patterns.
    *   `find_definition`: Enhance patterns for Rust-specific items (traits, impls, macros). Default language to Rust.
    *   `cargo_command`: Potentially add helper arguments or common command suggestions.
    *   `git_command`: No specific changes needed initially, but useful in Rust context.
    *   `read_file`/`write_file`: No specific changes needed initially.
    *   `shell`: Use primarily for Rust-related tasks (e.g., checking tool versions like `rust-analyzer`).
    *   `user_input`: No changes needed.
    *   `list_directory`: No changes needed.

3.  **Consider New Rust-Specific Tools:**
    *   `cargo_check_dependencies`: Analyze `Cargo.toml` for unused or outdated dependencies.
    *   `cargo_suggest_crates`: Suggest relevant crates based on a described need.
    *   `rustfmt_check`/`rustfmt_apply`: Integrate `rustfmt` directly. (Alternative to `cargo fmt`)
    *   `clippy_check`/`clippy_apply`: Integrate `clippy` directly. (Alternative to `cargo clippy`)
    *   `explain_rust_error`: Attempt to provide more context on common Rust compiler errors (e.g., lifetime issues).
    *   `lsp_find_references`/`lsp_rename` (Future): Interface with an LSP server.

4.  **Review & Refactor Codebase:**
    *   Identify areas in the Volition codebase itself where assuming a Rust context simplifies logic.
    *   Ensure internal error handling and logging align with Rust best practices.

5.  **Documentation:**
    *   Update README and any other documentation to reflect the Rust focus.

**Implementation Order:**

1.  Update System Prompt.
2.  Refine existing tools (defaults and patterns).
3.  Evaluate and potentially implement 1-2 high-impact new Rust-specific tools.
4.  Review codebase for simplifications.
5.  (Longer Term) Investigate LSP integration.
6.  Update documentation throughout the process.

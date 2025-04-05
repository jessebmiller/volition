# Task: Refactor CLI Logging Setup

**Context:**
The `volition-cli/src/main.rs` file currently handles logging initialization directly within the `main` function. This involves setting up `tracing`, file appenders, formatting, and environment filters, contributing significantly (~50-60 lines) to the length of `main`.

**Goal:**
Extract the logging setup logic into a dedicated function or module to simplify `main.rs` and improve organization.

**Proposed Steps:**
1.  Identify all code related to logging setup within `main` in `volition-cli/src/main.rs` (dependencies: `tracing`, `tracing_subscriber`, `tracing_appender`, `dirs`, `time`, `colored`).
2.  Create a new file, e.g., `volition-cli/src/logging.rs`.
3.  Define a public function within `logging.rs`, perhaps `pub fn setup_logging(verbosity: u8) -> Result<impl Drop, anyhow::Error>` (the `Drop` guard is needed for non-blocking file logging).
4.  Move the logging initialization logic from `main` into this new function.
5.  Update `main` in `main.rs` to call `logging::setup_logging(cli.verbose)`.
6.  Ensure all necessary imports are moved to `logging.rs` or added to `main.rs` if needed for the function call.
7.  Update the `volition-cli/src/main.rs` module declaration if `logging.rs` is created (`mod logging;`).
8.  Run `cargo check` and `cargo test` within `volition-cli` to ensure everything compiles and works correctly.

**Affected Files:**
*   `volition-cli/src/main.rs`
*   `volition-cli/src/logging.rs` (new)

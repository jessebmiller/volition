# Task: Refactor CLI Configuration Loading

**Context:**
The `volition-cli/src/main.rs` file contains functions (`find_project_root`, `load_cli_config`, `load_git_server_allowed_commands`), structs (`CliTomlConfig`), and logic within `main` dedicated to finding, loading, parsing, and modifying the `Volition.toml` configuration.

**Goal:**
Consolidate configuration loading logic into a dedicated module to simplify `main.rs` and improve modularity.

**Proposed Steps:**
1.  Identify all functions, structs, and logic related to configuration loading in `volition-cli/src/main.rs`.
    *   Functions: `find_project_root`, `load_cli_config`, `load_git_server_allowed_commands`.
    *   Structs: `GitServerCliConfig`, `CliTomlConfig`.
    *   Logic in `main`: Calling `load_cli_config`, handling its result, calling `load_git_server_allowed_commands`, and modifying the `AgentConfig` based on its result.
2.  Create a new file, e.g., `volition-cli/src/config.rs`.
3.  Move the identified functions and structs into `config.rs`.
4.  Consider creating a higher-level function in `config.rs`, e.g., `pub fn load_and_prepare_config() -> Result<(AgentConfig, PathBuf)>`, that encapsulates the steps currently performed in `main` (finding root, loading base config, loading CLI-specific TOML parts, modifying config).
5.  Update `main` in `main.rs` to call this new higher-level configuration loading function.
6.  Ensure all necessary imports (`toml`, `serde`, `fs`, `path`, `env`, `anyhow`, `volition_core::config::AgentConfig`) are moved or added correctly.
7.  Update the `volition-cli/src/main.rs` module declaration (`mod config;`).
8.  Run `cargo check` and `cargo test` within `volition-cli` to ensure correctness.

**Affected Files:**
*   `volition-cli/src/main.rs`
*   `volition-cli/src/config.rs` (new)

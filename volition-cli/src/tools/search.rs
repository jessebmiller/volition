// volition-cli/src/tools/search.rs

// Re-export the core tool functions for use within the CLI crate
pub use volition_agent_core::tools::search::{find_rust_definition, search_text};

// The wrappers run_search_text and run_find_rust_definition were removed
// as they provided no additional value over the core functions.
// Call sites should use the re-exported functions above directly.

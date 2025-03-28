// volition-cli/src/tools/search.rs

// This file now acts as a wrapper or re-exporter if needed.
// The core logic is in volition_agent_core::tools::search.

use anyhow::Result;
use std::path::Path;

// Re-export the core functions to be used by CliToolProvider
pub use volition_agent_core::tools::search::{find_rust_definition, search_text};

// No CLI-specific logic (like confirmations) needed for search/find_rust_definition,
// so the wrappers just call the core functions directly.

/// Wrapper for search_text (no CLI-specific logic added).
pub async fn run_search_text(
    pattern: &str,
    search_path: Option<&str>,
    file_glob: Option<&str>,
    case_sensitive: Option<bool>,
    context_lines: Option<u32>,
    max_results: Option<usize>,
    working_dir: &Path,
) -> Result<String> {
    // Directly call the core implementation
    search_text(
        pattern,
        search_path,
        file_glob,
        case_sensitive,
        context_lines,
        max_results,
        working_dir,
    )
    .await
}

/// Wrapper for find_rust_definition (no CLI-specific logic added).
pub async fn run_find_rust_definition(
    symbol: &str,
    search_path: Option<&str>,
    working_dir: &Path,
) -> Result<String> {
    // Directly call the core implementation
    find_rust_definition(symbol, search_path, working_dir).await
}

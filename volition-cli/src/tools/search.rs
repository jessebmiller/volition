// volition-cli/src/tools/search.rs

use anyhow::Result;
use std::path::Path;

pub use volition_agent_core::tools::search::{find_rust_definition, search_text};

// TODO are these wrappers needed? can we use the library functions directly?
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
    find_rust_definition(symbol, search_path, working_dir).await
}

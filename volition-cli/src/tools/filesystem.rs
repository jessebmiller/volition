// volition-cli/src/tools/filesystem.rs

// This file now acts as a wrapper or re-exporter if needed.
// The core logic is in volition_agent_core::tools::fs.

use anyhow::Result;
use std::path::Path;

// Re-export the core function
pub use volition_agent_core::tools::fs::list_directory_contents;

/// Wrapper for list_directory_contents (no CLI-specific logic added).
pub fn run_list_directory_contents(
    relative_path: &str,
    max_depth: Option<usize>,
    show_hidden: bool,
    working_dir: &Path,
) -> Result<String> {
    // Directly call the core implementation
    list_directory_contents(relative_path, max_depth, show_hidden, working_dir)
}

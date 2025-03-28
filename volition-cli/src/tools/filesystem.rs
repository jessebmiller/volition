// volition-cli/src/tools/filesystem.rs

use anyhow::Result;
use std::path::Path;

pub use volition_agent_core::tools::fs::list_directory_contents;

// TODO: is this wrapper needed any more?
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

// volition-cli/src/tools/file.rs
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use volition_agent_core::tools::fs::{read_file as read_file_core, write_file as write_file_core};

/// Wrapper for read_file (no CLI-specific logic needed).
pub async fn read_file(relative_path: &str, working_dir: &Path) -> Result<String> {
    read_file_core(relative_path, working_dir).await
}

/// Wrapper for write_file, includes CLI-specific confirmation for writes outside working_dir.
pub async fn write_file(relative_path: &str, content: &str, working_dir: &Path) -> Result<String> {
    let target_path_relative = PathBuf::from(relative_path);
    let absolute_target_path = working_dir.join(&target_path_relative);

    // Note: This check uses simple path prefixing. More robust checks might be needed
    // depending on security requirements (e.g., handling symlinks).
    let is_within_project = absolute_target_path.starts_with(working_dir);

    debug!(
        "CLI write_file check: Relative: {:?}, Absolute: {:?}, WorkingDir: {:?}, Within: {}",
        relative_path, absolute_target_path, working_dir, is_within_project
    );

    if !is_within_project {
        warn!(
            "Attempt to write file outside working directory via CLI tool: {}",
            relative_path
        );

        print!(
            "{}\n{}{} ",
            format!(
                "WARNING: Attempting to write OUTSIDE working directory: {}",
                relative_path
            )
            .red()
            .bold(),
            "Allow write? ".yellow(),
            "(y/N):".yellow().bold()
        );
        io::stdout().flush().context("Failed to flush stdout")?;

        let mut user_choice = String::new();
        io::stdin()
            .read_line(&mut user_choice)
            .context("Failed to read user input")?;

        if user_choice.trim().to_lowercase() != "y" {
            warn!(
                "User denied write to outside working directory: {}",
                relative_path
            );
            println!("{}", "File write denied.".red());
            return Ok(format!("File write denied by user: {}", relative_path));
        }
        info!(
            "User approved write outside working directory: {}",
            relative_path
        );
    }

    write_file_core(relative_path, content, working_dir).await
}

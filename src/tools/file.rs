use std::fs;
use std::path::{Path, PathBuf};
// Add standard IO for direct input
use std::io::{self, Write};
use anyhow::{Result, Context};
use colored::*;
// Import RuntimeConfig
use crate::config::RuntimeConfig;
// Removed unused UserInputArgs and user_input module import
use crate::models::tools::{ReadFileArgs, WriteFileArgs};
use tracing::{info, warn, debug};

pub async fn read_file(args: ReadFileArgs) -> Result<String> {
    let path = &args.path;

    info!("Reading file: {}", path);

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;

    info!("Read {} bytes from file", content.len());

    Ok(content)
}

pub async fn write_file(args: WriteFileArgs, config: &RuntimeConfig) -> Result<String> {
    let path_str = &args.path;
    let content = &args.content;
    let target_path = PathBuf::from(path_str);

    // --- Check if path is within project root ---
    let absolute_target_path = if target_path.is_absolute() {
        target_path.clone()
    } else {
        config.project_root.join(&target_path)
    };

    // Attempt to canonicalize for a more robust check, but fall back if it fails (e.g., path doesn't exist yet)
    let canonical_path = absolute_target_path.canonicalize().unwrap_or(absolute_target_path.clone());

    let is_within_project = canonical_path.starts_with(&config.project_root);
    debug!("Target path: {:?}, Absolute Attempt: {:?}, Canonical Attempt: {:?}, Project Root: {:?}, Within Project: {}",
           path_str, absolute_target_path, canonical_path, config.project_root, is_within_project);

    if !is_within_project {
        warn!("Attempt to write file outside project root: {}", path_str);

        // --- Updated Confirmation (y/N style, default No) ---
        print!(
            "{}\n{}{} ",
            format!("WARNING: Attempting to write OUTSIDE project directory: {}", path_str).red().bold(),
            "Allow write? ".yellow(),
            "(y/N):".yellow().bold() // Default to No
        );
        // Ensure the prompt is displayed before reading input
        io::stdout().flush().context("Failed to flush stdout")?;

        let mut user_choice = String::new();
        io::stdin()
            .read_line(&mut user_choice)
            .context("Failed to read user input")?;

        // Only proceed if the user explicitly types 'y' (case-insensitive)
        if user_choice.trim().to_lowercase() != "y" {
            warn!("User denied write to outside project root: {}", path_str);
            println!("{}", "File write denied.".red());
            return Ok(format!("File write denied by user: {}", path_str));
        }
        // --- End Confirmation ---
        info!("User approved write outside project root: {}", path_str);
    }
    // --- End Check ---

    info!("Writing to file: {}", path_str);

    // Create parent directories if they don't exist
    // Use the original target_path (relative or absolute) as provided by the user/AI
    let path_to_create = Path::new(path_str);
    if let Some(parent) = path_to_create.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
    }

    fs::write(path_to_create, content)
        .with_context(|| format!("Failed to write to file: {}", path_str))?;

    info!("Successfully wrote {} bytes to file", content.len());

    Ok(format!("Successfully wrote to file: {}", path_str))
}

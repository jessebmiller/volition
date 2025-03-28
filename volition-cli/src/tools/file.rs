// volition-cli/src/tools/file.rs
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use tracing::{debug, info, warn};

// Removed unused imports: RuntimeConfig, ReadFileArgs, WriteFileArgs

/// Reads the entire content of a file relative to the working directory.
pub async fn read_file(relative_path: &str, working_dir: &Path) -> Result<String> {
    let absolute_path = working_dir.join(relative_path);
    info!("Reading file (absolute): {:?}", absolute_path);
    let content = fs::read_to_string(&absolute_path)
        .with_context(|| format!("Failed to read file: {:?}", absolute_path))?;
    info!("Read {} bytes from file", content.len());
    Ok(content)
}

/// Writes content to a file relative to the working directory.
/// Includes safety check for writing outside the working directory.
pub async fn write_file(
    relative_path: &str,
    content: &str,
    working_dir: &Path,
) -> Result<String> {
    let target_path_relative = PathBuf::from(relative_path);

    // --- Construct Absolute Path --- Always resolve relative to working directory
    let absolute_target_path = if target_path_relative.is_absolute() {
        // If user provided absolute path, use it directly (but check sandbox below)
        target_path_relative.clone()
    } else {
        // Otherwise, join with working_dir
        working_dir.join(&target_path_relative)
    };
    // Clean the path (e.g. resolve ..)
    // Using std::fs::canonicalize requires existence, which might not be the case yet.
    // For simplicity, we rely on the starts_with check below, assuming no malicious symlinks.
    // let absolute_target_path = normalize_path(&absolute_target_path); // If a helper exists

    // --- Check if path is within working directory (sandbox) ---
    let is_within_project = absolute_target_path.starts_with(working_dir);

    debug!(
        "Target path: {:?}, Resolved Absolute: {:?}, Working Dir: {:?}, Within Dir: {}",
        relative_path, absolute_target_path, working_dir, is_within_project
    );

    if !is_within_project {
        warn!("Attempt to write file outside working directory: {}", relative_path);

        // --- Confirmation Logic (y/N style, default No) ---
        print!(
            "{}\n{}{} ",
            format!(
                "WARNING: Attempting to write OUTSIDE working directory: {}",
                relative_path // Show original relative path in warning
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
            warn!("User denied write to outside working directory: {}", relative_path);
            println!("{}", "File write denied.".red());
            return Ok(format!("File write denied by user: {}", relative_path));
        }
        info!("User approved write outside working directory: {}", relative_path);
    }
    // --- End Check ---

    info!(
        "Writing to file (absolute path): {:?}",
        absolute_target_path
    );

    // Create parent directories if they don't exist, using the absolute path
    if let Some(parent) = absolute_target_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            info!("Created parent directory: {:?}", parent);
        }
    }

    // Write using the absolute path
    fs::write(&absolute_target_path, content)
        .with_context(|| format!("Failed to write to file: {:?}", absolute_target_path))?;

    info!("Successfully wrote {} bytes to file", content.len());

    // Return the original relative path string provided by the user in the success message
    Ok(format!("Successfully wrote to file: {}", relative_path))
}

// Helper function might be needed for robust path normalization if not using external crate
// fn normalize_path(path: &Path) -> PathBuf { ... }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use tokio;
    use std::path::Path;

    #[tokio::test]
    async fn test_read_file_success() {
        let dir = tempdir().unwrap();
        let file_path_relative = "test_read.txt";
        let file_path_absolute = dir.path().join(file_path_relative);
        let expected_content = "Hello, Volition!";
        fs::write(&file_path_absolute, expected_content).unwrap();

        let result = read_file(file_path_relative, dir.path()).await;

        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content, expected_content);
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let dir = tempdir().unwrap();
        let file_path_relative = "non_existent_file.txt";

        let result = read_file(file_path_relative, dir.path()).await;

        assert!(result.is_err());
        let error_string = result.err().unwrap().to_string();
        assert!(error_string.contains("Failed to read file"));
    }

    #[tokio::test]
    async fn test_read_file_is_directory() {
        let dir = tempdir().unwrap();
        // Try reading the directory itself as a file
        let result = read_file(".", dir.path()).await; // Pass relative path "."

        assert!(result.is_err());
        let error_string = result.err().unwrap().to_string();
        assert!(error_string.contains("Failed to read file"));
    }

    // --- write_file tests ---

    #[tokio::test]
    async fn test_write_file_success_new() {
        let dir = tempdir().unwrap();
        let file_path_relative = "test_write_new.txt";
        let file_path_absolute = dir.path().join(file_path_relative);
        let content_to_write = "Writing a new file.";

        let result = write_file(file_path_relative, content_to_write, dir.path()).await;

        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(file_path_absolute.exists(), "File was not created");

        let read_content = fs::read_to_string(&file_path_absolute).unwrap();
        assert_eq!(read_content, content_to_write);
        assert!(result.unwrap().contains(&format!(
            "Successfully wrote to file: {}",
            file_path_relative
        )));
    }

    #[tokio::test]
    async fn test_write_file_success_overwrite() {
        let dir = tempdir().unwrap();
        let file_path_relative = "test_write_overwrite.txt";
        let file_path_absolute = dir.path().join(file_path_relative);
        let initial_content = "Initial content.";
        let content_to_write = "Overwritten content.";

        fs::write(&file_path_absolute, initial_content).unwrap();

        let result = write_file(file_path_relative, content_to_write, dir.path()).await;

        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(file_path_absolute.exists());

        let read_content = fs::read_to_string(&file_path_absolute).unwrap();
        assert_eq!(read_content, content_to_write);
        assert!(result.unwrap().contains(&format!(
            "Successfully wrote to file: {}",
            file_path_relative
        )));
    }

    #[tokio::test]
    async fn test_write_file_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let nested_dir_relative = Path::new("nested");
        let file_path_relative = nested_dir_relative.join("test_write_nested.txt");
        let nested_dir_absolute = dir.path().join(&nested_dir_relative);
        let file_path_absolute = dir.path().join(&file_path_relative);
        let content_to_write = "Content in nested directory.";

        assert!(!nested_dir_absolute.exists());

        let result = write_file(
            file_path_relative.to_str().unwrap(),
            content_to_write,
            dir.path(),
        )
        .await;

        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(nested_dir_absolute.exists(), "Nested dir not created");
        assert!(file_path_absolute.exists(), "File not created");

        let read_content = fs::read_to_string(&file_path_absolute).unwrap();
        assert_eq!(read_content, content_to_write);
        assert!(result.unwrap().contains(&format!(
            "Successfully wrote to file: {}",
            file_path_relative.display()
        )));
    }

    // TODO: Tests for writing outside working directory (requires stdin/stdout mocking)
}

use std::fs;
// Removed unused `Path` import, only `PathBuf` needed here now
use std::path::PathBuf;
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use crate::config::RuntimeConfig;
use crate::models::tools::{ReadFileArgs, WriteFileArgs};
use tracing::{debug, info, warn};

pub async fn read_file(args: ReadFileArgs) -> Result<String> {
    let path = &args.path;
    info!("Reading file: {}", path);
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path))?;
    info!("Read {} bytes from file", content.len());
    Ok(content)
}

pub async fn write_file(args: WriteFileArgs, config: &RuntimeConfig) -> Result<String> {
    let path_str = &args.path;
    let content = &args.content;
    let target_path_relative = PathBuf::from(path_str);

    // --- Construct Absolute Path --- Always resolve relative to project root
    let absolute_target_path = if target_path_relative.is_absolute() {
        // If user provided absolute path, use it directly (but check sandbox below)
        target_path_relative.clone()
    } else {
        // Otherwise, join with project root
        config.project_root.join(&target_path_relative)
    };
     // Clean the path (e.g. resolve ..) for more reliable checks. std::fs::canonicalize requires existence.
     // Using a simple normalization approach for now.
    // let absolute_target_path = normalize_path(&absolute_target_path); // Assuming a helper if needed


    // --- Check if path is within project root ---
    // Use starts_with on the potentially non-canonicalized path. This is generally safe
    // unless symlinks are used maliciously to escape the root.
    let is_within_project = absolute_target_path.starts_with(&config.project_root);

    debug!(
        "Target path: {:?}, Resolved Absolute: {:?}, Project Root: {:?}, Within Project: {}",
        path_str, absolute_target_path, config.project_root, is_within_project
    );

    if !is_within_project {
        warn!("Attempt to write file outside project root: {}", path_str);

        // --- Confirmation Logic (y/N style, default No) ---
        print!(
            "{}\n{}{} ",
            format!(
                "WARNING: Attempting to write OUTSIDE project directory: {}",
                path_str // Show original path in warning
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
            warn!("User denied write to outside project root: {}", path_str);
            println!("{}", "File write denied.".red());
            // Return Ok with a message, as denying isn't a program error
            return Ok(format!("File write denied by user: {}", path_str));
        }
        info!("User approved write outside project root: {}", path_str);
    }
    // --- End Check ---

    info!("Writing to file (absolute path): {:?}", absolute_target_path);

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

    // Return the original path string provided by the user in the success message
    Ok(format!("Successfully wrote to file: {}", path_str))
}

// Helper function might be needed for robust path normalization if not using external crate
// fn normalize_path(path: &Path) -> PathBuf { ... }


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuntimeConfig;
    use tempfile::tempdir;
    use std::fs::{File};
    use std::io::Write;
    use std::collections::HashMap;
    use tokio;
    // Need std::path::Path for helper function signature and tests
    use std::path::Path;

    #[allow(dead_code)]
    fn create_dummy_config_for_dir(project_dir: &Path) -> RuntimeConfig {
        RuntimeConfig {
            system_prompt: "".to_string(),
            selected_model: "".to_string(),
            models: HashMap::new(),
            api_key: "".to_string(),
            project_root: project_dir.to_path_buf(),
        }
    }

    #[tokio::test]
    async fn test_read_file_success() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_read.txt");
        let expected_content = "Hello, Volition!";
        let mut file = File::create(&file_path).unwrap();
        file.write_all(expected_content.as_bytes()).unwrap();
        drop(file);

        let args = ReadFileArgs { path: file_path.to_str().unwrap().to_string() };
        let result = read_file(args).await;

        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content, expected_content);
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("non_existent_file.txt");

        let args = ReadFileArgs { path: file_path.to_str().unwrap().to_string() };
        let result = read_file(args).await;

        assert!(result.is_err());
        let error_string = result.err().unwrap().to_string();
        assert!(error_string.contains("Failed to read file"));
    }

    #[tokio::test]
    async fn test_read_file_is_directory() {
        let dir = tempdir().unwrap();

        let args = ReadFileArgs { path: dir.path().to_str().unwrap().to_string() };
        let result = read_file(args).await;

        assert!(result.is_err());
         let error_string = result.err().unwrap().to_string();
        assert!(error_string.contains("Failed to read file"));
    }

    // --- write_file tests ---

    #[tokio::test]
    async fn test_write_file_success_new() {
        let dir = tempdir().unwrap();
        let config = create_dummy_config_for_dir(dir.path());
        let file_path_relative = "test_write_new.txt";
        let file_path_absolute = dir.path().join(file_path_relative);
        let content_to_write = "Writing a new file.";

        let args = WriteFileArgs {
            path: file_path_relative.to_string(),
            content: content_to_write.to_string(),
        };

        let result = write_file(args, &config).await;

        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(file_path_absolute.exists(), "File was not created at absolute path");

        let read_content = fs::read_to_string(&file_path_absolute).unwrap();
        assert_eq!(read_content, content_to_write);
        assert!(result.unwrap().contains(&format!("Successfully wrote to file: {}", file_path_relative)));
    }

    #[tokio::test]
    async fn test_write_file_success_overwrite() {
        let dir = tempdir().unwrap();
        let config = create_dummy_config_for_dir(dir.path());
        let file_path_relative = "test_write_overwrite.txt";
        let file_path_absolute = dir.path().join(file_path_relative);
        let initial_content = "Initial content.";
        let content_to_write = "Overwritten content.";

        fs::write(&file_path_absolute, initial_content).unwrap();

        let args = WriteFileArgs {
            path: file_path_relative.to_string(),
            content: content_to_write.to_string(),
        };

        let result = write_file(args, &config).await;

        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(file_path_absolute.exists());

        let read_content = fs::read_to_string(&file_path_absolute).unwrap();
        assert_eq!(read_content, content_to_write);
        assert!(result.unwrap().contains(&format!("Successfully wrote to file: {}", file_path_relative)));
    }

    #[tokio::test]
    async fn test_write_file_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let config = create_dummy_config_for_dir(dir.path());
        let nested_dir_relative = Path::new("nested");
        let file_path_relative = nested_dir_relative.join("test_write_nested.txt");
        let nested_dir_absolute = dir.path().join(&nested_dir_relative);
        let file_path_absolute = dir.path().join(&file_path_relative);
        let content_to_write = "Content in nested directory.";

        assert!(!nested_dir_absolute.exists());

        let args = WriteFileArgs {
            path: file_path_relative.to_str().unwrap().to_string(),
            content: content_to_write.to_string(),
        };

        let result = write_file(args, &config).await;

        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(nested_dir_absolute.exists(), "Nested directory was not created");
        assert!(file_path_absolute.exists(), "File was not created in nested directory");

        let read_content = fs::read_to_string(&file_path_absolute).unwrap();
        assert_eq!(read_content, content_to_write);
         assert!(result.unwrap().contains(&format!("Successfully wrote to file: {}", file_path_relative.display())));
    }

    // TODO: Tests for writing outside project root (requires stdin/stdout mocking)
}

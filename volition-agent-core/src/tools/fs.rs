// volition-agent-core/src/tools/fs.rs

use crate::utils::truncate_string; // <-- Import the helper
use anyhow::{anyhow, Context, Result};
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub async fn read_file(relative_path: &str, working_dir: &Path) -> Result<String> {
    let absolute_path = working_dir.join(relative_path);
    // Use relative path for logging, truncated
    let path_display = truncate_string(relative_path, 60);
    info!("Reading file: {}", path_display);

    let content = fs::read_to_string(&absolute_path)
        .with_context(|| format!("fs::read_to_string failed for: {:?}", absolute_path))?;
    info!("Read {} bytes from file {}", content.len(), path_display);
    Ok(content)
}

pub async fn write_file(relative_path: &str, content: &str, working_dir: &Path) -> Result<String> {
    let target_path_relative = PathBuf::from(relative_path);
    let absolute_target_path = working_dir.join(&target_path_relative);

    // Use relative path for logging, truncated
    let path_display = truncate_string(relative_path, 60);
    info!("Writing to file: {}", path_display);

    if let Some(parent) = absolute_target_path.parent() {
        if !parent.exists() {
            // Log absolute parent path, but truncated
            let parent_display = truncate_string(&parent.to_string_lossy(), 60);
            info!("Creating parent directory: {}", parent_display);
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }
    }

    fs::write(&absolute_target_path, content)
        .with_context(|| format!("fs::write failed for: {:?}", absolute_target_path))?;

    info!(
        "Successfully wrote {} bytes to file {}",
        content.len(),
        path_display
    );

    Ok(format!("Successfully wrote to file: {}", relative_path))
}

pub fn list_directory_contents(
    relative_path: &str,
    max_depth: Option<usize>,
    show_hidden: bool,
    working_dir: &Path,
) -> Result<String> {
    let start_path = working_dir.join(relative_path);

    if !start_path.is_dir() {
        return Err(anyhow!(
            "Resolved path is not a directory: {:?}",
            start_path
        ));
    }
    debug!(
        "Listing directory contents for {:?} (relative path: {}), depth: {:?}, hidden: {}",
        start_path, relative_path, max_depth, show_hidden
    );

    let mut output = String::new();
    let mut walker_builder = WalkBuilder::new(&start_path);
    walker_builder
        .hidden(!show_hidden)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true);

    let gitignore_path = start_path.join(".gitignore");
    if gitignore_path.is_file() {
        let _ = walker_builder.add_ignore(&gitignore_path);
    }

    if let Some(depth) = max_depth {
        walker_builder.max_depth(Some(depth));
    }

    let walker = walker_builder.build();

    for result in walker {
        match result {
            Ok(entry) => {
                if entry.depth() == 0 && max_depth.is_none_or(|d| d > 0) {
                    continue;
                }
                match entry.path().strip_prefix(&start_path) {
                    Ok(path_relative_to_start) => {
                        if path_relative_to_start.as_os_str().is_empty() {
                            continue;
                        }
                        output.push_str(&path_relative_to_start.display().to_string());
                        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                            output.push('/');
                        }
                        output.push('\n');
                    }
                    Err(_) => {
                        output.push_str(&entry.path().display().to_string());
                        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                            output.push('/');
                        }
                        output.push('\n');
                    }
                }
            }
            Err(err) => {
                debug!("Warning during directory walk: {}", err);
            }
        }
    }

    Ok(output.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;
    use tokio;

    #[tokio::test]
    async fn test_fs_read_file_success() {
        let dir = tempdir().unwrap();
        let file_path_relative = "test_read.txt";
        let file_path_absolute = dir.path().join(file_path_relative);
        let expected_content = "Hello, Volition FS!";
        fs::write(&file_path_absolute, expected_content).unwrap();
        let result = read_file(file_path_relative, dir.path()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_content);
    }

    #[tokio::test]
    async fn test_fs_read_file_not_found() {
        let dir = tempdir().unwrap();
        let file_path_relative = "non_existent_file.txt";
        let result = read_file(file_path_relative, dir.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fs_write_file_success_new() {
        let dir = tempdir().unwrap();
        let file_path_relative = "test_write_new.txt";
        let file_path_absolute = dir.path().join(file_path_relative);
        let content_to_write = "Writing a new file via core fs.";
        let result = write_file(file_path_relative, content_to_write, dir.path()).await;
        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(file_path_absolute.exists());
        let read_content = fs::read_to_string(&file_path_absolute).unwrap();
        assert_eq!(read_content, content_to_write);
        assert!(result.unwrap().contains(file_path_relative));
    }

    #[tokio::test]
    async fn test_fs_write_file_creates_parents() {
        let dir = tempdir().unwrap();
        let file_path_relative = "nested/test_write_nested.txt";
        let file_path_absolute = dir.path().join(file_path_relative);
        let content_to_write = "Nested write.";
        let result = write_file(file_path_relative, content_to_write, dir.path()).await;
        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(file_path_absolute.exists());
        let read_content = fs::read_to_string(&file_path_absolute).unwrap();
        assert_eq!(read_content, content_to_write);
    }

    fn sort_lines(text: &str) -> Vec<&str> {
        let mut lines: Vec<&str> = text.lines().collect();
        lines.sort();
        lines
    }

    #[test]
    fn test_fs_list_basic() -> Result<()> {
        let dir = tempdir()?;
        let wd = dir.path();
        File::create(wd.join("f1.txt"))?;
        fs::create_dir(wd.join("sd"))?;
        File::create(wd.join("sd/f2.txt"))?;
        let output = list_directory_contents(".", Some(1), false, wd)?;
        assert_eq!(sort_lines(&output), sort_lines("f1.txt\nsd/"));
        Ok(())
    }

    #[test]
    fn test_fs_list_depth() -> Result<()> {
        let dir = tempdir()?;
        let wd = dir.path();
        File::create(wd.join("f1.txt"))?;
        fs::create_dir(wd.join("sd"))?;
        File::create(wd.join("sd/f2.txt"))?;
        let output = list_directory_contents(".", Some(2), false, wd)?;
        let expected = format!("f1.txt\nsd/\nsd{}f2.txt", std::path::MAIN_SEPARATOR);
        assert_eq!(sort_lines(&output), sort_lines(&expected));
        Ok(())
    }
}

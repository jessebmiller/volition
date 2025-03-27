// src/tools/filesystem.rs
use anyhow::Result;
use ignore::WalkBuilder;
use std::path::Path;

/// Lists directory contents, respecting .gitignore rules, up to a specified depth.
///
/// Args:
///     path_str: The starting directory path.
///     max_depth: Maximum depth to traverse (None for unlimited, 0 for starting path only, 1 for contents, etc.).
///     show_hidden: Whether to include hidden files/directories.
///
/// Returns:
///     A string containing a newline-separated list of relative paths, or an error.
pub fn list_directory_contents(
    path_str: &str,
    max_depth: Option<usize>,
    show_hidden: bool,
) -> Result<String> {
    let start_path = Path::new(path_str);
    if !start_path.is_dir() {
        return Err(anyhow::anyhow!("Path is not a directory: {}", path_str));
    }

    let mut output = String::new();
    // Configure the WalkBuilder
    let mut walker_builder = WalkBuilder::new(start_path);
    walker_builder
        .hidden(!show_hidden) // If show_hidden is true, we negate it for the .hidden() setting
        .git_ignore(true)     // Enable .gitignore respecting
        .git_global(true)     // Respect global gitignore
        .git_exclude(true)    // Respect .git/info/exclude
        .parents(true);       // Respect ignore files in parent directories

    // Explicitly add the .gitignore file in the root path if it exists
    // This helps ensure it's respected even if not in a git repo (like in tests)
    let gitignore_path = start_path.join(".gitignore");
    if gitignore_path.is_file() {
        let _ = walker_builder.add_ignore(&gitignore_path);
    }

    // Set the maximum depth if specified
    if let Some(depth) = max_depth {
        // Note: WalkBuilder depth is relative to the *start* path.
        // Depth 0 = only the start path itself (if it matches filters).
        // Depth 1 = contents of the start path.
        // We adjust the user's expectation (depth 1 = contents) to WalkBuilder's (depth 1 = contents).
        walker_builder.max_depth(Some(depth));
    }

    let walker = walker_builder.build();

    for result in walker {
        match result {
            Ok(entry) => {
                // Skip the root path itself if depth > 0 or depth is None
                // WalkBuilder depth 0 is the starting point.
                if entry.depth() == 0 && max_depth.map_or(true, |d| d > 0) {
                    continue;
                }

                // Get the path relative to the start_path
                match entry.path().strip_prefix(start_path) {
                    Ok(relative_path) => {
                        // Skip empty paths (can happen for the root dir itself sometimes)
                        if relative_path.as_os_str().is_empty() {
                            continue;
                        }
                        // Append the relative path to the output string
                        output.push_str(&relative_path.display().to_string());
                        // Add trailing slash for directories
                        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                           output.push('/');
                        }
                        output.push('\n');
                    }
                    Err(_) => {
                        // Fallback to the full path if stripping the prefix fails (shouldn't normally happen)
                        output.push_str(&entry.path().display().to_string());
                        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                           output.push('/');
                        }
                        output.push('\n');
                    }
                }
            }
            Err(err) => {
                // Append warnings for inaccessible entries
                output.push_str(&format!("WARN: Failed to access entry: {}\n", err));
            }
        }
    }

    Ok(output.trim_end().to_string()) // Trim trailing newline if any
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir; // Import tempdir here

    // Helper function to sort lines for comparison
    fn sort_lines(text: &str) -> Vec<&str> {
        let mut lines: Vec<&str> = text.lines().collect();
        lines.sort();
        lines
    }

    #[test]
    fn test_list_basic() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        File::create(path.join("file1.txt"))?;
        fs::create_dir(path.join("subdir"))?;
        File::create(path.join("subdir/file2.txt"))?;

        let output = list_directory_contents(path.to_str().unwrap(), Some(1), false)?;
        let expected = "file1.txt\nsubdir/";

        assert_eq!(sort_lines(&output), sort_lines(expected));
        Ok(())
    }

    #[test]
    fn test_list_depth() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        File::create(path.join("file1.txt"))?;
        fs::create_dir(path.join("subdir"))?;
        File::create(path.join("subdir/file2.txt"))?;

        let output = list_directory_contents(path.to_str().unwrap(), Some(2), false)?;

        // Use format! to handle path separators correctly
        let expected = format!(
            "file1.txt\nsubdir/\nsubdir{}file2.txt",
            std::path::MAIN_SEPARATOR
        );

        assert_eq!(sort_lines(&output), sort_lines(&expected));
        Ok(())
    }

    #[test]
    fn test_list_hidden() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        File::create(path.join(".hidden_file"))?;
        fs::create_dir(path.join(".hidden_dir"))?;
        File::create(path.join(".hidden_dir/file3.txt"))?;
        File::create(path.join("visible_file.txt"))?;

        // Test without showing hidden (depth 1)
        let output_no_hidden = list_directory_contents(path.to_str().unwrap(), Some(1), false)?;
        assert_eq!(output_no_hidden.trim(), "visible_file.txt");

        // Test with showing hidden (depth 1)
        let output_hidden = list_directory_contents(path.to_str().unwrap(), Some(1), true)?;
        let expected_hidden = ".hidden_dir/\n.hidden_file\nvisible_file.txt";
        assert_eq!(sort_lines(&output_hidden), sort_lines(expected_hidden));

        // Test with showing hidden (depth 2)
        let output_hidden_depth2 = list_directory_contents(path.to_str().unwrap(), Some(2), true)?;
        let expected_hidden_depth2 = format!(
            ".hidden_dir/\n.hidden_dir{}file3.txt\n.hidden_file\nvisible_file.txt",
             std::path::MAIN_SEPARATOR
        );
         assert_eq!(sort_lines(&output_hidden_depth2), sort_lines(&expected_hidden_depth2));

        Ok(())
    }

     #[test]
    fn test_list_gitignore() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        // Create .gitignore
        let gitignore_path = path.join(".gitignore");
        let mut gitignore = File::create(&gitignore_path)?;
        writeln!(gitignore, "ignored_file.txt")?;
        writeln!(gitignore, "ignored_dir/")?;
        gitignore.flush()?; // Ensure flushed before use
        drop(gitignore);

        // Create files and dirs
        File::create(path.join("visible_file.txt"))?;
        File::create(path.join("ignored_file.txt"))?;
        fs::create_dir(path.join("visible_dir"))?;
        File::create(path.join("visible_dir/sub_file.txt"))?;
        fs::create_dir(path.join("ignored_dir"))?;
        File::create(path.join("ignored_dir/sub_ignored.txt"))?;

        // Test without hidden, depth 1
        let output = list_directory_contents(path.to_str().unwrap(), Some(1), false)?;
        let expected = "visible_dir/\nvisible_file.txt"; // .gitignore is hidden
        assert_eq!(sort_lines(&output), sort_lines(expected));

        // Test showing hidden, depth 1
        let output_hidden = list_directory_contents(path.to_str().unwrap(), Some(1), true)?;
        let expected_hidden = ".gitignore\nvisible_dir/\nvisible_file.txt"; // .gitignore now visible
        assert_eq!(sort_lines(&output_hidden), sort_lines(expected_hidden));

        // Test depth 2 (should not include contents of ignored_dir)
        let output_depth2 = list_directory_contents(path.to_str().unwrap(), Some(2), false)?;
        let expected_depth2 = format!(
            "visible_dir/\nvisible_dir{}sub_file.txt\nvisible_file.txt",
            std::path::MAIN_SEPARATOR
        );
        assert_eq!(sort_lines(&output_depth2), sort_lines(&expected_depth2));

        Ok(())
    }

    #[test]
    fn test_list_depth_zero() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        File::create(path.join("file1.txt"))?;

        // Depth 0 should list nothing (only the dir itself, which we skip)
        let output = list_directory_contents(path.to_str().unwrap(), Some(0), false)?;
        assert_eq!(output, "");

        Ok(())
    }

     #[test]
    fn test_list_non_existent_path() {
        let result = list_directory_contents("/path/that/does/not/exist", Some(1), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Path is not a directory"));
    }

     #[test]
    fn test_list_file_path() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        let file_path = path.join("file1.txt");
        File::create(&file_path)?;

        let result = list_directory_contents(file_path.to_str().unwrap(), Some(1), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Path is not a directory"));
        Ok(())
    }
}

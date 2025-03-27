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
    let gitignore_path = start_path.join(".gitignore");
    if gitignore_path.is_file() {
        // This might return an error if the file is invalid, handle it?
        // For now, just log or ignore the error in the test context
        let _ = walker_builder.add_ignore(&gitignore_path);
    }

    // Set the maximum depth if specified
    if let Some(depth) = max_depth {
        // Note: WalkBuilder depth is relative to the *start* path.
        // Depth 0 = only the start path itself (if it matches filters).
        // Depth 1 = contents of the start path.
        // So, we add 1 because we usually think of depth 1 as the immediate contents.
        // If the user requests depth 0 (just the dir itself), we set WalkBuilder depth to 0.
        // If the user requests depth 1 (contents), we set WalkBuilder depth to 1.
        walker_builder.max_depth(Some(depth));
    }

    let walker = walker_builder.build();

    for result in walker {
        match result {
            Ok(entry) => {
                // Skip the root path itself if depth > 0 or depth is None
                if entry.depth() == 0 && max_depth.map_or(true, |d| d > 0) {
                    continue;
                }

                // Get the path relative to the *current working directory*
                // or use the absolute path if preferred. Using relative path from start_path is often cleaner.
                match entry.path().strip_prefix(start_path) {
                    Ok(relative_path) => {
                        // Skip empty paths (can happen for the root dir itself sometimes)
                        if relative_path.as_os_str().is_empty() {
                            continue;
                        }
                        // Append the relative path to the output string
                        // Use display() for cross-platform compatibility
                        output.push_str(&relative_path.display().to_string());
                        // Add trailing slash for directories for clarity (like ls -F or tree -F)
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
                // Log the error or decide how to handle it.
                // For now, let's just append an error message to the output.
                output.push_str(&format!("WARN: Failed to access entry: {}\n", err));
            }
        }
    }

    Ok(output.trim_end().to_string()) // Trim trailing newline if any
}

// Optional: Add some tests here
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir; // Import tempdir here

    #[test]
    fn test_list_basic() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();
        File::create(path.join("file1.txt"))?;
        fs::create_dir(path.join("subdir"))?;
        File::create(path.join("subdir/file2.txt"))?;

        let output = list_directory_contents(path.to_str().unwrap(), Some(1), false)?;
        let expected = "file1.txt\nsubdir/";
        // Order might vary, so check contents
        let mut lines: Vec<&str> = output.lines().collect();
        lines.sort();
        let mut expected_lines: Vec<&str> = expected.lines().collect();
        expected_lines.sort();

        assert_eq!(lines, expected_lines);
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
        let mut lines: Vec<&str> = output.lines().collect();
        lines.sort();

        // Change to Vec<String> to own the formatted string
        let expected_lines: Vec<String> = vec![
            "file1.txt".to_string(),
            "subdir/".to_string(),
            format!("subdir{}file2.txt", std::path::MAIN_SEPARATOR), // format! creates owned String
        ];
        // Now map to &str for sorting and comparison
        let mut expected_lines_sorted = expected_lines.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        expected_lines_sorted.sort();

        // Normalize paths in output for comparison
        let normalized_lines: Vec<String> = lines.iter().map(|s| s.replace('/', &std::path::MAIN_SEPARATOR.to_string())).collect();
        let mut normalized_lines_sorted: Vec<&str> = normalized_lines.iter().map(|s| s.as_str()).collect();
        normalized_lines_sorted.sort();


        assert_eq!(normalized_lines_sorted, expected_lines_sorted);


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


        // Test without showing hidden
        let output_no_hidden = list_directory_contents(path.to_str().unwrap(), Some(1), false)?;
        assert_eq!(output_no_hidden.trim(), "visible_file.txt");

        // Test with showing hidden
        let output_hidden = list_directory_contents(path.to_str().unwrap(), Some(1), true)?;
        let mut lines: Vec<&str> = output_hidden.lines().collect();
        lines.sort();
        let expected = [".hidden_dir/", ".hidden_file", "visible_file.txt"];
        let mut expected_lines: Vec<&str> = expected.to_vec();
        expected_lines.sort();
        assert_eq!(lines, expected_lines);

        Ok(())
    }

     #[test]
    fn test_list_gitignore() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path();

        // Create .gitignore
        let mut gitignore = File::create(path.join(".gitignore"))?;
        writeln!(gitignore, "ignored_file.txt")?;
        writeln!(gitignore, "ignored_dir/")?;
        // Ensure the file is flushed before WalkBuilder runs
        gitignore.flush()?;
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
        let mut lines: Vec<&str> = output.lines().collect();
        lines.sort();
        let expected = ["visible_dir/", "visible_file.txt"]; // .gitignore itself is hidden by default
        let mut expected_lines: Vec<&str> = expected.to_vec();
        expected_lines.sort();
        assert_eq!(lines, expected_lines);

        // Test showing hidden, depth 1
        let output_hidden = list_directory_contents(path.to_str().unwrap(), Some(1), true)?;
        let mut lines_hidden: Vec<&str> = output_hidden.lines().collect();
        lines_hidden.sort();
        // .gitignore should now be visible
        let expected_hidden = [".gitignore", "visible_dir/", "visible_file.txt"];
        let mut expected_lines_hidden: Vec<&str> = expected_hidden.to_vec();
        expected_lines_hidden.sort();
        assert_eq!(lines_hidden, expected_lines_hidden);


        // Test depth 2 (should not include contents of ignored_dir)
        let output_depth2 = list_directory_contents(path.to_str().unwrap(), Some(2), false)?;
        let mut lines_depth2: Vec<&str> = output_depth2.lines().collect();
        lines_depth2.sort();

        // Change to Vec<String> to own the formatted string
        let expected_depth2_paths: Vec<String> = vec![
             "visible_dir/".to_string(),
             format!("visible_dir{}sub_file.txt", std::path::MAIN_SEPARATOR), // format! creates owned String
             "visible_file.txt".to_string(),
        ];
        // Now map to &str for sorting and comparison
        let mut expected_lines_depth2 : Vec<&str> = expected_depth2_paths.iter().map(|s| s.as_str()).collect();
        expected_lines_depth2.sort();

        // Normalize paths in output for comparison
        let normalized_lines_depth2: Vec<String> = lines_depth2.iter().map(|s| s.replace('/', &std::path::MAIN_SEPARATOR.to_string())).collect();
        let mut normalized_lines_sorted_depth2: Vec<&str> = normalized_lines_depth2.iter().map(|s| s.as_str()).collect();
        normalized_lines_sorted_depth2.sort();


        assert_eq!(normalized_lines_sorted_depth2, expected_lines_depth2);

        Ok(())
    }
}

// volition-cli/src/tools/filesystem.rs
use anyhow::{anyhow, Result};
use ignore::WalkBuilder;
use std::path::Path;

// Removed unused ListDirectoryArgs import

/// Lists directory contents relative to a working directory, respecting .gitignore rules.
///
/// Args:
///     relative_path: The starting directory path, relative to working_dir.
///     max_depth: Maximum depth to traverse (None for unlimited, 0 for starting path only, 1 for contents, etc.).
///     show_hidden: Whether to include hidden files/directories.
///     working_dir: The base directory against which relative_path is resolved.
///
/// Returns:
///     A string containing a newline-separated list of relative paths (relative to the starting directory), or an error.
pub fn list_directory_contents(
    relative_path: &str,
    max_depth: Option<usize>,
    show_hidden: bool,
    working_dir: &Path, // Added working_dir
) -> Result<String> {
    // Resolve the starting path relative to the working directory
    let start_path = working_dir.join(relative_path);

    if !start_path.is_dir() {
        return Err(anyhow!(
            "Resolved path is not a directory: {:?}",
            start_path
        ));
    }
    tracing::debug!(
        "Listing directory contents for {:?} (relative path: {}), depth: {:?}, hidden: {}",
        start_path,
        relative_path,
        max_depth,
        show_hidden
    );

    let mut output = String::new();
    // Configure the WalkBuilder using the absolute start_path
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

                // Get the path relative to the absolute start_path
                match entry.path().strip_prefix(&start_path) { // Strip absolute start_path
                    Ok(path_relative_to_start) => {
                        if path_relative_to_start.as_os_str().is_empty() {
                            continue;
                        }
                        // Append the path relative to the start directory
                        output.push_str(&path_relative_to_start.display().to_string());
                        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                            output.push('/');
                        }
                        output.push('\n');
                    }
                    Err(_) => {
                        // Fallback: Use full path if stripping fails
                        output.push_str(&entry.path().display().to_string());
                         if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                            output.push('/');
                        }
                        output.push('\n');
                    }
                }
            }
            Err(err) => {
                output.push_str(&format!("WARN: Failed to access entry: {}\n", err));
            }
        }
    }

    Ok(output.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;
    use std::path::PathBuf;

    fn sort_lines(text: &str) -> Vec<&str> {
        let mut lines: Vec<&str> = text.lines().collect();
        lines.sort();
        lines
    }

    #[test]
    fn test_list_basic() -> Result<()> {
        let dir = tempdir()?;
        let working_dir = dir.path();
        File::create(working_dir.join("file1.txt"))?;
        fs::create_dir(working_dir.join("subdir"))?;
        File::create(working_dir.join("subdir/file2.txt"))?;

        // List relative to working_dir (".")
        let output = list_directory_contents(".", Some(1), false, working_dir)?;
        let expected = "file1.txt\nsubdir/";

        assert_eq!(sort_lines(&output), sort_lines(expected));
        Ok(())
    }

    #[test]
    fn test_list_subdir_relative() -> Result<()> {
        let dir = tempdir()?;
        let working_dir = dir.path();
        fs::create_dir(working_dir.join("outer"))?;
        File::create(working_dir.join("outer/file1.txt"))?;
        fs::create_dir(working_dir.join("outer/inner"))?;
        File::create(working_dir.join("outer/inner/file2.txt"))?;

        // List relative to "outer"
        let output = list_directory_contents("outer", Some(1), false, working_dir)?;
        let expected = "file1.txt\ninner/";

        assert_eq!(sort_lines(&output), sort_lines(expected));
        Ok(())
    }

    #[test]
    fn test_list_depth() -> Result<()> {
        let dir = tempdir()?;
        let working_dir = dir.path();
        File::create(working_dir.join("file1.txt"))?;
        fs::create_dir(working_dir.join("subdir"))?;
        File::create(working_dir.join("subdir/file2.txt"))?;

        let output = list_directory_contents(".", Some(2), false, working_dir)?;
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
        let working_dir = dir.path();
        File::create(working_dir.join(".hidden_file"))?;
        fs::create_dir(working_dir.join(".hidden_dir"))?;
        File::create(working_dir.join(".hidden_dir/file3.txt"))?;
        File::create(working_dir.join("visible_file.txt"))?;

        let output_no_hidden = list_directory_contents(".", Some(1), false, working_dir)?;
        assert_eq!(output_no_hidden.trim(), "visible_file.txt");

        let output_hidden = list_directory_contents(".", Some(1), true, working_dir)?;
        let expected_hidden = ".hidden_dir/\n.hidden_file\nvisible_file.txt";
        assert_eq!(sort_lines(&output_hidden), sort_lines(expected_hidden));

        let output_hidden_depth2 = list_directory_contents(".", Some(2), true, working_dir)?;
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
        let working_dir = dir.path();

        let gitignore_path = working_dir.join(".gitignore");
        let mut gitignore = File::create(&gitignore_path)?;
        writeln!(gitignore, "ignored_file.txt")?;
        writeln!(gitignore, "ignored_dir/")?;
        drop(gitignore);

        File::create(working_dir.join("visible_file.txt"))?;
        File::create(working_dir.join("ignored_file.txt"))?;
        fs::create_dir(working_dir.join("visible_dir"))?;
        File::create(working_dir.join("visible_dir/sub_file.txt"))?;
        fs::create_dir(working_dir.join("ignored_dir"))?;
        File::create(working_dir.join("ignored_dir/sub_ignored.txt"))?;

        let output = list_directory_contents(".", Some(1), false, working_dir)?;
        let expected = "visible_dir/\nvisible_file.txt";
        assert_eq!(sort_lines(&output), sort_lines(expected));

        let output_hidden = list_directory_contents(".", Some(1), true, working_dir)?;
        let expected_hidden = ".gitignore\nvisible_dir/\nvisible_file.txt";
        assert_eq!(sort_lines(&output_hidden), sort_lines(expected_hidden));

        let output_depth2 = list_directory_contents(".", Some(2), false, working_dir)?;
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
        let working_dir = dir.path();
        File::create(working_dir.join("file1.txt"))?;

        let output = list_directory_contents(".", Some(0), false, working_dir)?;
        assert_eq!(output, "");
        Ok(())
    }

    #[test]
    fn test_list_non_existent_path() {
        let dir = tempdir().unwrap();
        let working_dir = dir.path();
        let result = list_directory_contents("non_existent_subdir", Some(1), false, working_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Resolved path is not a directory"));
    }

    #[test]
    fn test_list_file_path() -> Result<()> {
        let dir = tempdir()?;
        let working_dir = dir.path();
        let file_path_relative = "file1.txt";
        File::create(working_dir.join(file_path_relative))?;

        let result = list_directory_contents(file_path_relative, Some(1), false, working_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Resolved path is not a directory"));
        Ok(())
    }
}

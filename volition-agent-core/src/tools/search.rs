// volition-agent-core/src/tools/search.rs

// Use the shell executor from the same core tools module
use super::shell::execute_shell_command;
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

// Real check using std::process::Command
#[cfg(not(test))]
fn check_ripgrep_installed() -> Result<()> {
    let command_name = "rg";
    let check_command = if cfg!(target_os = "windows") {
        format!("Get-Command {}", command_name)
    } else {
        format!("command -v {}", command_name)
    };

    let output = Command::new(if cfg!(target_os = "windows") { "powershell" } else { "sh" })
        .arg(if cfg!(target_os = "windows") { "-Command" } else { "-c" })
        .arg(&check_command)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "\'ripgrep\' (rg) command not found. Please install it and ensure it\'s in your PATH. It\'s required for search/definition tools.\nInstallation instructions: https://github.com/BurntSushi/ripgrep#installation"
        ))
    }
}

// Test mock version
#[cfg(test)]
fn check_ripgrep_installed() -> Result<()> {
    Ok(())
}

/// Searches for a text pattern using ripgrep.
pub async fn search_text(
    pattern: &str,
    search_path: Option<&str>,
    file_glob: Option<&str>,
    case_sensitive: Option<bool>,
    context_lines: Option<u32>,
    max_results: Option<usize>,
    working_dir: &Path,
) -> Result<String> {
    check_ripgrep_installed()?;

    let path_arg = search_path.unwrap_or(".");
    let glob_arg = file_glob.unwrap_or("*");
    let ignore_case_flag = !case_sensitive.unwrap_or(false);
    let context_arg = context_lines.unwrap_or(1);
    let max_lines = max_results.unwrap_or(50);

    info!(
        "Searching for pattern: '{}' in path: '{}' (glob: '{}', context: {}, ignore_case: {}) -> max {} lines",
        pattern, path_arg, glob_arg, context_arg, ignore_case_flag, max_lines
    );

    let context_str = context_arg.to_string();
    let mut rg_cmd_vec = vec![
        "rg",
        "--pretty",
        "--trim",
        "--context",
        &context_str,
        "--glob",
        glob_arg,
    ];

    if ignore_case_flag {
        rg_cmd_vec.push("--ignore-case");
    }

    rg_cmd_vec.push(pattern);
    rg_cmd_vec.push(path_arg);

    let rg_cmd_base = rg_cmd_vec
        .iter()
        .map(|s| format!("'{}'", s.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");

    let full_cmd = format!("{} | head -n {}", rg_cmd_base, max_lines);

    debug!("Executing search command: {}", full_cmd);

    // Use the public shell executor from this crate
    let result = execute_shell_command(&full_cmd, working_dir).await?;

    // Check if the result indicates no matches found
    // Need to parse the output format of execute_shell_command
    let no_stdout = result.contains("\nStdout:\n<no output>");
    // Consider checking stderr too, or exit status if needed

    if no_stdout {
        Ok(format!(
            "No matches found for pattern: '{}' in path: '{}' matching glob: '{}'",
            pattern, path_arg, glob_arg
        ))
    } else {
        // Return the full formatted result from execute_shell_command
        Ok(result)
    }
}

/// Finds potential Rust definition sites using ripgrep.
pub async fn find_rust_definition(
    symbol: &str,
    search_path: Option<&str>,
    working_dir: &Path,
) -> Result<String> {
    check_ripgrep_installed()?;

    let directory_arg = search_path.unwrap_or(".");

    info!(
        "Finding Rust definition for symbol: {} in directory: {}",
        symbol,
        directory_arg
    );

    let file_pattern = "*.rs";
    let escaped_symbol = regex::escape(symbol);
    let pattern = format!(
        r"^(?:pub\s+)?(?:unsafe\s+)?(?:async\s+)?(fn|struct|enum|trait|const|static|type|mod|impl|macro_rules!)\s+{}\\b",
        escaped_symbol
    );

    let rg_cmd_vec = vec![
        "rg",
        "--pretty",
        "--trim",
        "--glob",
        file_pattern,
        "--ignore-case",
        "--max-count=10",
        "-e",
        &pattern,
        directory_arg,
    ];

    let full_cmd = rg_cmd_vec
        .iter()
        .map(|s| format!("'{}'", s.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");

    debug!("Executing find rust definition command: {}", full_cmd);

    // Use the public shell executor from this crate
    let result = execute_shell_command(&full_cmd, working_dir).await?;

    // Check result based on execute_shell_command output format
    let no_stdout = result.contains("\nStdout:\n<no output>");

    if no_stdout {
        Ok(format!("No Rust definition found for symbol: {}", symbol))
    } else {
        // Return the full formatted result
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio;

    fn test_working_dir() -> PathBuf {
        tempdir().map(|d| d.into_path()).unwrap_or_default()
    }

    #[tokio::test]
    async fn test_check_ripgrep_installed_mock() {
        assert!(check_ripgrep_installed().is_ok());
    }

    // Mock execute_shell_command for search tests
    async fn mock_shell_executor_for_search(cmd: &str, _wd: &Path) -> Result<String> {
        println!("[TEST] Mock shell executor called with: {}", cmd);
        if cmd.contains("rg") && cmd.contains("no_match_pattern") {
             Ok("Command executed: ...\nStatus: 1\nStdout:\n<no output>\nStderr:\n<no output>".to_string())
        } else if cmd.contains("rg") && cmd.contains("find_this_symbol") {
             Ok("Command executed: ...\nStatus: 0\nStdout:\nsrc/lib.rs:10:1:pub fn find_this_symbol() {}\nStderr:\n<no output>".to_string())
        } else {
             Ok("Command executed: ...\nStatus: 0\nStdout:\nMock search results\nStderr:\n<no output>".to_string())
        }
    }

    #[tokio::test]
    async fn test_search_text_no_matches() {
        let pattern = "no_match_pattern";
        let working_dir = test_working_dir();
        // Override the actual shell executor with our mock for this test scope
        async fn execute_shell_command(cmd: &str, wd: &Path) -> Result<String> { mock_shell_executor_for_search(cmd, wd).await }

        let result = search_text(pattern, None, None, None, None, None, &working_dir).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No matches found"));
    }

     #[tokio::test]
    async fn test_find_rust_definition_found() {
        let symbol = "find_this_symbol";
        let working_dir = test_working_dir();
        // Override the actual shell executor with our mock for this test scope
        async fn execute_shell_command(cmd: &str, wd: &Path) -> Result<String> { mock_shell_executor_for_search(cmd, wd).await }

        let result = find_rust_definition(symbol, None, &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        // Check that the output is the *formatted* output from the mock executor
        assert!(output.contains("Command executed:"));
        assert!(output.contains("src/lib.rs:10:1:pub fn find_this_symbol"));
    }
}

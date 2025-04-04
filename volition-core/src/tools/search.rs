// volition-agent-core/src/tools/search.rs

use super::CommandOutput;
use super::shell::execute_shell_command;
use crate::utils::truncate_string; // <-- Import the helper
use anyhow::Result;
use std::path::Path;
use tracing::{debug, info};

#[cfg(not(test))]
fn check_ripgrep_installed() -> Result<()> {
    use std::process::Command;
    let command_name = "rg";
    let check_command = if cfg!(target_os = "windows") {
        format!("Get-Command {}", command_name)
    } else {
        format!("command -v {}", command_name)
    };
    let output = Command::new(if cfg!(target_os = "windows") {
        "powershell"
    } else {
        "sh"
    })
    .arg(if cfg!(target_os = "windows") {
        "-Command"
    } else {
        "-c"
    })
    .arg(&check_command)
    .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "\'ripgrep\' (rg) command not found. Please install it and ensure it\'s in your PATH. It\'s required for search/definition tools.\nInstallation instructions: https://github.com/BurntSushi/ripgrep#installation"
        ))
    }
}

#[cfg(test)]
fn check_ripgrep_installed() -> Result<()> {
    Ok(())
}

/// Searches for a text pattern using ripgrep.
/// Returns the raw stdout on success, or a specific "No matches found" message.
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

    // Truncate pattern for logging
    let pattern_display = truncate_string(pattern, 60);

    info!(
        "Searching for pattern: '{}' in path: '{}' (glob: '{}', context: {}, ignore_case: {}) -> max {} lines",
        pattern_display, // <-- Use truncated version
        path_arg,
        glob_arg,
        context_arg,
        ignore_case_flag,
        max_lines
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
    rg_cmd_vec.push(pattern); // Use original pattern for command
    rg_cmd_vec.push(path_arg);

    let mut rg_cmd_parts = Vec::new();
    for arg in rg_cmd_vec.iter() {
        if *arg == pattern || *arg == path_arg {
            rg_cmd_parts.push(arg.to_string());
        } else {
            rg_cmd_parts.push(format!("'{}'", arg.replace('\'', "'\\''")));
        }
    }
    let rg_cmd_base = rg_cmd_parts.join(" ");

    let full_cmd = format!("{} | head -n {}", rg_cmd_base, max_lines);

    debug!("Executing search command via shell: {}", full_cmd);

    let cmd_output: CommandOutput = execute_shell_command(&full_cmd, working_dir).await?;

    let no_match_status = cmd_output.status == 1;
    let no_stdout = cmd_output.stdout.trim().is_empty();

    if no_match_status || no_stdout {
        Ok(format!(
            "No matches found for pattern: '{}' in path: '{}' matching glob: '{}'",
            pattern, path_arg, glob_arg
        ))
    } else {
        Ok(cmd_output.stdout.trim().to_string())
    }
}

/// Finds potential Rust definition sites using ripgrep.
/// Returns the raw stdout on success, or a specific "No definition found" message.
pub async fn find_rust_definition(
    symbol: &str,
    search_path: Option<&str>,
    working_dir: &Path,
) -> Result<String> {
    check_ripgrep_installed()?;

    let directory_or_file_arg = search_path.unwrap_or(".");
    let is_dir = working_dir.join(directory_or_file_arg).is_dir();

    // Truncate symbol for logging
    let symbol_display = truncate_string(symbol, 60);

    info!(
        "Finding Rust definition for symbol: {} in path: {} (is_dir: {})",
        symbol_display, // <-- Use truncated version
        directory_or_file_arg,
        is_dir
    );

    let file_pattern = "*.rs";
    let escaped_symbol = regex::escape(symbol);
    let pattern = format!(
        r"(?:pub\s+)?(?:unsafe\s+)?(?:async\s+)?(fn|struct|enum|trait|const|static|type|mod|impl|macro_rules!)\s+{}\\b",
        escaped_symbol
    );

    let mut command_parts = vec!["rg".to_string()];
    command_parts.push("--trim".to_string());
    if is_dir {
        command_parts.push("--glob".to_string());
        command_parts.push(file_pattern.to_string());
    }
    command_parts.push("--ignore-case".to_string());
    command_parts.push("--max-count=10".to_string());
    command_parts.push("-e".to_string());
    command_parts.push(format!("'{}'", pattern.replace('\'', "'\\''")));
    command_parts.push(directory_or_file_arg.to_string());

    let full_cmd = command_parts.join(" ");

    debug!(
        "Executing find rust definition command via shell: {}",
        full_cmd
    );

    let cmd_output: CommandOutput = execute_shell_command(&full_cmd, working_dir).await?;

    let no_match_status = cmd_output.status == 1;
    let no_stdout = cmd_output.stdout.trim().is_empty();

    if no_match_status || no_stdout {
        Ok(format!("No Rust definition found for symbol: {}", symbol))
    } else {
        Ok(cmd_output.stdout.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio;

    fn test_working_dir() -> PathBuf {
        tempdir().expect("Failed to create temp dir").into_path()
    }

    #[tokio::test]
    async fn test_check_ripgrep_installed_mock() {
        assert!(check_ripgrep_installed().is_ok());
    }

    #[tokio::test]
    async fn test_search_text_no_matches() {
        let pattern = "pattern_that_will_not_match_in_a_million_years";
        let working_dir = test_working_dir();
        fs::write(working_dir.join("dummy.txt"), "content").unwrap();
        let result = search_text(pattern, None, None, None, None, None, &working_dir).await;
        assert!(result.is_ok());
        let output_str = result.unwrap();
        println!("search_text_no_matches output:\n{}", output_str);
        assert!(
            output_str.contains("No matches found"),
            "Output should indicate no matches were found"
        );
    }

    #[tokio::test]
    #[ignore = "Relies on external rg command and shell execution details"]
    async fn test_find_rust_definition_found_in_test_file() -> Result<()> {
        let symbol = "find_this_test_fn_abc";
        let working_dir = test_working_dir();
        let test_file_name = "test_src_find_def.rs";
        let test_file_path = working_dir.join(test_file_name);
        let file_content = format!(
            "\n  // Some comment\npub fn {}() {{\n    println!(\"Found!\");\n}}\n",
            symbol
        );
        fs::write(&test_file_path, file_content)?;

        let result = find_rust_definition(symbol, None, &working_dir).await;

        assert!(
            result.is_ok(),
            "find_rust_definition failed: {:?}",
            result.err()
        );
        let output_str = result.unwrap();
        println!("find_rust_definition output:\n{}", output_str);

        let expected_line = format!("pub fn {}()", symbol);
        assert!(
            output_str.contains(&expected_line),
            "Output did not contain function signature"
        );
        assert!(
            !output_str.contains("No Rust definition found"),
            "Output incorrectly stated no definition found"
        );
        Ok(())
    }
}

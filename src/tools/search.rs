// This file now contains the implementation for the search_text tool
// and the find_rust_definition tool.

use crate::models::tools::{FindRustDefinitionArgs, SearchTextArgs}; // Updated import
use crate::tools::shell::execute_shell_command_internal; // Use internal executor
use anyhow::{anyhow, Result};
use std::process::Command;

// Check if ripgrep (rg) is installed and available in PATH
fn check_ripgrep_installed() -> Result<()> {
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
        Err(anyhow!(
            "'ripgrep' (rg) command not found. Please install it and ensure it's in your PATH. It's required for search/definition tools.\nInstallation instructions: https://github.com/BurntSushi/ripgrep#installation"
        ))
    }
}

// search_text implementation (unchanged)
pub async fn search_text(args: SearchTextArgs) -> Result<String> {
    check_ripgrep_installed()?;

    let pattern = &args.pattern;
    let path = args.path.as_deref().unwrap_or(".");
    let file_glob = args.file_glob.as_deref().unwrap_or("*");
    let case_sensitive = args.case_sensitive.unwrap_or(false);
    let context_lines = args.context_lines.unwrap_or(1);
    let max_results = args.max_results.unwrap_or(50);

    tracing::info!(
        "Searching for pattern: '{}' in path: '{}' within files matching glob: '{}' (context: {}, case_sensitive: {}) -> max {} lines",
        pattern, path, file_glob, context_lines, case_sensitive, max_results
    );

    let mut rg_cmd_parts = vec![
        "rg".to_string(),
        "--pretty".to_string(),
        "--trim".to_string(),
        format!("--context={}", context_lines),
        format!("--glob='{}'", file_glob),
    ];

    if !case_sensitive {
        rg_cmd_parts.push("--ignore-case".to_string());
    }

    rg_cmd_parts.push(format!("'{}'", pattern));
    rg_cmd_parts.push(path.to_string());

    let rg_cmd = format!("{} | head -n {}", rg_cmd_parts.join(" "), max_results);

    tracing::debug!("Executing search command: {}", rg_cmd);

    let result = execute_shell_command_internal(&rg_cmd).await?;

    if result.is_empty()
        || result.contains("Command executed successfully with no output")
        || result.contains("Shell command execution denied")
    {
        Ok(format!(
            "No matches found for pattern: '{}' in path: '{}' matching glob: '{}'",
            pattern, path, file_glob
        ))
    } else {
        Ok(format!(
            "Search results (format: path:line_number:content):\n{}",
            result
        ))
    }
}

// Renamed and simplified find_definition to find_rust_definition
pub async fn find_rust_definition(args: FindRustDefinitionArgs) -> Result<String> {
    // Updated args type
    // Check if rg is installed before proceeding (required for this tool now)
    check_ripgrep_installed()?;

    let symbol = &args.symbol;
    let directory = args.path.as_deref().unwrap_or(".");

    tracing::info!(
        "Finding Rust definition for symbol: {} in directory: {}", // Updated log message
        symbol,
        directory
    );

    // Hardcoded Rust file pattern and regex
    let file_pattern = "*.rs";
    let pattern = format!(
        r"(fn|struct|enum|trait|const|static|type|mod|impl|macro_rules!\s+){}[\s<(:{{]", // Escaped { -> {{
        symbol
    );

    // Build the ripgrep command
    let rg_cmd = format!(
        // Use --type rust for more specific filtering if desired, but glob works
        "rg --pretty --trim --glob='{}' --ignore-case --max-count=10 \"{}\" {}",
        file_pattern,
        pattern,   // Regex pattern
        directory  // Directory to search
    );

    tracing::debug!("Executing find rust definition command: {}", rg_cmd);

    // Execute the search command
    let result = execute_shell_command_internal(&rg_cmd).await?;

    if result.is_empty()
        || result.contains("Command executed successfully with no output")
        || result.contains("Shell command execution denied")
    {
        Ok(format!("No Rust definition found for symbol: {}", symbol))
    } else {
        Ok(result)
    }
}

// This file now contains the implementation for the search_text tool
// and the find_rust_definition tool.

use crate::models::tools::{FindRustDefinitionArgs, SearchTextArgs};
use crate::tools::shell::execute_shell_command_internal;
use anyhow::Result; // Only Result needed by both

// Imports only needed for the non-test version
#[cfg(not(test))]
use {
    anyhow::anyhow, // anyhow macro only used in non-test version
    std::process::Command,
};

// Real check using std::process::Command
#[cfg(not(test))]
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
        Err(anyhow!( // Use anyhow macro here
            "'ripgrep' (rg) command not found. Please install it and ensure it's in your PATH. It's required for search/definition tools.\nInstallation instructions: https://github.com/BurntSushi/ripgrep#installation"
        ))
    }
}

// Test mock version - assume rg is always installed
#[cfg(test)]
fn check_ripgrep_installed() -> Result<()> {
     println!("[TEST] Mock check_ripgrep_installed called - assuming OK");
     Ok(())
}

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

    // Construct command that pipes rg output to head for limiting results
    let rg_cmd = format!("{} | head -n {}", rg_cmd_parts.join(" "), max_results);

    tracing::debug!("Executing search command: {}", rg_cmd);

    // This call will use the appropriate (real or mock) version of execute_shell_command_internal
    let result = execute_shell_command_internal(&rg_cmd).await?;

    // Check if the result indicates no matches found (based on mock output or real rg behavior)
    if result.is_empty() // Check for genuinely empty output
       || result.starts_with("Command executed") && result.contains("Stdout:\n<no output>") // Check mock/real output indicating no stdout
       // Add other checks if needed, e.g., specific exit codes if execute_shell_command_internal provides them clearly
    {
        Ok(format!(
            "No matches found for pattern: '{}' in path: '{}' matching glob: '{}'",
            pattern, path, file_glob
        ))
    } else {
        // Assume the result string already contains the formatted output from execute_shell_command_internal
        Ok(format!(
            "Search results (details included below):\n{}",
            result
        ))
    }
}

pub async fn find_rust_definition(args: FindRustDefinitionArgs) -> Result<String> {
    check_ripgrep_installed()?;

    let symbol = &args.symbol;
    let directory = args.path.as_deref().unwrap_or(".");

    tracing::info!(
        "Finding Rust definition for symbol: {} in directory: {}",
        symbol,
        directory
    );

    let file_pattern = "*.rs";
    // Updated regex to be slightly more robust for different definition styles
    let pattern = format!(
        r"^(?:pub\s+)?(?:unsafe\s+)?(?:async\s+)?(fn|struct|enum|trait|const|static|type|mod|impl|macro_rules!)\s+{}\\b",
        regex::escape(symbol) // Escape symbol for regex safety
    );

    let rg_cmd = format!(
        "rg --pretty --trim --glob='{}' --ignore-case --max-count=10 -e \"{}\" {}",
        file_pattern,
        pattern,   // Use -e for pattern to treat it as regex explicitly
        directory
    );

    tracing::debug!("Executing find rust definition command: {}", rg_cmd);

    let result = execute_shell_command_internal(&rg_cmd).await?;

     if result.is_empty() // Check for genuinely empty output
       || result.starts_with("Command executed") && result.contains("Stdout:\n<no output>") // Check mock/real output indicating no stdout
    {
        Ok(format!("No Rust definition found for symbol: {}", symbol))
    } else {
        // Assume the result string already contains the formatted output from execute_shell_command_internal
        Ok(format!(
            "Potential definition(s) found (details included below):\n{}",
            result
        ))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_check_ripgrep_installed_mock() {
        // This test just ensures the mock function compiles and returns Ok
        let result = check_ripgrep_installed();
        assert!(result.is_ok());
    }

    // NOTE: Deferring detailed tests for search_text and find_rust_definition command construction
    // and output formatting, as they require better mocking of the shared
    // execute_shell_command_internal function (e.g., using mockall) to verify inputs
    // and control outputs effectively across modules.
    // Current tests rely on the simple #[cfg(test)] mock in shell.rs.
}

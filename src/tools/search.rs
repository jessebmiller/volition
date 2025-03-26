// This file now contains the implementation for the search_text tool
// and the find_definition tool.

use crate::models::tools::{SearchTextArgs, FindDefinitionArgs, ShellArgs};
use crate::tools::shell::run_shell_command;
use anyhow::{Result, anyhow};
use std::process::Command;

// Check if ripgrep (rg) is installed and available in PATH
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
            "'ripgrep' (rg) command not found. Please install it and ensure it's in your PATH. It's required for the search_text tool.
Installation instructions: https://github.com/BurntSushi/ripgrep#installation"
        ))
    }
}

// New search_text implementation using ripgrep
pub async fn search_text(args: SearchTextArgs) -> Result<String> {
    // Check if rg is installed before proceeding
    check_ripgrep_installed()?;

    let pattern = &args.pattern;
    let path = args.path.as_deref().unwrap_or(".");
    let file_glob = args.file_glob.as_deref().unwrap_or("*");
    let case_sensitive = args.case_sensitive.unwrap_or(false);
    let context_lines = args.context_lines.unwrap_or(1); // Default context lines = 1
    let max_results = args.max_results.unwrap_or(50); // Default max lines = 50

    tracing::info!(
        "Searching for pattern: '{}' in path: '{}' within files matching glob: '{}' (context: {}, case_sensitive: {}) -> max {} lines",
        pattern, path, file_glob, context_lines, case_sensitive, max_results
    );

    // Build the ripgrep command
    // We use `rg --json` initially to get structured output, then fall back to text if needed.
    // Using --trim to remove leading whitespace which can confuse LLMs.
    // Using --pretty to add file names and line numbers.
    let mut rg_cmd_parts = vec![
        "rg".to_string(),
        "--pretty".to_string(), // Enables file names and line numbers
        "--trim".to_string(),
        format!("--context={}", context_lines),
        // Note: Using --glob requires forward slashes even on Windows.
        format!("--glob='{}'", file_glob), // Use single quotes for safety
    ];

    if !case_sensitive {
        rg_cmd_parts.push("--ignore-case".to_string());
    }

    // Add pattern and path (must be last)
    rg_cmd_parts.push(format!("'{}'", pattern)); // Quote pattern for safety
    rg_cmd_parts.push(path.to_string());

    // Limit results using head (simpler than parsing rg output count)
    // Note: `head` might cut off mid-context block, but it's a simple way to limit output size.
    let rg_cmd = format!("{} | head -n {}", rg_cmd_parts.join(" "), max_results);

    tracing::debug!("Executing search command: {}", rg_cmd);

    // Execute the search command
    let shell_args = ShellArgs { command: rg_cmd };
    let result = run_shell_command(shell_args).await?;

    if result.is_empty() || result.contains("Command executed successfully with no output") {
        Ok(format!("No matches found for pattern: '{}' in path: '{}' matching glob: '{}'", pattern, path, file_glob))
    } else {
        // Prepend a note about the output format
        Ok(format!(
            "Search results (format: path:line_number:content):
{}",
            result
        ))
    }
}

// Keeping find_definition for now
pub async fn find_definition(args: FindDefinitionArgs) -> Result<String> {
    let symbol = &args.symbol;
    let directory = args.path.as_deref().unwrap_or(".");

    tracing::info!("Finding definition for symbol: {} in directory: {}", symbol, directory);

    // Determine language-specific search patterns
    let (file_pattern, pattern) = match args.language.as_deref() {
        Some("rust") => ("*.rs", format!(r"(fn|struct|enum|trait|const|static|type)\s+{}[\s<(]", symbol)),
        Some("javascript") | Some("js") => ("*.{js,jsx,ts,tsx}", format!(r"(function|class|const|let|var)\s+{}[\s(=]", symbol)),
        Some("python") | Some("py") => ("*.py", format!(r"(def|class)\s+{}[\s(:]", symbol)),
        Some("go") => ("*.go", format!(r"(func|type|var|const)\s+{}[\s(]", symbol)),
        Some("java") | Some("kotlin") => ("*.{java,kt}", format!(r"(class|interface|enum|[a-zA-Z0-9]+\s+[a-zA-Z0-9]+)\s+{}[\s<(]", symbol)),
        Some("c") | Some("cpp") | Some("c++") => ("*.{c,cpp,h,hpp}", format!(r"([a-zA-Z0-9_]+\s+{}\s*\([^)]*\)|class\s+{})", symbol, symbol)),
        _ => ("*", symbol.to_string()),
    };

    // Build the search command based on the OS
    // TODO: Consider using ripgrep here too for consistency and potentially better results?
    let search_cmd = if cfg!(target_os = "windows") {
        format!(
            "powershell -Command \"Get-ChildItem -Path {} -Recurse -File -Include {} | Select-String -Pattern '{}' | Select-Object -First 10\"",
            directory, file_pattern, pattern
        )
    } else {
        format!(
            "find {} -type f -name \"{}\" -not -path \"*/\\.*\" -not -path \"*/node_modules/*\" -not -path \"*/target/*\" | xargs grep -l \"{}\" | xargs grep -n \"{}\" | head -10",
            directory, file_pattern, symbol, pattern
        )
    };

    tracing::debug!("Executing find definition command: {}", search_cmd);

    // Execute the search command
    let shell_args = ShellArgs { command: search_cmd };
    let result = run_shell_command(shell_args).await?;

    if result.is_empty() || result.contains("Command executed successfully with no output") {
        Ok(format!("No definition found for symbol: {}", symbol))
    } else {
        Ok(result)
    }
}

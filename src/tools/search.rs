// This file now contains the implementation for the search_text tool
// and the find_definition tool.

// Removed unused RuntimeConfig import
use crate::models::tools::{SearchTextArgs, FindDefinitionArgs};
use crate::tools::shell::execute_shell_command_internal; // Use internal executor
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
            "'ripgrep' (rg) command not found. Please install it and ensure it's in your PATH. It's required for the search_text tool.\nInstallation instructions: https://github.com/BurntSushi/ripgrep#installation"
        ))
    }
}

// New search_text implementation using ripgrep
// Removed unused config argument
pub async fn search_text(args: SearchTextArgs) -> Result<String> { // Removed config
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
    let mut rg_cmd_parts = vec![
        "rg".to_string(),
        "--pretty".to_string(), // Enables file names and line numbers
        "--trim".to_string(),
        format!("--context={}", context_lines),
        format!("--glob='{}'", file_glob), // Use single quotes for safety
    ];

    if !case_sensitive {
        rg_cmd_parts.push("--ignore-case".to_string());
    }

    // Add pattern and path (must be last)
    rg_cmd_parts.push(format!("'{}'", pattern)); // Quote pattern for safety
    rg_cmd_parts.push(path.to_string());

    // Limit results using head
    let rg_cmd = format!("{} | head -n {}", rg_cmd_parts.join(" "), max_results);

    tracing::debug!("Executing search command: {}", rg_cmd);

    // Execute the search command
    let result = execute_shell_command_internal(&rg_cmd).await?; // Pass command string directly

    if result.is_empty() || result.contains("Command executed successfully with no output") || result.contains("Shell command execution denied") {
        Ok(format!("No matches found for pattern: '{}' in path: '{}' matching glob: '{}'", pattern, path, file_glob))
    } else {
        Ok(format!(
            "Search results (format: path:line_number:content):\n{}",
            result
        ))
    }
}

// Removed unused config argument
pub async fn find_definition(args: FindDefinitionArgs) -> Result<String> { // Removed config
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
    let result = execute_shell_command_internal(&search_cmd).await?; // Pass command string directly

    if result.is_empty() || result.contains("Command executed successfully with no output") || result.contains("Shell command execution denied") {
        Ok(format!("No definition found for symbol: {}", symbol))
    } else {
        Ok(result)
    }
}

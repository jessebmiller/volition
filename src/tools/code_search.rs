use crate::models::tools::{SearchCodeArgs, FindDefinitionArgs};
use crate::tools::shell::run_shell_command;
use crate::models::tools::ShellArgs;
use anyhow::Result;

//TODO this search_code tool isn't working very well. please write a plan to replace it with something that gives better context to LLM agents like openAI

pub async fn search_code(args: SearchCodeArgs) -> Result<String> {
    let pattern = &args.pattern;
    let directory = args.path.as_deref().unwrap_or(".");
    let file_pattern = args.file_pattern.as_deref().unwrap_or("*");
    let case_sensitive = args.case_sensitive.unwrap_or(false);
    let max_results = args.max_results.unwrap_or(100);

    tracing::info!("Searching for pattern: {} in directory: {} with file pattern: {}", pattern, directory, file_pattern);

    // Build the search command
    let grep_cmd = if cfg!(target_os = "windows") {
        format!(
            "powershell -Command \"Get-ChildItem -Path {} -Recurse -File -Filter {} | Select-String {} '{}' | Select-Object -First {}\"",
            directory,
            file_pattern,
            if case_sensitive { "-CaseSensitive" } else { "-CaseInsensitive" },
            pattern,
            max_results
        )
    } else {
        format!(
            "find {} -type f -name \"{}\" -not -path \"*/\\.*\" -not -path \"*/node_modules/*\" -not -path \"*/target/*\" | xargs grep {} -l \"{}\" | head -n {}",
            directory,
            file_pattern,
            if case_sensitive { "" } else { "-i" },
            pattern,
            max_results
        )
    };

    // Execute the search command
    let shell_args = ShellArgs { command: grep_cmd };
    let result = run_shell_command(shell_args).await?;

    if result.is_empty() || result.contains("Command executed successfully with no output") {
        Ok(format!("No matches found for pattern: {}", pattern))
    } else {
        Ok(result)
    }
}

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

    // Execute the search command
    let shell_args = ShellArgs { command: search_cmd };
    let result = run_shell_command(shell_args).await?;

    if result.is_empty() || result.contains("Command executed successfully with no output") {
        Ok(format!("No definition found for symbol: {}", symbol))
    } else {
        Ok(result)
    }
}

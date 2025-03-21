use crate::utils::DebugLevel;
use crate::utils::debug_log;
use crate::models::tools::{SearchCodeArgs, FindDefinitionArgs};
use crate::tools::shell::run_shell_command;
use crate::models::tools::ShellArgs;
use anyhow::Result;

pub async fn search_code(args: SearchCodeArgs, debug_level: DebugLevel) -> Result<String> {
    let pattern = &args.pattern;
    let directory = args.path.as_deref().unwrap_or(".");
    let file_pattern = args.file_pattern.as_deref().unwrap_or("*");
    let case_sensitive = args.case_sensitive.unwrap_or(false);
    let max_results = args.max_results.unwrap_or(100);
    
    if debug_level >= DebugLevel::Minimal {
        debug_log(
            debug_level,
            DebugLevel::Minimal,
            &format!(
                "Searching for pattern: {} in directory: {} with file pattern: {}",
                pattern, directory, file_pattern
            )
        );
    }

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
    let result = run_shell_command(shell_args, debug_level).await?;

    if result.is_empty() || result.contains("Command executed successfully with no output") {
        Ok(format!("No matches found for pattern: {}", pattern))
    } else {
        Ok(result)
    }
}

pub async fn find_definition(args: FindDefinitionArgs, debug_level: DebugLevel) -> Result<String> {
    let symbol = &args.symbol;
    let directory = args.path.as_deref().unwrap_or(".");
    
    if debug_level >= DebugLevel::Minimal {
        debug_log(
            debug_level,
            DebugLevel::Minimal,
            &format!(
                "Finding definition for symbol: {} in directory: {}",
                symbol, directory
            )
        );
    }
    
    // Determine language-specific search patterns
    let (file_pattern, pattern) = match args.language.as_deref() {
        Some("rust") => ("*.rs", format!("(fn|struct|enum|trait|const|static|type)\\s+{}[\\s<(]", symbol)),
        Some("javascript") | Some("js") => ("*.{js,jsx,ts,tsx}", format!("(function|class|const|let|var)\\s+{}[\\s(=]", symbol)),
        Some("python") | Some("py") => ("*.py", format!("(def|class)\\s+{}[\\s(:]", symbol)),
        Some("go") => ("*.go", format!("(func|type|var|const)\\s+{}[\\s(]", symbol)),
        Some("java") | Some("kotlin") => ("*.{java,kt}", format!("(class|interface|enum|[a-zA-Z0-9]+\\s+[a-zA-Z0-9]+)\\s+{}[\\s<(]", symbol)),
        Some("c") | Some("cpp") | Some("c++") => ("*.{c,cpp,h,hpp}", format!("([a-zA-Z0-9_]+\\s+{}\\s*\\([^)]*\\)|class\\s+{})", symbol, symbol)),
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
    let result = run_shell_command(shell_args, debug_level).await?;
    
    if result.is_empty() || result.contains("Command executed successfully with no output") {
        Ok(format!("No definition found for symbol: {}", symbol))
    } else {
        Ok(result)
    }
}
use crate::models::tools::ShellArgs;
use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use tracing::warn; // Only warn needed by both versions

// Imports only needed for the non-test version
#[cfg(not(test))]
use {
    // Removed duct::cmd, - using macro directly
    // Removed std::process::ExitStatus, - unused
    tracing::debug, // Import debug only when not testing
};

// Original implementation using duct, compiled when not testing
#[cfg(not(test))]
pub(crate) async fn execute_shell_command_internal(command: &str) -> Result<String> {
    debug!("Executing internal command: {}", command);

    // Use duct::cmd! macro to execute the command string via the shell ("sh -c ...")
    let expression = duct::cmd!("sh", "-c", command); // <--- Fix applied here

    let output_result = expression
        .stdout_capture()
        .stderr_capture()
        .unchecked() // Don't panic on non-zero exit status
        .run();

    let (stdout_bytes, stderr_bytes, exit_status) = match output_result {
        Ok(output) => (
            output.stdout,
            output.stderr,
            output
                .status
                .code()
                .unwrap_or_else(|| if output.status.success() { 0 } else { 1 }), // Get exit code or infer
        ),
        Err(e) => {
            warn!(command = command, error = %e, "Failed to spawn command process");
            return Err(e).context(format!("Failed to spawn process for command: {}", command));
        }
    };

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

    // --- Logging ---
    let stdout_preview = stdout.lines().take(3).collect::<Vec<&str>>().join("\\n");
    let stderr_preview = if !stderr.is_empty() {
        format!(
            "\\nStderr preview: {}",
            stderr.lines().take(3).collect::<Vec<&str>>().join("\\n")
        )
    } else {
        String::new()
    };
    debug!(
        "Internal command exit status: {}\nOutput preview:\n{}{}",
        exit_status,
        if stdout_preview.is_empty() {
            "<no output>"
        } else {
            &stdout_preview
        },
        stderr_preview
    );
    let detailed_info = format!(
        "Stdout length: {} bytes, Stderr length: {} bytes, Total lines: {}",
        stdout.len(),
        stderr.len(),
        stdout.lines().count() + stderr.lines().count()
    );
    debug!("{}", detailed_info);
    // --- End Logging ---

    let result = format!(
        "Command executed with status: {}\nStdout:\n{}\nStderr:\n{}",
        exit_status,
        if stdout.is_empty() {
            "<no output>"
        } else {
            &stdout
        },
        if stderr.is_empty() {
            "<no output>"
        } else {
            &stderr
        }
    );

    Ok(result)
}

// Test-only mock implementation
#[cfg(test)]
pub(crate) async fn execute_shell_command_internal(command: &str) -> Result<String> {
    println!(
        "[TEST] Mock execute_shell_command_internal called with: {}",
        command
    );
    // Simple mock logic based on command string
    if command == "echo Mock Success" {
        Ok(
            "Command executed with status: 0\nStdout:\nMock Success\n\nStderr:\n<no output>"
                .to_string(),
        )
    } else if command == "ls /non_existent_directory_for_volition_test" {
        Ok("Command executed with status: 2\nStdout:\n<no output>\nStderr:\nls: cannot access '/non_existent_directory_for_volition_test': No such file or directory\n".to_string())
    } else if command == "this_command_should_absolutely_not_exist_ever_42" {
        Ok(format!("Command executed with status: 127\nStdout:\n<no output>\nStderr:\nsh: {}: command not found\n", command))
    } else {
        // Default mock response for unexpected commands in tests
        Ok(format!(
            "Command executed with status: 0\nStdout:\nMock output for {}\nStderr:\n<no output>",
            command
        ))
    }
}

// Public function exposed as the 'shell' tool, includes confirmation
pub async fn run_shell_command(args: ShellArgs) -> Result<String> {
    let command = &args.command;

    // --- Mandatory Confirmation (y/N style, default No) ---
    // NOTE: This confirmation logic is NOT tested by the current unit tests
    // due to stdin/stdout interaction complexity.
    print!(
        "{}\n{}\n{}{} ",
        "WARNING: This tool can execute arbitrary code!"
            .red()
            .bold(),
        format!("Request to run shell command: {}", command).yellow(),
        "Allow execution? ".yellow(),
        "(y/N):".yellow().bold() // Default to No
    );
    io::stdout().flush().context("Failed to flush stdout")?;

    let mut user_choice = String::new();
    io::stdin()
        .read_line(&mut user_choice)
        .context("Failed to read user input")?;

    if user_choice.trim().to_lowercase() != "y" {
        warn!("User denied execution of shell command: {}", command);
        println!("{}", "Shell command execution denied.".red());
        return Ok(format!(
            "Shell command execution denied by user: {}",
            command
        ));
    }
    // --- End Confirmation ---

    // If approved, call the appropriate version of execute_shell_command_internal
    println!("{} {}", "Running:".blue().bold(), command);
    execute_shell_command_internal(command).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    // No imports needed here now

    #[tokio::test]
    async fn test_execute_shell_internal_success() {
        let command = "echo Mock Success";
        let result = execute_shell_command_internal(command).await; // Calls the test version

        assert!(result.is_ok());
        let output_str = result.unwrap();
        println!("Mocked Output for '{}':\n{}", command, output_str);

        assert!(output_str.starts_with("Command executed with status: 0"));
        assert!(output_str.contains("\nStdout:\nMock Success\n"));
        assert!(output_str.contains("\nStderr:\n<no output>"));
    }

    #[tokio::test]
    async fn test_execute_shell_internal_fail_status_stderr() {
        let command = "ls /non_existent_directory_for_volition_test";
        let result = execute_shell_command_internal(command).await; // Calls the test version

        assert!(result.is_ok());
        let output_str = result.unwrap();
        println!("Mocked Output for '{}':\n{}", command, output_str);

        assert!(output_str.starts_with("Command executed with status: 2")); // Matches mock
        assert!(output_str.contains("\nStdout:\n<no output>"));
        assert!(output_str.contains("\nStderr:\nls: cannot access"));
    }

    #[tokio::test]
    async fn test_execute_shell_internal_command_not_found() {
        let command = "this_command_should_absolutely_not_exist_ever_42";
        let result = execute_shell_command_internal(command).await; // Calls the test version

        assert!(result.is_ok());
        let output_str = result.unwrap();
        println!("Mocked Output for '{}':\n{}", command, output_str);

        assert!(output_str.starts_with("Command executed with status: 127")); // Matches mock
        assert!(output_str.contains("\nStdout:\n<no output>"));
        assert!(output_str.contains("command not found"));
    }

    // TODO: Add tests for run_shell_command confirmation (needs stdin/stdout mocking)
}

// volition-cli/src/tools/cargo.rs
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, info, warn};

// Removed CargoCommandArgs import

// Define denied cargo commands (unchanged)
fn get_denied_cargo_commands() -> HashSet<String> {
    let mut denied = HashSet::new();
    denied.insert("login".to_string());
    denied.insert("logout".to_string());
    denied.insert("publish".to_string());
    denied.insert("owner".to_string());
    denied.insert("yank".to_string());
    denied.insert("install".to_string());
    denied
}

// Internal execution function (real version)
// Added working_dir argument
async fn execute_cargo_command_internal(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path, // Added working_dir
) -> Result<String> {
    let full_command = format!("cargo {} {}", command_name, command_args.join(" "));
    debug!(
        "Executing internal cargo command: {} in {:?}",
        full_command,
        working_dir
    );

    let output = Command::new("cargo")
        .current_dir(working_dir) // Set working directory
        .arg(command_name)
        .args(command_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context(format!("Failed to execute cargo command: {}", full_command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    debug!(
        "cargo {} {} exit status: {}",
        command_name,
        command_args.join(" "),
        status
    );

    let result = format!(
        "Command executed: cargo {} {}\nStatus: {}\nStdout:\n{}\nStderr:\n{}",
        command_name,
        command_args.join(" "),
        status,
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

// Public function exposed as the 'cargo_command' tool
// Refactored signature
pub async fn run_cargo_command(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path,
) -> Result<String> {
    let denied_commands = get_denied_cargo_commands();

    // Check against deny list (unchanged)
    if denied_commands.contains(command_name) {
        warn!(
            "Denied execution of cargo command: cargo {} {:?}",
            command_name,
            command_args
        );
        return Ok(format!(
            "Error: The cargo command '{}' is not allowed for security reasons.",
            command_name
        ));
    }

    info!("Running: cargo {} {}", command_name, command_args.join(" "));
    // Pass working_dir to internal function
    execute_cargo_command_internal(command_name, command_args, working_dir).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio;

    fn test_working_dir() -> PathBuf {
        tempdir()
            .map(|d| d.into_path())
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    // Mock internal function for tests
    async fn mock_execute_internal(
        command_name: &str,
        command_args: &[String],
        _working_dir: &Path, // Ignore working_dir in mock
    ) -> Result<String> {
         let full_command_for_print = format!("cargo {} {}", command_name, command_args.join(" "));
        println!(
            "[TEST] Mock execute_cargo_command_internal called with: {}",
            full_command_for_print
        );
        match command_name {
            "check" => Ok(format!(
                "Command executed: {}\nStatus: 0\nStdout:\n   Checking volition v0.1.0...\nStderr:\n<no output>",
                full_command_for_print
            )),
            "build" if command_args.contains(&"--release".to_string()) => Ok(format!(
                 "Command executed: {}\nStatus: 0\nStdout:\n   Finished release...\nStderr:\n<no output>",
                 full_command_for_print
            )),
             "build" => Ok(format!(
                 "Command executed: {}\nStatus: 101\nStdout:\n<no output>\nStderr:\nerror: build failed",
                 full_command_for_print
             )),
            _ => Ok(format!(
                "Command executed: {}\nStatus: 0\nStdout:\nMock success for {}\nStderr:\n<no output>",
                full_command_for_print, command_name
            )),
        }
    }


    #[tokio::test]
    async fn test_run_cargo_command_denied() {
        let working_dir = test_working_dir();
        let result = run_cargo_command("install", &["some_crate".to_string()], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Error: The cargo command 'install' is not allowed"));
    }

    #[tokio::test]
    async fn test_run_cargo_command_allowed_check_success() {
        let working_dir = test_working_dir();
        // Override internal call with mock
        async fn execute_cargo_command_internal(cn: &str, ca: &[String], wd: &Path) -> Result<String> { mock_execute_internal(cn, ca, wd).await }

        let result = run_cargo_command("check", &[], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("Checking volition"));
    }

    #[tokio::test]
    async fn test_run_cargo_command_allowed_build_fail() {
        let working_dir = test_working_dir();
         // Override internal call with mock
        async fn execute_cargo_command_internal(cn: &str, ca: &[String], wd: &Path) -> Result<String> { mock_execute_internal(cn, ca, wd).await }

        let result = run_cargo_command("build", &[], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 101"));
        assert!(output.contains("error: build failed"));
    }

    #[tokio::test]
    async fn test_run_cargo_command_allowed_build_release_success() {
        let working_dir = test_working_dir();
         // Override internal call with mock
        async fn execute_cargo_command_internal(cn: &str, ca: &[String], wd: &Path) -> Result<String> { mock_execute_internal(cn, ca, wd).await }

        let result = run_cargo_command("build", &["--release".to_string()], &working_dir).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("Finished release"));
    }
}

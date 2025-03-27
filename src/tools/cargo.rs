// src/tools/cargo.rs
use crate::models::tools::CargoCommandArgs;
use anyhow::Result; // Only Result needed by both
use std::collections::HashSet;
use tracing::{info, warn}; // info and warn needed by both

// Imports only needed for the non-test version
#[cfg(not(test))]
use {
    anyhow::Context, // Context only used in real execution
    std::process::{Command, Stdio},
    tracing::debug, // debug only used in real execution
};

// Define denied cargo commands
fn get_denied_cargo_commands() -> HashSet<String> {
    let mut denied = HashSet::new();
    // Commands that might change credentials, publish crates, or install global binaries
    denied.insert("login".to_string());
    denied.insert("logout".to_string());
    denied.insert("publish".to_string());
    denied.insert("owner".to_string());
    denied.insert("yank".to_string());
    denied.insert("install".to_string()); // Can install globally
    denied
}

// Internal execution function (real version)
#[cfg(not(test))]
async fn execute_cargo_command_internal(command_name: &str, command_args: &[String]) -> Result<String> {
    let full_command = format!("cargo {} {}", command_name, command_args.join(" "));
    debug!("Executing internal cargo command: {}", full_command);

    let output = Command::new("cargo")
        .arg(command_name)
        .args(command_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context(format!("Failed to execute cargo command: {}", full_command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1); // Use -1 if status code isn't available

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
        if stdout.is_empty() { "<no output>" } else { &stdout },
        if stderr.is_empty() { "<no output>" } else { &stderr }
    );

    Ok(result)
}

// Internal execution function (test mock version)
#[cfg(test)]
async fn execute_cargo_command_internal(command_name: &str, command_args: &[String]) -> Result<String> {
    let full_command_for_print = format!("cargo {} {}", command_name, command_args.join(" "));
    println!("[TEST] Mock execute_cargo_command_internal called with: {}", full_command_for_print);

    // Mock based on command_name and potentially args
    match command_name {
        "check" => Ok(format!(
            "Command executed: {}\nStatus: 0\nStdout:\n   Checking volition v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s)\nStderr:\n<no output>",
            full_command_for_print
        )),
        "build" if command_args.contains(&"--release".to_string()) => Ok(format!(
             "Command executed: {}\nStatus: 0\nStdout:\n   Compiling volition v0.1.0\n    Finished release [optimized] target(s)\nStderr:\n<no output>",
             full_command_for_print
        )),
         "build" => Ok(format!( // Simulating build error without --release
             "Command executed: {}\nStatus: 101\nStdout:\n   Compiling volition v0.1.0\nStderr:\nerror[E0308]: mismatched types\n --> src/main.rs:10:5\n...",
             full_command_for_print
         )),
        _ => Ok(format!( // Default mock for other allowed commands
            "Command executed: {}\nStatus: 0\nStdout:\nMock success for {}
Stderr:\n<no output>",
            full_command_for_print, command_name
        )),
    }
}

// Public function exposed as the 'cargo_command' tool
pub async fn run_cargo_command(args: CargoCommandArgs) -> Result<String> {
    let command_name = &args.command;
    let command_args = &args.args;
    let denied_commands = get_denied_cargo_commands();

    // Check against deny list
    if denied_commands.contains(command_name) {
        warn!(
            "Denied execution of cargo command: cargo {} {:?}",
            command_name, command_args
        );
        return Ok(format!(
            "Error: The cargo command '{}' is not allowed for security reasons.",
            command_name
        ));
    }

    // If allowed, call the appropriate internal execution function
    info!("Running: cargo {} {}", command_name, command_args.join(" "));
    execute_cargo_command_internal(command_name, command_args).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_run_cargo_command_denied() {
        let args = CargoCommandArgs {
            command: "install".to_string(), // Denied command
            args: vec!["some_crate".to_string()],
        };
        let result = run_cargo_command(args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Error: The cargo command 'install' is not allowed"));
    }

     #[tokio::test]
    async fn test_run_cargo_command_allowed_check_success() {
         // This test now implicitly tests execute_cargo_command_internal mock
        let args = CargoCommandArgs {
            command: "check".to_string(), // Allowed command
            args: vec![],
        };
        let result = run_cargo_command(args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("Finished dev"));
        assert!(output.contains("Stderr:\n<no output>"));
    }

     #[tokio::test]
    async fn test_run_cargo_command_allowed_build_fail() {
         // This test now implicitly tests execute_cargo_command_internal mock for build failure
        let args = CargoCommandArgs {
            command: "build".to_string(), // Allowed command
            args: vec![], // No --release, triggers mock failure case
        };
        let result = run_cargo_command(args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
         println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 101"));
        assert!(output.contains("Stderr:\nerror[E0308]: mismatched types"));
    }

     #[tokio::test]
    async fn test_run_cargo_command_allowed_build_release_success() {
         // This test now implicitly tests execute_cargo_command_internal mock for release build
        let args = CargoCommandArgs {
            command: "build".to_string(), // Allowed command
            args: vec!["--release".to_string()],
        };
        let result = run_cargo_command(args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
         println!("Mocked Output:\n{}", output);
        assert!(output.contains("Status: 0"));
         assert!(output.contains("Finished release"));
        assert!(output.contains("Stderr:\n<no output>"));
    }
}

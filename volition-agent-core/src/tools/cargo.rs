// volition-agent-core/src/tools/cargo.rs

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, info}; // Removed warn as deny list is removed

/// Executes a cargo command in a specified working directory.
///
/// Note: This does not perform safety checks. Callers should ensure
/// the command/args are safe or implement checks separately.
pub async fn execute_cargo_command(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path,
) -> Result<String> {
    let full_command_log = format!("cargo {} {}", command_name, command_args.join(" "));
    info!(
        "Executing cargo command: {} in {:?}",
        full_command_log,
        working_dir
    );

    let output = Command::new("cargo")
        .current_dir(working_dir)
        .arg(command_name)
        .args(command_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to execute cargo command: {}", full_command_log))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    debug!(
        "cargo {} exit status: {}",
        full_command_log,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tokio;

    fn test_working_dir() -> PathBuf {
        // Try to create a temp dir for cargo commands, default to . if it fails
        tempdir()
            .map(|d| {
                let path = d.into_path();
                // Basic cargo init for some tests?
                // For now, just return the path.
                path
            })
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    #[tokio::test]
    async fn test_execute_cargo_check() {
        let working_dir = test_working_dir();
        // Create a dummy Cargo.toml if testing in temp dir
        if working_dir != Path::new(".") {
            std::fs::write(working_dir.join("Cargo.toml"), "[package]\nname = \"test_crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
             std::fs::create_dir(working_dir.join("src")).unwrap();
             std::fs::write(working_dir.join("src/lib.rs"), "pub fn hello() {}").unwrap();
        }

        let result = execute_cargo_command("check", &[], &working_dir).await;
        assert!(result.is_ok(), "cargo check failed: {:?}", result.err());
        let output = result.unwrap();
        println!("Output:\n{}", output);
        assert!(output.contains("Status: 0"));
        assert!(output.contains("Checking test_crate") || output.contains("Checking volition")); // Name depends on where test runs
        assert!(output.contains("Finished dev"));
    }

     #[tokio::test]
    async fn test_execute_cargo_build_fail_no_src() {
        let working_dir = test_working_dir();
         // Create ONLY Cargo.toml, no src/lib.rs
        if working_dir != Path::new(".") {
            std::fs::write(working_dir.join("Cargo.toml"), "[package]\nname = \"test_crate_fail\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
        }

        // Don't run this test if working_dir is "." as it might find real src
        if working_dir != Path::new(".") {
            let result = execute_cargo_command("build", &[], &working_dir).await;
            assert!(result.is_ok(), "Expected Ok result even on build failure");
            let output = result.unwrap();
            println!("Output:\n{}", output);
            assert!(output.contains("Status: 101")); // Build should fail
            assert!(output.contains("failed to run `rustc`")); // Common error message
        }
    }
}

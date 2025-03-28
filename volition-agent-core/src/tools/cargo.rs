// volition-agent-core/src/tools/cargo.rs

use super::CommandOutput;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, info};

pub async fn execute_cargo_command(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path,
) -> Result<CommandOutput> {
    let full_command_log = format!("cargo {} {}", command_name, command_args.join(" "));
    info!(
        "Executing cargo command: {} in {:?}",
        full_command_log, working_dir
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

    debug!("cargo {} exit status: {}", full_command_log, status);

    Ok(CommandOutput {
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;
    use tokio;

    fn test_working_dir() -> PathBuf {
        tempdir()
            .map(|d| {
                let path = d.into_path();
                path
            })
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    #[tokio::test]
    async fn test_execute_cargo_check() {
        let working_dir = test_working_dir();
        if working_dir != Path::new(".") {
            std::fs::write(
                working_dir.join("Cargo.toml"),
                "[package]\nname = \"test_crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            )
            .unwrap();
            std::fs::create_dir(working_dir.join("src")).unwrap();
            std::fs::write(working_dir.join("src/lib.rs"), "pub fn hello() {}").unwrap();
        }

        let result = execute_cargo_command("check", &[], &working_dir).await;
        assert!(result.is_ok(), "cargo check failed: {:?}", result.err());
        let output = result.unwrap();
        println!("Output: {:?}", output);
        assert_eq!(output.status, 0);
        assert!(
            output.stderr.contains("Checking test_crate")
                || output.stderr.contains("Checking volition")
        );
        assert!(output.stderr.contains("Finished `dev` profile"));
    }

    #[tokio::test]
    async fn test_execute_cargo_build_fail_no_src() {
        let working_dir = test_working_dir();
        if working_dir != Path::new(".") {
            std::fs::write(
                working_dir.join("Cargo.toml"),
                "[package]\nname = \"test_crate_fail\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            )
            .unwrap();
        }

        if working_dir != Path::new(".") {
            let result = execute_cargo_command("build", &[], &working_dir).await;
            assert!(result.is_ok());
            let output = result.unwrap();
            println!("Output: {:?}", output);
            assert_ne!(output.status, 0);
            assert!(output
                .stderr
                .contains("no targets specified in the manifest"));
        }
    }
}

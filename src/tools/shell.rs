use std::process::{Command, Stdio};
use anyhow::{Result, Context};
use colored::*;
use crate::models::tools::ShellArgs;
use tracing::{debug};

pub async fn run_shell_command(args: ShellArgs) -> Result<String> {
    let command = &args.command;

    println!("{} {}", "Running:".blue().bold(), command);
    
    debug!("Executing command: {}", command);

    // Regular command execution logic
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute Windows command")?
    } else {
        Command::new("sh")
            .args(["-c", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute shell command")?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = output.status.code().unwrap_or(-1);

    let stdout_preview = stdout.lines().take(3).collect::<Vec<&str>>().join("\n");
    let stderr_preview = if !stderr.is_empty() {
        format!("\nStderr preview: {}", stderr.lines().take(3).collect::<Vec<&str>>().join("\n"))
    } else {
        String::new()
    };

    debug!("Command exit status: {}\nOutput preview:\n{}{}", 
           status,
           if stdout_preview.is_empty() { "<no output>" } else { &stdout_preview },
           stderr_preview);

    let detailed_info = format!(
        "Stdout length: {} bytes, Stderr length: {} bytes, Total lines: {}",
        stdout.len(),
        stderr.len(),
        stdout.lines().count() + stderr.lines().count()
    );
    debug!("{}", detailed_info);

    let result = format!(
        "Command executed with status: {}\nStdout:\n{}\nStderr:\n{}",
        status,
        if stdout.is_empty() { "<no output>" } else { &stdout },
        if stderr.is_empty() { "<no output>" } else { &stderr }
    );

    Ok(result)
}

use std::process::{Command, Stdio};
use anyhow::{Result, Context};
use colored::*;
use crate::utils::DebugLevel;
use crate::utils::debug_log;
use crate::models::tools::ShellArgs;

pub async fn run_shell_command(args: ShellArgs, debug_level: DebugLevel) -> Result<String> {
    let command = &args.command;

    println!("{} {}", "Running:".blue().bold(), command);

    if debug_level >= DebugLevel::Verbose {
        debug_log(debug_level, DebugLevel::Verbose, &format!("Executing command: {}", command));
    }

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

    if debug_level >= DebugLevel::Minimal {
        // Get first few lines of output for preview
        let stdout_preview = stdout.lines().take(3).collect::<Vec<&str>>().join("\n");
        let stderr_preview = if !stderr.is_empty() {
            format!("\nStderr preview: {}", stderr.lines().take(3).collect::<Vec<&str>>().join("\n"))
        } else {
            String::new()
        };

        debug_log(
            debug_level,
            DebugLevel::Minimal,
            &format!(
                "Command exit status: {}
Output preview:
{}{}",
                status,
                if stdout_preview.is_empty() { "<no output>" } else { &stdout_preview },
                stderr_preview
            )
        );

        // Add more detailed info at verbose level
        if debug_level >= DebugLevel::Verbose {
            debug_log(
                debug_level,
                DebugLevel::Verbose,
                &format!(
                    "Stdout length: {} bytes, Stderr length: {} bytes, Total lines: {}",
                    stdout.len(),
                    stderr.len(),
                    stdout.lines().count() + stderr.lines().count()
                )
            );
        }
    }

    // Format the result as a single string
    let result = format!(
        "Command executed with status: {}\nStdout:\n{}\nStderr:\n{}",
        status,
        if stdout.is_empty() { "<no output>" } else { &stdout },
        if stderr.is_empty() { "<no output>" } else { &stderr }
    );

    Ok(result)
}

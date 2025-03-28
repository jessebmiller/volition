// volition-cli/src/tools/git.rs
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use tracing::warn;

use volition_agent_core::tools::git::execute_git_command as execute_git_command_core;
use volition_agent_core::tools::CommandOutput;

fn is_git_command_denied(command_name: &str, args: &[String]) -> bool {
    let denied_commands: HashSet<&str> = [
        "push", "reset", "rebase", "checkout", "merge", "clone", "remote", "fetch", "pull",
    ]
    .iter()
    .cloned()
    .collect();
    if denied_commands.contains(command_name) {
        return true;
    }
    if command_name == "branch" && args.contains(&"-D".to_string()) {
        return true;
    }
    false
}

pub async fn run_git_command(
    command_name: &str,
    command_args: &[String],
    working_dir: &Path,
) -> Result<String> {
    if is_git_command_denied(command_name, command_args) {
        warn!(
            "Denied execution of git command: git {} {:?}",
            command_name, command_args
        );
        return Ok(format!(
            "Error: The git command 'git {} {}' is not allowed for security or stability reasons.",
            command_name,
            command_args.join(" ")
        ));
    }

    let cmd_output: CommandOutput =
        execute_git_command_core(command_name, command_args, working_dir).await?;

    let command_str = format!("git {} {}", command_name, command_args.join(" "));
    Ok(format!(
        "Command executed: {}\nStatus: {}\nStdout:\n{}\nStderr:\n{}",
        command_str,
        cmd_output.status,
        if cmd_output.stdout.is_empty() {
            "<no output>"
        } else {
            &cmd_output.stdout
        },
        if cmd_output.stderr.is_empty() {
            "<no output>"
        } else {
            &cmd_output.stderr
        }
    ))
}

// volition-agent-core/src/tools/mod.rs

// Modules for standard, non-interactive tool implementations
pub mod cargo;
pub mod fs;
pub mod git;
pub mod search;
pub mod shell;

/// Represents the structured output of an executed external command.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandOutput {
    /// The exit status code of the command.
    pub status: i32,
    /// The captured standard output.
    pub stdout: String,
    /// The captured standard error.
    pub stderr: String,
}

impl CommandOutput {
    /// Checks if the command executed successfully (status code 0).
    pub fn success(&self) -> bool {
        self.status == 0
    }

    /// Formats the output similar to how `execute_shell_command` used to.
    /// This can be used by providers or wrappers to create the final string for the AI.
    pub fn format_for_ai(&self, command_str: &str) -> String {
        format!(
            "Command executed: {}\nStatus: {}\nStdout:\n{}\nStderr:\n{}",
            command_str,
            self.status,
            if self.stdout.is_empty() { "<no output>" } else { &self.stdout },
            if self.stderr.is_empty() { "<no output>" } else { &self.stderr }
        )
    }
}

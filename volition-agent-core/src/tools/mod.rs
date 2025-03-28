// volition-agent-core/src/tools/mod.rs

//! Contains implementations for standard, non-interactive tools.
//! These functions provide the core logic for interacting with external commands
//! or the filesystem. They are designed as reusable building blocks for
//! `ToolProvider` implementations.
//! 
//! **Important:** These functions generally do *not* include safety checks
//! (like command argument validation, file path sandboxing) or user interaction
//! (like confirmation prompts). Callers, typically `ToolProvider` implementations,
//! are responsible for adding necessary safety layers before invoking these core functions.

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

    // Formatting is now responsibility of the caller (e.g., ToolProvider impl)
}

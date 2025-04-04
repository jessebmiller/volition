// volition-cli/src/models/cli.rs
use clap::{ArgAction, Parser, Subcommand}; // Import Subcommand
use uuid::Uuid; // Import Uuid

/// Volition: An AI-powered assistant for software engineering tasks.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Increase message verbosity.
    ///
    /// Specify multiple times for more verbose output:
    ///  -v:  INFO level
    ///  -vv: DEBUG level
    ///  -vvv: TRACE level (most verbose)
    #[arg(short, long, action = ArgAction::Count, global = true)] // Make global
    pub verbose: u8,

    /// Optional prompt for a non-interactive single turn (starts a new conversation).
    #[arg(long)]
    pub turn: Option<String>,

    // Keep the old -t/--task for backward compatibility or remove if desired.
    // If kept, it should probably conflict with `turn` and subcommands.
    // For now, let's remove it to enforce the new structure.
    // /// Run a single task non-interactively (DEPRECATED: Use --turn instead).
    // #[arg(short, long)]
    // pub task: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Resume an existing interactive conversation.
    Resume {
        /// ID of the conversation to resume.
        id: Uuid, // Use Uuid directly

        /// Optional prompt for a non-interactive single turn on the resumed conversation.
        #[arg(long)]
        turn: Option<String>,
    },
    /// List recent conversations.
    List {
        /// Maximum number of conversations to list.
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// View the details of a conversation.
    View {
        /// ID of the conversation to view.
        id: Uuid, // Use Uuid directly

        /// Show the full message content (can be long).
        #[arg(long)]
        full: bool,
    },
    /// Delete a conversation history.
    Delete {
        /// ID of the conversation to delete.
        id: Uuid, // Use Uuid directly
    },
    // Future commands like 'config' could go here
}

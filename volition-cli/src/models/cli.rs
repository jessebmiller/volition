use clap::{ArgAction, Parser}; // Import ArgAction

/// Volition: An AI-powered assistant.
/// Starts an interactive session.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Increase message verbosity.
    ///
    /// Specify multiple times for more verbose output:
    ///  -v:  INFO level
    ///  -vv: DEBUG level
    ///  -vvv: TRACE level (most verbose)
    #[arg(short, long, action = ArgAction::Count)] // Use count action
    pub verbose: u8, // Store the count as u8

                     // Removed debug field

                     // Removed command field
                     // Removed rest field
}

// Removed Commands enum

use clap::Parser;

/// Volition: An AI-powered assistant.
/// Starts an interactive session.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose debug logging (TRACE level)
    #[arg(short, long)]
    pub verbose: bool,

    /// Enable informational debug logging (INFO level)
    #[arg(short, long)]
    pub debug: bool,

    // Removed command field
    // Removed rest field
}

// Removed Commands enum

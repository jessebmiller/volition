use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose debug logging
    #[arg(short, long)]
    pub verbose: bool,

    /// Enable minimal debug logging
    #[arg(short, long)]
    pub debug: bool,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub rest: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a command without using an interactive session
    Run {
        /// Enable verbose debug logging
        #[arg(short, long)]
        verbose: bool,

        /// Enable minimal debug logging
        #[arg(short, long)]
        debug: bool,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
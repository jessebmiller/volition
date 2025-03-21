use colored::*;

/// Debug logging level enumeration
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DebugLevel {
    None,
    Minimal,
    Verbose,
}

/// Helper function to log debug messages based on the current debug level
pub fn debug_log(level: DebugLevel, min_level: DebugLevel, message: &str) {
    if level >= min_level {
        println!("{} {}", "DEBUG:".yellow().bold(), message);
    }
}
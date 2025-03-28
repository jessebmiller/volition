// volition-agent-core/src/tools/mod.rs

// Modules for standard, non-interactive tool implementations
pub mod cargo;
pub mod fs; // For file system operations (read, write, list)
pub mod git;
pub mod search; // For ripgrep-based search/find
pub mod shell;

// Re-export the tool structs for easier use?
// Or maybe just the functions?
// Let's start without re-exports and see how it feels.

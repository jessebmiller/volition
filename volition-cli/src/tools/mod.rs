// volition-cli/src/tools/mod.rs

// Declare implementation modules
pub mod cargo;
pub mod file;
pub mod filesystem;
pub mod git;
pub mod search;
pub mod shell;
pub mod user_input;

// Declare and export the provider module
pub mod provider;
pub use provider::CliToolProvider;

// Old handle_tool_calls function and its imports/tests are removed.

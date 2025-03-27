// src/main.rs
mod api;
mod config;
mod models;
mod rendering;
mod tools;

use anyhow::{Context, Result};
use colored::*;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use tokio::time::Duration;

use crate::api::chat_with_api;
use crate::config::{load_runtime_config, RuntimeConfig};
use crate::models::chat::ResponseMessage;
use crate::models::cli::Cli;
use crate::rendering::print_formatted;
use crate::tools::handle_tool_calls;

use clap::Parser;
use tracing::{debug, error, info, warn, Level}; // Added warn, info, debug
use tracing_subscriber::FmtSubscriber;

const RECOVERY_FILE_PATH: &str = ".conversation_state.json";

// --- Helper Functions ---

fn print_welcome_message() {
    println!("\n{}", "Volition - AI Assistant".cyan().bold());
    println!(
        "{}",
        "Type 'exit' or press Enter on an empty line to quit.".cyan()
    );
    println!();
}

/// Attempts to load a previous session state or initializes a new one based on user input.
/// Returns Ok(Some(messages)) if a session should start.
/// Returns Ok(None) if the user exits immediately during initial prompt.
/// Returns Err if a critical error occurs.
fn load_or_initialize_session(config: &RuntimeConfig) -> Result<Option<Vec<ResponseMessage>>> {
    let recovery_path = Path::new(RECOVERY_FILE_PATH);
    let mut messages_option: Option<Vec<ResponseMessage>> = None;

    // --- Load State Logic ---
    if recovery_path.exists() {
        info!(
            "Found existing conversation state file: {}",
            RECOVERY_FILE_PATH
        );
        print!(
            "{}",
            "An incomplete session state was found. Resume? (Y/n): "
                .yellow()
                .bold()
        );
        io::stdout().flush()?;

        let mut user_choice = String::new();
        io::stdin().read_line(&mut user_choice)?;

        if user_choice.trim().to_lowercase() != "n" {
            match fs::read_to_string(recovery_path) {
                Ok(state_json) => match serde_json::from_str(&state_json) {
                    Ok(loaded_messages) => {
                        messages_option = Some(loaded_messages); // Assign loaded messages
                        info!("Successfully resumed session from state file.");
                        println!("{}", "Resuming previous session...".cyan());
                        let _ = fs::remove_file(recovery_path); // Clean up immediately after successful load
                    }
                    Err(e) => {
                        error!("Failed to deserialize state file: {}. Starting fresh.", e);
                        println!(
                            "{}",
                            "Error reading state file. Starting a fresh session.".red()
                        );
                        let _ = fs::remove_file(recovery_path); // Clean up even on error
                    }
                },
                Err(e) => {
                    error!("Failed to read state file: {}. Starting fresh.", e);
                    println!(
                        "{}",
                        "Error reading state file. Starting a fresh session.".red()
                    );
                    let _ = fs::remove_file(recovery_path); // Clean up even on error
                }
            }
        } else {
            info!("User chose not to resume. Starting fresh.");
            println!("{}", "Starting a fresh session.".cyan());
            let _ = fs::remove_file(recovery_path); // Clean up if user declines
        }
    }
    // --- End Load State Logic ---

    // --- Get Initial Query if messages_option is still None ---
    if messages_option.is_none() {
        println!("{}", "How can I help you?".cyan());
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;

        let mut initial_input = String::new();
        io::stdin().read_line(&mut initial_input)?;
        let initial_input = initial_input.trim();

        if initial_input.is_empty() || initial_input.to_lowercase() == "exit" {
            // User wants to exit immediately
            return Ok(None);
        }
        // Initialize messages only if we got valid initial input
        messages_option = Some(initialize_messages(initial_input, &config.system_prompt));
    }
    // --- End Initial Query ---

    Ok(messages_option) // Return Some(messages) or None if already handled exit
}

/// Runs the main interactive conversation loop.
async fn run_conversation_loop(
    config: &RuntimeConfig,
    client: &reqwest::Client,
    messages: &mut Vec<ResponseMessage>,
) -> Result<()> {
    let mut conversation_active = true;
    while conversation_active {
        debug!("Current message history: {:?}", messages);

        // --- Save State Logic (at the beginning of the loop iteration) ---
        match serde_json::to_string_pretty(&messages) {
            Ok(state_json) => {
                if let Err(e) = fs::write(RECOVERY_FILE_PATH, state_json) {
                    error!("Failed to write recovery state file: {}", e);
                    // Consider if this should be a fatal error? For now, just log.
                } else {
                    debug!("Successfully saved conversation state.");
                }
            }
            Err(e) => {
                error!("Failed to serialize conversation state: {}", e);
                // Consider if this should be a fatal error? For now, just log.
            }
        }
        // --- End Save State Logic ---

        // Call the API
        let response_result = chat_with_api(client, config, messages.clone()).await;

        // Check for API errors or empty choices
        let message_option = match response_result {
            Ok(response) => {
                if let Some(choice) = response.choices.into_iter().next() {
                    // Take the first choice
                    Some(choice.message)
                } else {
                    error!("API response did not contain any choices.");
                    println!(
                        "{}",
                        "Error: Received an empty response from the AI service.".red()
                    );
                    None // Indicate no valid message received
                }
            }
            Err(e) => {
                error!("API call failed: {}", e);
                println!("{}
{}", "Error calling AI service:".red(), e);
                None // Indicate no valid message received
            }
        };

        // Process the message if we received one
        if let Some(message) = message_option {
            // Add a newline before printing AI response for better spacing
            println!();

            if let Some(content) = &message.content {
                if !content.is_empty() {
                    // Use the new formatted print function and handle potential error
                    if let Err(e) = print_formatted(content) {
                        // Log the rendering error and fall back to plain print
                        error!("Failed to render markdown: {}. Printing raw content.", e);
                        println!("{}", content); // Fallback
                    }
                }
            }
            // Add a newline after printing AI response for better spacing
            println!();

            // Add assistant message to history BEFORE processing tool calls
            let assistant_message = ResponseMessage {
                role: "assistant".to_string(),
                content: message.content.clone(),
                tool_calls: message.tool_calls.clone(),
                tool_call_id: None,
            };
            messages.push(assistant_message);

            // Process tool calls if present
            if let Some(tool_calls) = message.tool_calls {
                // Use the tool_calls from the original message
                info!("Processing {} tool calls", tool_calls.len());
                // Pass the full config to handle_tool_calls
                if let Err(e) =
                    handle_tool_calls(client, config, tool_calls.to_vec(), messages).await
                {
                    error!("Error handling tool calls: {}", e);
                    println!("{}
{}", "Error during tool execution:".red(), e);
                    // Decide whether to continue or break here. Let's continue for now.
                }
                // After handling tool calls, loop back to call API again
                continue; // Skip the user input prompt for this iteration
            }
            // If no tool calls, fall through to prompt user
        } else {
            // If message_option was None (due to API error or empty choices),
            // we skip processing and directly prompt the user again.
            println!(
                "\n{}",
                "Please try again or enter a different query:".yellow()
            );
        }

        // Prompt for next user input (only if no tool calls were processed in this iteration)
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        if input.is_empty() || input.to_lowercase() == "exit" {
            println!("\n{}", "Goodbye!".cyan());
            conversation_active = false; // Exit the loop
        } else {
            messages.push(ResponseMessage {
                role: "user".to_string(),
                content: Some(input),
                tool_calls: None,
                tool_call_id: None,
            });
        }
    } // End while conversation_active

    Ok(())
}

/// Cleans up the session state recovery file.
fn cleanup_session_state() -> Result<()> {
    let recovery_path = Path::new(RECOVERY_FILE_PATH);
    if recovery_path.exists() {
        if let Err(e) = fs::remove_file(recovery_path) {
            // Log as warning, not critical if cleanup fails
            warn!("Failed to remove recovery state file on exit: {}", e);
        } else {
            info!("Removed recovery state file on clean exit.");
        }
    }
    Ok(())
}

/// Initializes the message history with system prompt and initial user query.
fn initialize_messages(initial_query: &str, system_prompt: &str) -> Vec<ResponseMessage> {
    vec![
        ResponseMessage {
            role: "system".to_string(),
            content: Some(system_prompt.to_string()), // Use the passed argument
            tool_calls: None,
            tool_call_id: None,
        },
        ResponseMessage {
            role: "user".to_string(),
            content: Some(initial_query.to_string()),
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}

// --- Main Application Entry Point ---

/// Main orchestrator for the interactive session.
async fn start_interactive_session(config: &RuntimeConfig) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60)) // Consider making timeout configurable
        .build()?;

    print_welcome_message();

    // Try to load or initialize the session messages
    match load_or_initialize_session(config)? {
        // This remains synchronous for stdin
        Some(mut messages) => {
            // If successful, run the main conversation loop
            run_conversation_loop(config, &client, &mut messages).await?;
        }
        None => {
            // If load_or_initialize_session returned None, it means the user exited immediately
            println!("\n{}", "Goodbye!".cyan());
            // No loop needed, session ends here.
        }
    }

    // Always attempt cleanup after the session ends (either normally or via early exit)
    cleanup_session_state()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Setup tracing subscriber based on verbosity count
    let level = match cli.verbose {
        0 => Level::WARN,  // Default
        1 => Level::INFO,  // -v
        2 => Level::DEBUG, // -vv
        _ => Level::TRACE, // -vvv or more
    };

    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // Load Configuration
    let config = load_runtime_config()
        .context("Failed to load configuration from Volition.toml and environment")?;

    // Start Interactive Session Directly
    start_interactive_session(&config).await?;

    Ok(())
}

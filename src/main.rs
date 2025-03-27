mod api;
mod config;
mod models;
mod tools;

// Removed unused anyhow import
use anyhow::{Context, Result};
use colored::*;
use serde_json;
use std::{fs, io::{self, Write}, path::Path};
use tokio::time::Duration;

use crate::api::chat_with_api;
use crate::config::{load_runtime_config, RuntimeConfig};
use crate::models::chat::ResponseMessage;
// Updated import: Remove Commands
use crate::models::cli::Cli;
use crate::tools::handle_tool_calls;

use clap::Parser;
use tracing::{Level};
use tracing_subscriber::FmtSubscriber;

const RECOVERY_FILE_PATH: &str = ".conversation_state.json";

// Renamed and modified handle_conversation to start_interactive_session
async fn start_interactive_session(
    config: &RuntimeConfig, // Use the combined config
    // Removed query parameter
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    // Updated, more general welcome message
    println!("\n{}", "Volition - AI Assistant".cyan().bold());
    println!("{}", "Type 'exit' or press Enter on an empty line to quit.".cyan());
    println!("");

    // Declare messages Option, initialize later
    let mut messages_option: Option<Vec<ResponseMessage>> = None;
    let recovery_path = Path::new(RECOVERY_FILE_PATH);

    // --- Load State Logic ---
    if recovery_path.exists() {
        tracing::info!("Found existing conversation state file: {}", RECOVERY_FILE_PATH);
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
                        tracing::info!("Successfully resumed session from state file.");
                        println!("{}", "Resuming previous session...".cyan());
                        let _ = fs::remove_file(recovery_path);
                    }
                    Err(e) => {
                        tracing::error!("Failed to deserialize state file: {}. Starting fresh.", e);
                        println!("{}", "Error reading state file. Starting a fresh session.".red());
                        let _ = fs::remove_file(recovery_path);
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read state file: {}. Starting fresh.", e);
                    println!("{}", "Error reading state file. Starting a fresh session.".red());
                    let _ = fs::remove_file(recovery_path);
                }
            }
        } else {
            tracing::info!("User chose not to resume. Starting fresh.");
            println!("{}", "Starting a fresh session.".cyan());
            let _ = fs::remove_file(recovery_path);
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
            println!("\n{}", "Goodbye!".cyan());
            return Ok(()); // Exit immediately if first input is empty or exit
        }
        // Initialize messages only if we got valid initial input
        messages_option = Some(initialize_messages(initial_input, &config.system_prompt));
    }
    // --- End Initial Query ---

    // --- Main Conversation Loop ---
    // Ensure messages_option is Some before starting the loop
    if let Some(mut messages) = messages_option { // Shadow messages_option with the actual Vec
        let mut conversation_active = true;
        while conversation_active {
            tracing::debug!("Current message history: {:?}", messages);

            // Call the API
            let response_result = chat_with_api(&client, config, messages.clone()).await;

            // Check for API errors or empty choices
            let message_option = match response_result {
                Ok(response) => {
                    if let Some(choice) = response.choices.into_iter().next() { // Take the first choice
                        Some(choice.message)
                    } else {
                        tracing::error!("API response did not contain any choices.");
                        println!("{}", "Error: Received an empty response from the AI service.".red());
                        None // Indicate no valid message received
                    }
                }
                Err(e) => {
                    tracing::error!("API call failed: {}", e);
                    println!("{}\n{}", "Error calling AI service:".red(), e);
                    None // Indicate no valid message received
                }
            };

            // Process the message if we received one
            if let Some(message) = message_option {
                if let Some(content) = &message.content {
                    if !content.is_empty() {
                        println!("\n{}", content);
                    }
                }

                // Add assistant message to history BEFORE processing tool calls
                let assistant_message = ResponseMessage {
                    role: "assistant".to_string(),
                    content: message.content.clone(),
                    tool_calls: message.tool_calls.clone(),
                    tool_call_id: None,
                };
                messages.push(assistant_message);


                // Process tool calls if present
                if let Some(tool_calls) = message.tool_calls { // Use the tool_calls from the original message
                    tracing::info!("Processing {} tool calls", tool_calls.len());
                    // Handle tool calls (which will add tool responses to messages)
                    if let Err(e) = handle_tool_calls(&client, &config.api_key, tool_calls.to_vec(), &mut messages).await {
                         tracing::error!("Error handling tool calls: {}", e);
                         println!("{}\n{}", "Error during tool execution:".red(), e);
                         // Decide whether to continue or break here. Let's continue for now.
                    }
                    // After handling tool calls, loop back to call API again
                    continue; // Skip the user input prompt for this iteration
                }
                // If no tool calls, fall through to prompt user
            } else {
                // If message_option was None (due to API error or empty choices),
                // we skip processing and directly prompt the user again.
                println!("\n{}", "Please try again or enter a different query:".yellow());
            }

            // Prompt for next user input (only if no tool calls were processed in this iteration)
            print!("{} ", ">".green().bold());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_string();

            if input.is_empty() || input.to_lowercase() == "exit" {
                println!("\n{}", "Goodbye!".cyan());
                conversation_active = false;
            } else {
                messages.push(ResponseMessage {
                    role: "user".to_string(),
                    content: Some(input),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }

            // --- Save State Logic ---
             if conversation_active {
                match serde_json::to_string_pretty(&messages) {
                    Ok(state_json) => {
                        if let Err(e) = fs::write(RECOVERY_FILE_PATH, state_json) {
                            tracing::error!("Failed to write recovery state file: {}", e);
                        } else {
                            tracing::debug!("Successfully saved conversation state.");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize conversation state: {}", e);
                    }
                }
            }
        } // End while conversation_active
    } // End if let Some(mut messages)
    // --- End Main Conversation Loop ---

    // --- Cleanup Logic (unchanged) ---
    if Path::new(RECOVERY_FILE_PATH).exists() {
        if let Err(e) = fs::remove_file(RECOVERY_FILE_PATH) {
            tracing::warn!("Failed to remove recovery state file on exit: {}", e);
        } else {
            tracing::info!("Removed recovery state file on clean exit.");
        }
    }

    Ok(())
}

// Renamed default_messages to initialize_messages
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

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (unchanged)
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Setup tracing subscriber based on verbosity count
    let level = match cli.verbose {
        0 => Level::WARN,  // Default
        1 => Level::INFO,  // -v
        2 => Level::DEBUG, // -vv
        _ => Level::TRACE, // -vvv or more
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // --- Load Configuration ---
    let config = load_runtime_config()
        .context("Failed to load configuration from Volition.toml and environment")?;
    // --- End Load Configuration ---

    // --- Start Interactive Session Directly ---
    // Remove the match statement and directly call the session handler
    start_interactive_session(&config).await?;
    // --- End Interactive Session ---

    Ok(())
}

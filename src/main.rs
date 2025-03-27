mod api;
mod config; // Keep this for service config
mod models;
mod tools;

use anyhow::{anyhow, Context, Result};
use colored::*;
use serde_json;
use std::{fs, io::{self, Write}, path::Path};
use tokio::time::Duration;

use crate::api::chat_with_api;
// Use renamed config struct and loading function
use crate::config::{load_config, load_volition_project_config, VolitionProjectConfig};
use crate::models::chat::ResponseMessage;
use crate::models::cli::{Commands, Cli};
use crate::tools::handle_tool_calls;

use clap::Parser;
use tracing::{Level};
use tracing_subscriber::FmtSubscriber;

const RECOVERY_FILE_PATH: &str = ".conversation_state.json";

// Modified handle_conversation to accept VolitionProjectConfig
async fn handle_conversation(
    service_config: &config::Config,
    project_config: &VolitionProjectConfig, // Updated type
    query: &str,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    // Print welcome message (unchanged)
    println!("\n{}", "Volition - AI Software Engineering Assistant".cyan().bold());
    println!("{}", "Ready to help you understand and improve your codebase.".cyan());
    println!("{}", "Type 'exit' or press Enter on an empty line to quit".cyan());
    println!("");

    let mut messages: Vec<ResponseMessage>;
    let recovery_path = Path::new(RECOVERY_FILE_PATH);

    // --- Load State Logic (modified to pass project_config.system_prompt) ---
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
                        messages = loaded_messages;
                        tracing::info!("Successfully resumed session from state file.");
                        println!("{}", "Resuming previous session...".cyan());
                        let _ = fs::remove_file(recovery_path);
                    }
                    Err(e) => {
                        tracing::error!("Failed to deserialize state file: {}. Starting fresh.", e);
                        println!("{}", "Error reading state file. Starting a fresh session.".red());
                        let _ = fs::remove_file(recovery_path);
                        messages = default_messages(query, &project_config.system_prompt); // Pass prompt
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read state file: {}. Starting fresh.", e);
                    println!("{}", "Error reading state file. Starting a fresh session.".red());
                    let _ = fs::remove_file(recovery_path);
                    messages = default_messages(query, &project_config.system_prompt); // Pass prompt
                }
            }
        } else {
            tracing::info!("User chose not to resume. Starting fresh.");
            println!("{}", "Starting a fresh session.".cyan());
            let _ = fs::remove_file(recovery_path);
            messages = default_messages(query, &project_config.system_prompt); // Pass prompt
        }
    } else {
        messages = default_messages(query, &project_config.system_prompt); // Pass prompt
    }
    // --- End Load State Logic ---

    let mut conversation_active = true;

    // --- Main Conversation Loop ---
    while conversation_active {
        tracing::debug!("Current message history: {:?}", messages);

        let response = chat_with_api(&client, service_config, messages.clone(), None).await?;

         let message = match response.choices.get(0) {
            Some(choice) => &choice.message,
            None => {
                 tracing::error!("API response did not contain any choices.");
                 println!("{}", "Error: Received an empty response from the AI service.".red());
                 break;
            }
        };

        if let Some(content) = &message.content {
            if !content.is_empty() {
                println!("\n{}", content);
            }
        }

        messages.push(ResponseMessage {
            role: "assistant".to_string(),
            content: message.content.clone(),
            tool_calls: message.tool_calls.clone(),
            tool_call_id: None,
        });

        if let Some(tool_calls) = &message.tool_calls {
            tracing::info!("Processing {} tool calls", tool_calls.len());
            handle_tool_calls(&client, &service_config.api_key, tool_calls.to_vec(), &mut messages).await?;
        } else {
            println!("\n{}", "Enter a follow-up question or press Enter to exit:".cyan().bold());
            print!("{} ", ">".green().bold());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_string();

            if input.is_empty() || input.to_lowercase() == "exit" {
                println!("\n{}", "Goodbye! Thank you for using Volition.".cyan());
                conversation_active = false;
            } else {
                messages.push(ResponseMessage {
                    role: "user".to_string(),
                    content: Some(input),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        }

        // --- Save State Logic (unchanged) ---
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
    }
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

// Modified default_messages to accept system_prompt as an argument
fn default_messages(query: &str, system_prompt: &str) -> Vec<ResponseMessage> {
    vec![
        ResponseMessage {
            role: "system".to_string(),
            content: Some(system_prompt.to_string()), // Use the passed argument
            tool_calls: None,
            tool_call_id: None,
        },
        ResponseMessage {
            role: "user".to_string(),
            content: Some(query.to_string()),
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

    // Setup tracing subscriber (unchanged)
    let level = if cli.verbose {
        Level::DEBUG
    } else if cli.debug {
        Level::INFO
    } else {
        Level::WARN
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // --- Load Configurations ---
    // Load service config first (API keys, models, etc.)
    let service_config = load_config().context("Failed to load service configuration")?;
    // Load project config (system prompt, etc.) using the renamed function
    let project_config = load_volition_project_config()
        .context("Failed to load project configuration from Volition.toml")?; // Updated context message
    // --- End Load Configurations ---

    // Main command matching and execution logic (modified to pass both configs)
    match &cli.command {
        Some(Commands::Run { args, verbose: _, debug: _ }) => {
            let query = args.join(" ");
            if query.is_empty() {
                return Err(anyhow!("Please provide a command to run"));
            }
            // Pass both configs to handle_conversation
            handle_conversation(&service_config, &project_config, &query).await?;
        }
        None => {
            if cli.rest.is_empty() {
                // Print usage instructions (unchanged)
                println!("Welcome to Volition - AI Software Engineering Assistant");
                println!("Usage: volition <command> [arguments]");
                 println!("Examples:");
                println!("  volition \"Analyze the src directory and list the main components\"");
                println!("  volition \"Find all usages of the login function and refactor it to use async/await\"");
                println!("  volition \"Help me understand how the routing system works in this codebase\"");
                println!("  volition --help       - Show more information");
                return Ok(());
            }
            let query = cli.rest.join(" ");
            // Pass both configs to handle_conversation
            handle_conversation(&service_config, &project_config, &query).await?;
        }
    }

    Ok(())
}

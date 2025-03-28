// volition-cli/src/main.rs
mod models;
mod rendering;
mod tools;

use anyhow::{anyhow, Context, Result};
use colored::*;
use std::{
    env,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use tokio::time::Duration;

use volition_agent_core::{
    config::RuntimeConfig,
    models::chat::ChatMessage, // Keep this
    AgentOutput,              // Import AgentOutput
    ToolProvider,
    Agent,
};

use crate::models::cli::Cli;
use crate::rendering::print_formatted;
use crate::tools::CliToolProvider;

use clap::Parser;
use reqwest::Client;
use std::sync::Arc;
use tracing::{debug, error, info, warn, Level}; // Added debug
use tracing_subscriber::FmtSubscriber;

const CONFIG_FILENAME: &str = "Volition.toml";
const RECOVERY_FILE_PATH: &str = ".conversation_state.json";

// --- Configuration Loading ---

fn find_project_root() -> Result<PathBuf> {
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    let mut current = current_dir.as_path();
    loop {
        let config_path = current.join(CONFIG_FILENAME);
        if config_path.exists() && config_path.is_file() {
            info!("Found configuration file at: {:?}", config_path);
            return Ok(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(anyhow!(
                    "Could not find '{}' in current directory or any parent directory.",
                    CONFIG_FILENAME
                ));
            }
        }
    }
}

fn load_cli_config() -> Result<(RuntimeConfig, PathBuf)> {
    let project_root = find_project_root()?;
    let config_path = project_root.join(CONFIG_FILENAME);
    let config_toml_content = fs::read_to_string(&config_path).with_context(|| {
        format!("Failed to read project config file: {:?}", config_path)
    })?;
    let api_key = env::var("API_KEY")
        .context("Failed to read API_KEY environment variable. Please ensure it is set.")?;

    let runtime_config = RuntimeConfig::from_toml_str(&config_toml_content, api_key)
        .context("Failed to parse or validate configuration content")?;

    Ok((runtime_config, project_root))
}

// --- Session Management ---

fn print_welcome_message() {
    println!("
{}", "Volition - AI Assistant".cyan().bold());
    println!(
        "{}",
        "Type 'exit' or press Enter on an empty line to quit.".cyan()
    );
    println!();
}

// Refined: Doesn't delete recovery file prematurely
fn load_or_initialize_session(
    config: &RuntimeConfig,
    project_root: &Path,
) -> Result<Option<Vec<ChatMessage>>> {
    let recovery_path = project_root.join(RECOVERY_FILE_PATH);
    let mut messages_option: Option<Vec<ChatMessage>> = None;

    if recovery_path.exists() {
        info!("Found existing session state file: {:?}", recovery_path);
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
            match fs::read_to_string(&recovery_path) {
                Ok(state_json) => match serde_json::from_str(&state_json) {
                    Ok(loaded_messages) => {
                        messages_option = Some(loaded_messages);
                        info!("Successfully loaded session state from file.");
                        println!("{}", "Resuming previous session...".cyan());
                        // DO NOT remove recovery file here. Remove only after successful run or clean exit.
                    }
                    Err(e) => {
                        error!("Failed to deserialize state file: {}. Starting fresh.", e);
                        println!("{}", "Error reading state file. Starting fresh.".red());
                        // Optionally remove the corrupted file
                        // let _ = fs::remove_file(&recovery_path);
                    }
                },
                Err(e) => {
                    error!("Failed to read state file: {}. Starting fresh.", e);
                    println!("{}", "Error reading state file. Starting fresh.".red());
                    // Optionally remove the unreadable file
                    // let _ = fs::remove_file(&recovery_path);
                }
            }
        } else {
            info!("User chose not to resume. Starting fresh.");
            println!("{}", "Starting a fresh session.".cyan());
            // DO NOT remove recovery file here. It will be overwritten or removed later.
        }
    }

    // Initialize with system prompt if no session was loaded
    if messages_option.is_none() {
        messages_option = Some(vec![ChatMessage {
            role: "system".to_string(),
            content: Some(config.system_prompt.clone()),
            ..Default::default()
        }]);
        info!("Initialized new session with system prompt.");
    }

    // We just return the messages (or None if user chose 'n' during a failed load maybe - though current logic prevents this)
    // The caller (main) will handle asking for the first goal.
    // We return Option<Vec<ChatMessage>> directly.
    Ok(messages_option)
}


// --- Agent Execution ---
// Uses Agent::run(goal: &str)
async fn run_agent_session(
    config: &RuntimeConfig,
    _client: &Client, // Keep client for potential future use
    tool_provider: Arc<dyn ToolProvider>,
    initial_goal: String, // Takes the goal string
    working_dir: &Path,
) -> Result<AgentOutput> { // Still returns AgentOutput
    // Agent::new requires Arc<dyn ToolProvider>, cloning it here is fine.
    let agent = Agent::new(config.clone(), Arc::clone(&tool_provider))
        .context("Failed to create agent instance")?;

    info!("Starting agent run with goal: {}", initial_goal);
    debug!("Agent config: {:?}, Tool Provider: Arc<dyn ToolProvider>", config); // Example debug

    // Call the original run method which takes goal: &str
    match agent.run(&initial_goal, working_dir).await {
        Ok(agent_output) => {
            info!("Agent run finished successfully.");
            println!("
{}", "--- Agent Run Summary ---".bold());

            if !agent_output.applied_tool_results.is_empty() {
                println!("
{}:", "Tool Execution Results".cyan());
                for result in &agent_output.applied_tool_results { // Borrow result
                    let status_color = match result.status {
                        volition_agent_core::ToolExecutionStatus::Success => "Success".green(),
                        volition_agent_core::ToolExecutionStatus::Failure => "Failure".red(),
                    };
                    println!(
                        "- Tool: {}, Status: {}",
                        result.tool_name.yellow(),
                        status_color
                    );
                }
            }

            if let Some(final_desc) = &agent_output.final_state_description { // Borrow final_desc
                println!("
{}:", "Final AI Message".cyan());
                if let Err(e) = print_formatted(final_desc) {
                    error!(
                        "Failed to render final AI message markdown: {}. Printing raw.",
                        e
                    );
                    println!("{}", final_desc);
                } else {
                    println!(); // Add newline after successful markdown rendering
                }
            } else {
                 warn!("Agent finished but provided no final description in output.");
            }
            println!("-----------------------
");
            Ok(agent_output) // Return the output
        }
        Err(e) => {
            error!("Agent run failed: {:?}", e);
            // Error is printed in the main loop, just propagate it
            Err(e)
        }
    }
}

// --- Cleanup ---

fn cleanup_session_state(project_root: &Path) -> Result<()> {
    let recovery_path = project_root.join(RECOVERY_FILE_PATH);
    if recovery_path.exists() {
        if let Err(e) = fs::remove_file(&recovery_path) {
            warn!("Failed to remove recovery state file on exit: {}", e);
        } else {
            info!("Removed recovery state file on clean exit.");
        }
    }
    Ok(())
}

// --- Main Application Entry Point ---

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };
    // Consider adding file logging sink later if needed
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let (config, project_root) = load_cli_config()
        .context("Failed to load configuration and find project root")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(120)) // Increased timeout slightly
        .build()
        .context("Failed to build HTTP client")?;

    // Create Arc<dyn ToolProvider> explicitly here
    let tool_provider: Arc<dyn ToolProvider> = Arc::new(CliToolProvider::new());


    // Load existing session or initialize a new one
    let mut messages = match load_or_initialize_session(&config, &project_root)? {
        Some(msgs) => msgs,
        None => {
            // This case should ideally not happen with the current logic of load_or_initialize_session
            // unless the user explicitly exits during a failed recovery, which isn't implemented yet.
             error!("Failed to load or initialize session messages.");
             return Err(anyhow!("Session initialization failed"));
        }
    };

    // Ensure messages always starts with the system prompt if somehow empty after loading/init
    if messages.is_empty() || messages[0].role != "system" {
         warn!("Messages list was empty or missing system prompt after init. Re-initializing.");
         messages = vec![ChatMessage {
             role: "system".to_string(),
             content: Some(config.system_prompt.clone()),
             ..Default::default()
         }];
    }

    print_welcome_message();

    // Main interaction loop
    loop {
        println!("
{}", "What is your request? (or type 'exit')".cyan());
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        let trimmed_input = user_input.trim();

        if trimmed_input.is_empty() || trimmed_input.to_lowercase() == "exit" {
            break; // Exit the loop
        }

        // Add user message to history
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(trimmed_input.to_string()),
            ..Default::default()
        });

        // Save state *before* the agent run
        let recovery_path = project_root.join(RECOVERY_FILE_PATH);
        match serde_json::to_string_pretty(&messages) { // Use pretty for readability
            Ok(state_json) => {
                if let Err(e) = fs::write(&recovery_path, state_json) {
                    warn!("Failed to write recovery state file: {}", e);
                    // Decide if we should abort or just warn? For now, warn and continue.
                } else {
                    info!("Saved conversation state to {:?}", recovery_path);
                }
            }
            Err(e) => {
                // This is more serious, serialization failed.
                error!("Failed to serialize conversation state: {}. Cannot guarantee recovery.", e);
                // Maybe abort here? For now, log error and continue.
            }
        }

        let current_goal = trimmed_input.to_string(); // Define goal from input

        // Now Arc::clone(&tool_provider) should work as it's cloning an Arc<dyn ToolProvider>
        // Run the agent with the current user request as the goal
        match run_agent_session(
            &config,
            &client, // Pass client reference
            Arc::clone(&tool_provider), // Pass the cloned Arc<dyn ToolProvider>
            current_goal,     // Pass the user's latest request string
            &project_root,
        )
        .await // Don't forget await!
        {
            Ok(agent_output) => {
                 // Agent run succeeded, add assistant response to history
                 if let Some(final_desc) = agent_output.final_state_description {
                     messages.push(ChatMessage {
                         role: "assistant".to_string(),
                         content: Some(final_desc),
                         // TODO: Potentially add tool_calls from agent_output if available/needed
                         ..Default::default()
                     });
                 } else {
                     // Agent succeeded but gave no text response. Don't add empty message.
                     // Logged inside run_agent_session
                 }

                 // Clean up the recovery file now that this turn is successfully completed
                 if recovery_path.exists() {
                     if let Err(e) = fs::remove_file(&recovery_path) {
                         warn!("Failed to remove recovery state file after successful run: {}", e);
                     } else {
                         info!("Removed recovery state file after successful run.");
                     }
                 }
            }
            Err(e) => {
                // Agent run failed. Print error and let the loop continue.
                // Do NOT remove the recovery file, it holds the state *before* the failed run.
                println!("
{}: {:?}
", "Agent run encountered an error".red(), e);
                // Remove the last user message we optimistically added, so they can retry
                // or ask something else based on the previous state.
                messages.pop(); // Remove the user message that led to the error
                info!("Removed last user message from history due to agent error.");
            }
        }
        // Loop continues for the next user input
    }

    // Cleanup recovery file only on clean exit from the loop
    let _ = cleanup_session_state(&project_root);
    println!("
{}
", "Goodbye!".cyan());
    Ok(())
}

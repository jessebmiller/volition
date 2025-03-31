// volition-cli/src/main.rs
mod models;
mod rendering;
mod tools;

use anyhow::{anyhow, Context, Result};
use colored::*;
use std::{
    env,
    fs, // Added fs import back
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::time::Duration;

// Updated imports
use volition_agent_core::{
    async_trait,
    config::RuntimeConfig,
    errors::AgentError,                              // Added AgentError
    strategies::complete_task::CompleteTaskStrategy, // Added Strategy
    Agent,
    ToolProvider,
    UserInteraction,
};

use crate::models::cli::Cli;
use crate::rendering::print_formatted;
use crate::tools::CliToolProvider;

use clap::Parser;
use reqwest::Client;
use time::macros::format_description;
use tracing::{debug, error, info, trace, Level}; // Removed warn
use tracing_subscriber::{fmt::time::LocalTime, EnvFilter};

const CONFIG_FILENAME: &str = "Volition.toml";
// const RECOVERY_FILE_PATH: &str = ".conversation_state.json"; // Removed recovery logic

/// Simple struct to handle CLI user interactions.
struct CliUserInteraction;

#[async_trait]
impl UserInteraction for CliUserInteraction {
    /// Asks the user a question via the command line.
    async fn ask(&self, prompt: String, _options: Vec<String>) -> Result<String> {
        // Keep existing ask implementation
        print!("{}", prompt.yellow().bold());
        io::stdout().flush().context("Failed to flush stdout")?;

        let mut buffer = String::new();
        io::stdin()
            .read_line(&mut buffer)
            .context("Failed to read line from stdin")?;

        Ok(buffer.trim().to_string())
    }
}

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
    let config_toml_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read project config file: {:?}", config_path))?;
    let api_key = env::var("API_KEY")
        .context("Failed to read API_KEY environment variable. Please ensure it is set.")?;

    let runtime_config = RuntimeConfig::from_toml_str(&config_toml_content, api_key)
        .context("Failed to parse or validate configuration content")?;

    Ok((runtime_config, project_root))
}

fn print_welcome_message() {
    println!(
        "
{}",
        "Volition - AI Assistant".cyan().bold()
    );
    println!(
        "{}",
        "Type 'exit' or press Enter on an empty line to quit.".cyan()
    );
    println!();
}

// Removed load_or_initialize_session function

// Refactored run_agent_session
async fn run_agent_session(
    config: &RuntimeConfig,
    _client: &Client, // Keep client in case needed later
    tool_provider: Arc<dyn ToolProvider>,
    ui_handler: Arc<CliUserInteraction>,
    initial_task: String,
    working_dir: &Path,
) -> Result<String, AgentError> {
    // Return AgentError

    // Create agent with strategy and initial task
    let mut agent = Agent::new(
        config.clone(),
        tool_provider, // No need to clone Arc here
        ui_handler,    // No need to clone Arc here
        Box::new(CompleteTaskStrategy::new()),
        initial_task,
    )
    // Map the anyhow::Result from Agent::new to AgentError if needed
    // Assuming Agent::new returns anyhow::Result for now.
    .map_err(|e| AgentError::Other(format!("Failed to create agent instance: {}", e)))?;

    info!("Starting agent run.");
    debug!(
        "Agent config: {:?}, Tool Provider: Arc<dyn ToolProvider>, Strategy: CompleteTask",
        config
    );

    // Call the refactored run method
    match agent.run(working_dir).await {
        Ok(final_message) => {
            info!("Agent run finished successfully.");
            println!("{}", "--- Agent Response ---".bold());
            if let Err(e) = print_formatted(&final_message) {
                error!(
                    "Failed to render final AI message markdown: {}. Printing raw.",
                    e
                );
                println!("{}", final_message); // Print raw on error
            } else {
                println!(); // Add newline after successful markdown print
            }
            println!("----------------------");
            Ok(final_message) // Return the final message
        }
        Err(e) => {
            error!(
                "Agent run failed: {:?}
",
                e
            ); // Added newline for clarity
               // Log the error, return it
            Err(e)
        }
    }
}

// Removed cleanup_session_state function

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // --- Logging Setup (unchanged) ---
    let default_level = match cli.verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::default().add_directive(default_level.into()));
    let time_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let local_timer = LocalTime::new(time_format);
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_timer(local_timer)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    info!(
        "Logging initialized. Level determined by RUST_LOG or -v flags (default: {}).",
        default_level
    );
    debug!("Debug logging enabled.");
    trace!("Trace logging enabled.");
    // --- End Logging Setup ---

    let (config, project_root) =
        load_cli_config().context("Failed to load configuration and find project root")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(120)) // Example timeout
        .build()
        .context("Failed to build HTTP client")?;

    let tool_provider: Arc<dyn ToolProvider> = Arc::new(CliToolProvider::new());
    let ui_handler = Arc::new(CliUserInteraction);

    // Removed max_iterations logic
    // Removed message history loading/initialization

    print_welcome_message();

    // Simplified main loop
    loop {
        println!("{}", "How can I help you?".cyan());
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        let trimmed_input = user_input.trim();

        if trimmed_input.is_empty() || trimmed_input.to_lowercase() == "exit" {
            break;
        }

        // Removed saving conversation state

        // Call the refactored agent session function for each input
        match run_agent_session(
            &config,
            &client,
            Arc::clone(&tool_provider),
            Arc::clone(&ui_handler),
            trimmed_input.to_string(), // Pass user input as the initial task
            &project_root,
        )
        .await
        {
            Ok(_) => {
                // Success message is already printed within run_agent_session
                info!("Agent session completed successfully for user input.");
            }
            Err(e) => {
                // Error is logged in run_agent_session, print user-facing message
                println!(
                    "{}: {:?}
", // Added newline for clarity
                    "Agent run encountered an error".red(),
                    e // Display the specific AgentError
                );
            }
        }
        // Removed adding assistant response to message list
        // Removed recovery file cleanup on success
    }

    // Removed final cleanup call
    println!("{}", "Thanks!".cyan());
    Ok(())
}

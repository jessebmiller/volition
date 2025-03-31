// volition-cli/src/main.rs
mod models;
mod rendering;
mod tools;

// Simplified imports
use anyhow::{anyhow, Context, Result};
use colored::*;
// Break down the std use statement
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf; // Removed unused Path
use std::sync::Arc;
// use tokio::time::Duration; // Removed unused Duration

// Import items directly from volition_agent_core
use volition_agent_core::async_trait;
use volition_agent_core::config::RuntimeConfig;
use volition_agent_core::errors::AgentError;
use volition_agent_core::strategies::complete_task::CompleteTaskStrategy;
use volition_agent_core::strategies::conversation::ConversationStrategy;
use volition_agent_core::{Agent, AgentState, ToolProvider, UserInteraction};


use crate::models::cli::Cli;
use crate::rendering::print_formatted;
use crate::tools::CliToolProvider;

use clap::Parser;
// use reqwest::Client; // Removed unused Client
use time::macros::format_description;
use tracing::{debug, error, info, trace, Level};
use tracing_subscriber::{fmt::time::LocalTime, EnvFilter};

const CONFIG_FILENAME: &str = "Volition.toml";

/// Simple struct to handle CLI user interactions.
struct CliUserInteraction;

#[async_trait]
impl UserInteraction for CliUserInteraction {
    /// Asks the user a question via the command line.
    async fn ask(&self, prompt: String, _options: Vec<String>) -> Result<String> {
        print!("{} ", prompt.yellow().bold());
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
        "\n{}",
        "Volition - AI Assistant".cyan().bold()
    );
    println!(
        "{}\n{}",
        "Type 'exit' or press Enter on an empty line to quit.".cyan(),
        "Type 'new' to start a fresh conversation.".cyan()
    );
    println!();
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

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

    let (config, project_root) =
        load_cli_config().context("Failed to load configuration and find project root")?;

    let tool_provider: Arc<dyn ToolProvider> = Arc::new(CliToolProvider::new());
    let ui_handler = Arc::new(CliUserInteraction);

    print_welcome_message();

    let mut conversation_state: Option<AgentState> = None;

    loop {
        println!("\n{}", "How can I help you?".cyan());
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        let trimmed_input = user_input.trim();

        if trimmed_input.is_empty() || trimmed_input.to_lowercase() == "exit" {
            break;
        }

        if trimmed_input.to_lowercase() == "new" {
            println!("{}", "Starting a new conversation...".cyan());
            conversation_state = None;
            continue;
        }

        let mut agent = {
            let inner_strategy = Box::new(CompleteTaskStrategy::new());
            let conversation_strategy: Box<dyn volition_agent_core::Strategy + Send + Sync> =
                if let Some(state) = conversation_state.take() {
                    info!("Continuing conversation.");
                    Box::new(ConversationStrategy::with_state(inner_strategy, state))
                } else {
                    info!("Starting new conversation.");
                    Box::new(ConversationStrategy::new(inner_strategy))
                };

            Agent::new(
                config.clone(),
                Arc::clone(&tool_provider),
                Arc::clone(&ui_handler),
                conversation_strategy,
                trimmed_input.to_string(),
            )
            .map_err(|e| AgentError::Other(format!("Failed to create agent instance: {}", e)))?
        };

        match agent.run(&project_root).await {
            Ok((final_message, updated_state)) => {
                info!("Agent session completed successfully for user input.");
                println!("{}", "--- Agent Response ---".bold());
                if let Err(e) = print_formatted(&final_message) {
                    error!(
                        "Failed to render final AI message markdown: {}. Printing raw.",
                        e
                    );
                    println!("{}", final_message);
                } else {
                    println!();
                }
                println!("----------------------");

                conversation_state = Some(updated_state);
            }
            Err(e) => {
                println!(
                    "{}: {:?}\n",
                    "Agent run encountered an error".red(),
                    e
                );
            }
        }
    }

    println!("\n{}", "Thanks!".cyan());
    Ok(())
}

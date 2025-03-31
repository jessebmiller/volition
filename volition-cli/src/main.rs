"""// volition-cli/src/main.rs
mod models;
mod rendering;
// mod tools; // Old tool provider removed

use anyhow::{anyhow, Context, Result};
use colored::*;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

// Import new core types
use volition_agent_core::{
    agent::Agent, // Correct path
    config::AgentConfig, // Use AgentConfig
    errors::AgentError,
    // Import necessary strategies
    strategies::{
        complete_task::CompleteTaskStrategy, // Assuming this still exists/is useful
        conversation::ConversationStrategy,
        plan_execute::PlanExecuteStrategy, // Import new strategy
        StrategyConfig, // Import StrategyConfig
    },
    // ToolProvider is removed
    AgentState,
    UserInteraction, // Keep UserInteraction trait
    async_trait, // Keep async_trait
};

use crate::models::cli::Cli;
use crate::rendering::print_formatted;
// Removed CliToolProvider import

use clap::Parser;
use time::macros::format_description;
use tracing::{debug, error, info, trace, Level};
use tracing_subscriber::{fmt::time::LocalTime, EnvFilter};

const CONFIG_FILENAME: &str = "Volition.toml";

struct CliUserInteraction;

#[async_trait]
impl UserInteraction for CliUserInteraction {
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

// Load new AgentConfig
fn load_cli_config() -> Result<(AgentConfig, PathBuf)> {
    let project_root = find_project_root()?;
    let config_path = project_root.join(CONFIG_FILENAME);
    let config_toml_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read project config file: {:?}", config_path))?;

    // API keys are now handled during Agent creation based on provider config
    let agent_config = AgentConfig::from_toml_str(&config_toml_content)
        .context("Failed to parse or validate configuration content")?;

    Ok((agent_config, project_root))
}

fn print_welcome_message() {
    println!(
        "
{}",
        "Volition - AI Assistant (MCP Refactor)".cyan().bold() // Updated title
    );
    println!(
        "{}
{}",
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

    // ToolProvider is removed, tools are accessed via MCP
    let ui_handler = Arc::new(CliUserInteraction);

    print_welcome_message();

    let mut conversation_state: Option<AgentState> = None;

    // --- Select Strategy ---
    // For now, let's default to PlanExecuteStrategy if configured, otherwise CompleteTask
    let strategy_name = "plan_execute"; // Or get from CLI args?
    let strategy_config = config.strategies.get(strategy_name)
        .cloned()
        .unwrap_or_else(|| {
            warn!("Strategy '{}' not found in config, using default.", strategy_name);
            // Create a default StrategyConfig if needed, or handle error
             StrategyConfig { planning_provider: None, execution_provider: None }
        });

    // Create the chosen strategy instance
    // Need to handle UI generic parameter correctly
    // We need to define the Agent type first to know the UI type for the Strategy trait object
    type CliAgent = Agent<CliUserInteraction>; // Define agent type with concrete UI

    let mut strategy_instance: Box<dyn volition_agent_core::Strategy<CliUserInteraction> + Send + Sync> =
        if strategy_name == "plan_execute" {
             // Ensure required providers are configured for PlanExecute
             if strategy_config.planning_provider.is_none() || strategy_config.execution_provider.is_none() {
                 error!("'plan_execute' strategy requires 'planning_provider' and 'execution_provider' in config.");
                 // Fallback to CompleteTask or return error?
                 warn!("Falling back to 'CompleteTask' strategy.");
                 Box::new(CompleteTaskStrategy::default())
             } else {
                 info!("Using PlanExecute strategy.");
                 Box::new(PlanExecuteStrategy::new(strategy_config))
             }
        } else {
            info!("Using CompleteTask strategy.");
            Box::new(CompleteTaskStrategy::default())
        };


    loop {
        println!("
{}", "How can I help you?".cyan());
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
            // Re-initialize strategy? Or assume Agent::new handles it?
            // For simplicity, let Agent::new handle state reset.
            continue;
        }

        // --- Agent Creation ---
        // We now create the agent inside the loop if conversation_state is None,
        // otherwise we need a way to update the agent's state or create a new one
        // with the existing history. The current Agent::new doesn't support history.
        // Let's simplify: always create a new agent, but potentially pass history
        // to the strategy initialization if we adapt ConversationStrategy.

        // For now, using the selected strategy directly without ConversationStrategy wrapper
        let mut agent = CliAgent::new( // Use type alias CliAgent
            config.clone(), // Clone config for each loop iteration
            Arc::clone(&ui_handler),
            strategy_instance, // Pass the strategy instance directly
            trimmed_input.to_string(),
        )
        .context("Failed to create agent instance")?; // Use context for better error

        // Agent::run now takes working_dir, which we have
        match agent.run(&project_root).await {
            Ok((final_message, _updated_state)) => { // Ignore updated_state for now
                info!("Agent session completed successfully.");
                println!("{}", "--- Agent Response ---".bold());
                if let Err(e) = print_formatted(&final_message) {
                    error!(
                        "Failed to render final AI message markdown: {}. Printing raw.",
                        e
                    );
                    println!("{}", final_message);
                } else {
                    println!(); // Add newline after formatted output
                }
                println!("----------------------");

                // Re-assign the strategy instance back for the next loop iteration
                // This assumes the strategy's internal state is correctly managed across runs
                strategy_instance = agent.strategy;

                // TODO: Handle conversation history persistence if needed
                // conversation_state = Some(updated_state);
            }
            Err(e) => {
                // Use Display formatting for AgentError
                println!(
                    "{}: {}
", // Use Display format
                    "Agent run encountered an error".red(),
                    e
                );
                // Optionally reset strategy or state on error?
                 // For now, just continue the loop.
            }
        }
    }

    println!("
{}", "Thanks!".cyan());
    Ok(())
}

""
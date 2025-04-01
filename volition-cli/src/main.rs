// volition-cli/src/main.rs
mod models;
mod rendering;

use anyhow::{anyhow, Context, Result};
use colored::*;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode; // For returning error codes
use std::sync::Arc;

// Import new core types
use volition_agent_core::{
    agent::Agent,
    config::AgentConfig,
    errors::AgentError,
    strategies::{
        complete_task::CompleteTaskStrategy,
        conversation::ConversationStrategy,
        plan_execute::PlanExecuteStrategy,
    },
    UserInteraction, async_trait, ChatMessage,
};

use crate::models::cli::Cli;
use crate::rendering::print_formatted;

use clap::Parser;
use time::macros::format_description;
use tracing::{debug, error, info, trace, Level, warn};
use tracing_subscriber::{fmt::time::LocalTime, EnvFilter};

const CONFIG_FILENAME: &str = "Volition.toml";

// Define agent type with concrete UI
type CliAgent = Agent<CliUserInteraction>;
// Define strategy trait object type with concrete UI
type CliStrategy = Box<dyn volition_agent_core::Strategy<CliUserInteraction> + Send + Sync>;

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

fn load_cli_config() -> Result<(AgentConfig, PathBuf)> {
    let project_root = find_project_root()?;
    let config_path = project_root.join(CONFIG_FILENAME);
    let config_toml_content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read project config file: {:?}", config_path))?;
    let agent_config = AgentConfig::from_toml_str(&config_toml_content)
        .context("Failed to parse or validate configuration content")?;
    Ok((agent_config, project_root))
}

fn print_welcome_message() {
    println!(
        "
{}",
        "Volition - AI Assistant (MCP Refactor)".cyan().bold()
    );
    println!(
        "{}
{}",
        "Type 'exit' or press Enter on an empty line to quit.".cyan(),
        "Type 'new' to start a fresh conversation.".cyan()
    );
    println!();
}

/// Selects the base strategy based on config and potentially CLI args (future).
fn select_base_strategy(config: &AgentConfig) -> CliStrategy {
    // TODO: Allow selecting strategy via CLI arg or config
    let strategy_name = "plan_execute"; // Hardcoded for now

    if strategy_name == "plan_execute" {
        match config.strategies.get(strategy_name) {
            Some(strategy_config)
                if strategy_config.planning_provider.is_some()
                    && strategy_config.execution_provider.is_some() =>
            {
                info!("Using PlanExecute strategy with provided config.");
                Box::new(PlanExecuteStrategy::new(strategy_config.clone()))
            }
            _ => {
                warn!("'plan_execute' strategy selected but config is missing or incomplete. Falling back to 'CompleteTask'.");
                info!("Using CompleteTask strategy.");
                Box::new(CompleteTaskStrategy::default())
            }
        }
    } else {
        // Default to CompleteTask if plan_execute isn't the chosen strategy
        info!("Using CompleteTask strategy.");
        Box::new(CompleteTaskStrategy::default())
    }
}

/// Runs the agent non-interactively for a single task.
async fn run_non_interactive(
    task: String,
    config: AgentConfig,
    project_root: PathBuf,
    ui_handler: Arc<CliUserInteraction>,
) -> Result<()> {
    info!(task = %task, "Running non-interactive task.");

    let base_strategy = select_base_strategy(&config);

    // --- Agent Creation (No Conversation Wrapper) ---
    // Fix: Add None for override arguments
    let mut agent = CliAgent::new(
        config.clone(),
        ui_handler,
        base_strategy, // Use the base strategy directly
        task,
        None, // provider_registry_override
        None, // mcp_connections_override
    )
    .map_err(|e| AgentError::Config(format!("Failed to create agent instance: {}", e)))?;

    match agent.run(&project_root).await {
        Ok((final_message, _updated_state)) => {
            info!("Agent session completed successfully.");
            println!("{}", "--- Agent Response ---".bold());
            // Print raw output in non-interactive mode for easier parsing/piping
            println!("{}", final_message);
            println!("----------------------");
            Ok(()) // Indicate success
        }
        Err(e) => {
            error!("Agent run encountered an error: {}", e);
            // Return the error to indicate failure
            Err(anyhow!(e))
        }
    }
}

/// Runs the agent in interactive mode.
async fn run_interactive(
    config: AgentConfig,
    project_root: PathBuf,
    ui_handler: Arc<CliUserInteraction>,
) -> Result<()> {
    print_welcome_message();
    // We only store the message history for conversation strategy.
    // PlanExecute manages its own state per run.
    let mut conversation_messages: Option<Vec<ChatMessage>> = None;

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
            conversation_messages = None;
            continue;
        }

        let user_message = trimmed_input.to_string();
        let base_strategy = select_base_strategy(&config);
        let is_plan_execute = base_strategy.name() == "PlanExecute";

        // Wrap with ConversationStrategy only if it's NOT PlanExecute
        let agent_strategy: CliStrategy = if !is_plan_execute {
             if let Some(messages) = conversation_messages.take() {
                info!("Continuing conversation.");
                Box::new(ConversationStrategy::with_history(
                    base_strategy,
                    messages, // Pass previous messages
                ))
             } else {
                info!("Starting new conversation.");
                Box::new(ConversationStrategy::new(base_strategy))
             }
        } else {
            info!("Using PlanExecute strategy directly (no conversation history passed).");
            base_strategy // Use PlanExecute directly
        };

        // --- Agent Creation ---
        // Fix: Add None for override arguments
        let mut agent = CliAgent::new(
            config.clone(),
            Arc::clone(&ui_handler),
            agent_strategy, // Pass the potentially wrapped strategy
            user_message.clone(), // Pass the user input as the initial task
            None, // provider_registry_override
            None, // mcp_connections_override
        )
        .map_err(|e| AgentError::Config(format!("Failed to create agent instance: {}", e)))?;

        match agent.run(&project_root).await {
            Ok((final_message, updated_state)) => {
                info!("Agent session completed successfully.");
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
                // Store message history only if not using PlanExecute
                if !is_plan_execute {
                    conversation_messages = Some(updated_state.messages);
                } else {
                     // Discard state for PlanExecute, it starts fresh each time
                     conversation_messages = None;
                }
            }
            Err(e) => {
                println!(
                    "{}: {}
",
                    "Agent run encountered an error".red(),
                    e // Display AgentError directly
                );
                // Reset conversation history on error
                conversation_messages = None;
            }
        }
    }
    println!("
{}", "Thanks!".cyan());
    Ok(())
}

// Use Tokio main, but return ExitCode on error
#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // Initialize logging (same as before)
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
        .with_target(false) // Don't include module paths
        .with_timer(local_timer)
        .finish();
    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
         eprintln!("Failed to set global tracing subscriber: {}", e);
         return ExitCode::FAILURE;
    }
    info!(
        "Logging initialized. Level determined by RUST_LOG or -v flags (default: {}).",
        default_level
    );
    debug!("Debug logging enabled.");
    trace!("Trace logging enabled.");


    // Load config (common step)
    let (config, project_root) = match load_cli_config() {
         Ok(c) => c,
         Err(e) => {
              error!("Failed to load configuration: {}", e);
              return ExitCode::FAILURE;
         }
    };

    let ui_handler: Arc<CliUserInteraction> = Arc::new(CliUserInteraction);

    // Decide mode based on --task flag
    let result = if let Some(task) = cli.task {
        run_non_interactive(task, config, project_root, ui_handler).await
    } else {
        run_interactive(config, project_root, ui_handler).await
    };

    // Return appropriate exit code
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            // Error should have already been logged by run_non_interactive/run_interactive
            eprintln!("Operation failed: {}", e); // Print final error summary
            ExitCode::FAILURE
        }
    }
}

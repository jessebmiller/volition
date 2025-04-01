// volition-cli/src/main.rs
mod models;
mod rendering;

use anyhow::{anyhow, Context, Result};
use colored::*;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use volition_agent_core::{
    agent::Agent,
    config::AgentConfig,
    errors::AgentError,
    strategies::{
        complete_task::CompleteTaskStrategy,
        // Removed: conversation::ConversationStrategy,
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

type CliAgent = Agent<CliUserInteraction>;
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

fn select_base_strategy(config: &AgentConfig) -> CliStrategy {
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
        info!("Using CompleteTask strategy.");
        Box::new(CompleteTaskStrategy::default())
    }
}

async fn run_non_interactive(
    task: String,
    config: AgentConfig,
    project_root: PathBuf,
    ui_handler: Arc<CliUserInteraction>,
) -> Result<()> {
    info!(task = %task, "Running non-interactive task.");

    let base_strategy = select_base_strategy(&config);

    // Call Agent::new with None history
    let mut agent = CliAgent::new(
        config.clone(),
        ui_handler,
        base_strategy,
        None, // history
        task, // current_user_input
        None, // provider_registry_override
        None, // mcp_connections_override
    )
    .map_err(|e| AgentError::Config(format!("Failed to create agent instance: {}", e)))?;

    match agent.run(&project_root).await {
        Ok((final_message, _updated_state)) => {
            info!("Agent session completed successfully.");
            println!("{}", "--- Agent Response ---".bold());
            println!("{}", final_message);
            println!("----------------------");
            Ok(())
        }
        Err(e) => {
            error!("Agent run encountered an error: {}", e);
            Err(anyhow!(e))
        }
    }
}

async fn run_interactive(
    config: AgentConfig,
    project_root: PathBuf,
    ui_handler: Arc<CliUserInteraction>,
) -> Result<()> {
    print_welcome_message();
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
        let agent_strategy = select_base_strategy(&config);

        // Agent::new now handles history initialization via AgentState::new_turn
        let mut agent = CliAgent::new(
            config.clone(),
            Arc::clone(&ui_handler),
            agent_strategy,
            conversation_messages.take(),
            user_message.clone(),
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
                // Always store the message history returned by the agent
                conversation_messages = Some(updated_state.messages);
            }
            Err(e) => {
                println!(
                    "{}: {}
",
                    "Agent run encountered an error".red(),
                    e
                );
                conversation_messages = None;
            }
        }
    }
    println!("
{}", "Thanks!".cyan());
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
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


    let (config, project_root) = match load_cli_config() {
         Ok(c) => c,
         Err(e) => {
              error!("Failed to load configuration: {}", e);
              return ExitCode::FAILURE;
         }
    };

    let ui_handler: Arc<CliUserInteraction> = Arc::new(CliUserInteraction);

    let result = if let Some(task) = cli.task {
        // Pass None history for non-interactive mode
        run_non_interactive(task, config, project_root, ui_handler).await
    } else {
        run_interactive(config, project_root, ui_handler).await
    };

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Operation failed: {}", e);
            ExitCode::FAILURE
        }
    }
}

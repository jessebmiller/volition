// volition-cli/src/main.rs
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

use volition_agent_core::{
    config::{load_runtime_config, RuntimeConfig},
    // Updated import: ResponseMessage -> ChatMessage
    models::chat::ChatMessage,
    ToolProvider,
    Agent,
};

use crate::models::cli::Cli;
use crate::rendering::print_formatted;
use crate::tools::CliToolProvider;

use clap::Parser;
use reqwest::Client;
use std::sync::Arc;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

const RECOVERY_FILE_PATH: &str = ".conversation_state.json";

fn print_welcome_message() {
    println!("\n{}", "Volition - AI Assistant".cyan().bold());
    println!(
        "{}",
        "Type \'exit\' or press Enter on an empty line to quit.".cyan()
    );
    println!();
}

/// Returns Ok(Some((messages, goal))) or Ok(None) if user exits.
fn load_or_initialize_session(
    config: &RuntimeConfig,
    // Updated type: ResponseMessage -> ChatMessage
) -> Result<Option<(Vec<ChatMessage>, String)>> {
    let recovery_path = Path::new(RECOVERY_FILE_PATH);
    // Updated type: ResponseMessage -> ChatMessage
    let mut messages_option: Option<Vec<ChatMessage>> = None;
    let initial_goal: Option<String>;

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
                    // Updated type: ResponseMessage -> ChatMessage
                    Ok(loaded_messages) => {
                        messages_option = Some(loaded_messages);
                        info!("Successfully resumed session from state file.");
                        println!("{}", "Resuming previous session...".cyan());
                        let _ = fs::remove_file(recovery_path);
                    }
                    Err(e) => {
                        error!("Failed to deserialize state file: {}. Starting fresh.", e);
                        println!("{}", "Error reading state file. Starting fresh.".red());
                        let _ = fs::remove_file(recovery_path);
                    }
                },
                Err(e) => {
                    error!("Failed to read state file: {}. Starting fresh.", e);
                    println!("{}", "Error reading state file. Starting fresh.".red());
                    let _ = fs::remove_file(recovery_path);
                }
            }
        } else {
            info!("User chose not to resume. Starting fresh.");
            println!("{}", "Starting a fresh session.".cyan());
            let _ = fs::remove_file(recovery_path);
        }
    }

    if messages_option.is_none() {
        println!("{}", "How can I help you?".cyan());
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;
        let mut initial_input = String::new();
        io::stdin().read_line(&mut initial_input)?;
        let trimmed_input = initial_input.trim();

        if trimmed_input.is_empty() || trimmed_input.to_lowercase() == "exit" {
            return Ok(None);
        }
        initial_goal = Some(trimmed_input.to_string());
        // Use ChatMessage
        messages_option = Some(vec![ChatMessage {
            role: "system".to_string(),
            content: Some(config.system_prompt.clone()),
            ..Default::default()
        }]);
    } else {
        println!("{}", "What is the main goal for this resumed session?".cyan());
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;
        let mut goal_input = String::new();
        io::stdin().read_line(&mut goal_input)?;
        let trimmed_input = goal_input.trim();
        if trimmed_input.is_empty() {
             println!("{}", "Goal cannot be empty. Exiting.".red());
             return Ok(None);
        }
        initial_goal = Some(trimmed_input.to_string());
    }

    if let (Some(messages), Some(goal)) = (messages_option, initial_goal) {
        Ok(Some((messages, goal)))
    } else {
        error!("Failed to establish initial messages and goal.");
        Ok(None)
    }
}

async fn run_agent_session(
    config: &RuntimeConfig,
    _client: &Client,
    tool_provider: Arc<dyn ToolProvider>,
    _initial_messages: Vec<ChatMessage>, // Updated type
    initial_goal: String,
) -> Result<()> {
    let agent = Agent::new(config.clone(), Arc::clone(&tool_provider))
        .context("Failed to create agent instance")?;

    info!("Starting agent run with goal: {}", initial_goal);
    let working_dir = &config.project_root;

    match agent.run(&initial_goal, working_dir).await {
        Ok(agent_output) => {
            info!("Agent run finished successfully.");
            println!("\n{}", "--- Agent Run Summary ---".bold());

            if let Some(summary) = agent_output.suggested_summary {
                println!("{}:", "Suggested Summary".cyan());
                println!("{}", summary);
            }

            if !agent_output.applied_tool_results.is_empty() {
                println!("\n{}:", "Tool Execution Results".cyan());
                for result in agent_output.applied_tool_results {
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

            if let Some(final_desc) = agent_output.final_state_description {
                 println!("\n{}:", "Final AI Message".cyan());
                 if let Err(e) = print_formatted(&final_desc) {
                    error!("Failed to render final AI message markdown: {}. Printing raw.", e);
                    println!("{}", final_desc);
                 } else {
                    println!();
                 }
            }
            println!("-----------------------\n");
        }
        Err(e) => {
            error!("Agent run failed: {:?}", e);
            println!("{}", "Agent run encountered an error:".red());
            println!("{:?}", e);
            return Err(e);
        }
    }
    cleanup_session_state()
}

fn cleanup_session_state() -> Result<()> {
    let recovery_path = Path::new(RECOVERY_FILE_PATH);
    if recovery_path.exists() {
        if let Err(e) = fs::remove_file(recovery_path) {
            warn!("Failed to remove recovery state file on exit: {}", e);
        } else {
            info!("Removed recovery state file on clean exit.");
        }
    }
    Ok(())
}

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
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = load_runtime_config()
        .context("Failed to load configuration from Volition.toml and environment")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("Failed to build HTTP client")?;

    let tool_provider = Arc::new(CliToolProvider::new(client.clone()));

    print_welcome_message();

    match load_or_initialize_session(&config)? {
        Some((initial_messages, initial_goal)) => {
            if let Err(_e) = run_agent_session(
                &config,
                &client,
                tool_provider,
                initial_messages,
                initial_goal,
            )
            .await
            {
                std::process::exit(1);
            }
        }
        None => {
            println!("\n{}", "Goodbye!".cyan());
        }
    }

    Ok(())
}

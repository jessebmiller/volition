// volition-cli/src/main.rs
mod models;
mod rendering;
mod tools;

use anyhow::{anyhow, Context, Result};
use colored::*;
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use tokio::time::Duration;

use volition_agent_core::{
    async_trait, config::RuntimeConfig, models::chat::ChatMessage, Agent, AgentOutput,
    ToolProvider, UserInteraction,
};

use crate::models::cli::Cli;
use crate::rendering::print_formatted;
use crate::tools::CliToolProvider;

use clap::Parser;
use reqwest::Client;
use std::sync::Arc;
use tracing::{debug, error, info, warn}; // Removed Level import
use tracing_subscriber::EnvFilter; // Import EnvFilter

const CONFIG_FILENAME: &str = "Volition.toml";
const RECOVERY_FILE_PATH: &str = ".conversation_state.json";

/// Simple struct to handle CLI user interactions.
struct CliUserInteraction;

#[async_trait]
impl UserInteraction for CliUserInteraction {
    /// Asks the user a question via the command line.
    /// The prompt should ideally include formatting like "[Y/n]".
    async fn ask(&self, prompt: String, _options: Vec<String>) -> Result<String> {
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
    println!("\n{}", "Volition - AI Assistant".cyan().bold());
    println!(
        "{}",
        "Type 'exit' or press Enter on an empty line to quit.".cyan()
    );
    println!();
}

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
            "An incomplete session state was found. Resume? [Y/n]: "
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
                    }
                    Err(e) => {
                        error!("Failed to deserialize state file: {}. Starting fresh.", e);
                        println!("{}", "Error reading state file. Starting fresh.".red());
                    }
                },
                Err(e) => {
                    error!("Failed to read state file: {}. Starting fresh.", e);
                    println!("{}", "Error reading state file. Starting fresh.".red());
                }
            }
        } else {
            info!("User chose not to resume. Starting fresh.");
            println!("{}", "Starting a fresh session.".cyan());
        }
    }

    if messages_option.is_none() {
        messages_option = Some(vec![ChatMessage {
            role: "system".to_string(),
            content: Some(config.system_prompt.clone()),
            ..Default::default()
        }]);
        info!("Initialized new session with system prompt.");
    }

    Ok(messages_option)
}

async fn run_agent_session(
    config: &RuntimeConfig,
    _client: &Client,
    tool_provider: Arc<dyn ToolProvider>,
    messages: Vec<ChatMessage>,
    working_dir: &Path,
    max_iterations: usize,
    ui_handler: Arc<CliUserInteraction>,
) -> Result<AgentOutput> {
    let agent = Agent::new(
        config.clone(),
        Arc::clone(&tool_provider),
        ui_handler,
        max_iterations,
    )
    .context("Failed to create agent instance")?;

    info!("Starting agent run with {} messages", messages.len());
    debug!(
        "Agent config: {:?}, Tool Provider: Arc<dyn ToolProvider>, Max Iterations: {}",
        config, max_iterations
    );

    match agent.run(messages, working_dir).await {
        Ok(agent_output) => {
            info!("Agent run finished successfully.");
            println!("{}", "--- Agent Run Summary ---".bold());

            if !agent_output.applied_tool_results.is_empty() {
                println!("{}:", "Tool Execution Results".cyan());
                for result in &agent_output.applied_tool_results {
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

            if let Some(final_desc) = &agent_output.final_state_description {
                println!("{}:", "Final AI Message".cyan());
                if let Err(e) = print_formatted(final_desc) {
                    error!(
                        "Failed to render final AI message markdown: {}. Printing raw.",
                        e
                    );
                    println!("{}", final_desc);
                } else {
                    println!();
                }
            } else {
                warn!("Agent finished but provided no final description in output.");
            }
            println!("-----------------------");
            Ok(agent_output)
        }
        Err(e) => {
            error!("Agent run failed: {:?}", e);
            Err(e)
        }
    }
}

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

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let _cli = Cli::parse(); // Parse CLI args but don't use cli.verbose anymore

    // --- Start Logging Initialization ---
    // Use EnvFilter to read RUST_LOG environment variable.
    // Default to "info" level if RUST_LOG is not set.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Build the subscriber
    // Note: Removed .with_max_level() as EnvFilter handles levels.
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(env_filter) // Use the EnvFilter
        .with_target(false) // Don't include module paths in logs
        .with_timer(tracing_subscriber::fmt::time::Uptime::default()) // Add uptime
        .finish();

    // Set the global default subscriber
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    // --- End Logging Initialization ---

    info!("Logging initialized. Default level: INFO, controllable via RUST_LOG.");

    let (config, project_root) =
        load_cli_config().context("Failed to load configuration and find project root")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("Failed to build HTTP client")?;

    let tool_provider: Arc<dyn ToolProvider> = Arc::new(CliToolProvider::new());

    const DEFAULT_MAX_ITERATIONS: usize = 20;
    let max_iterations =
        env::var("VOLITION_MAX_ITERATIONS")
            .ok()
            .and_then(|s| {
                s.parse::<usize>().map_err(|e| {
                warn!(
                    env_var = "VOLITION_MAX_ITERATIONS",
                    value = %s,
                    error = ?e,
                    "Failed to parse iteration limit from environment variable. Using default."
                );
                e
            }).ok()
            })
            .unwrap_or(DEFAULT_MAX_ITERATIONS);

    info!(
        limit = max_iterations,
        source = if env::var("VOLITION_MAX_ITERATIONS").is_ok()
            && env::var("VOLITION_MAX_ITERATIONS")
                .unwrap()
                .parse::<usize>()
                .is_ok()
        {
            "env(VOLITION_MAX_ITERATIONS)"
        } else if env::var("VOLITION_MAX_ITERATIONS").is_ok() {
            "env(parse_failed)->default"
        } else {
            "default"
        },
        "Agent iteration limit set."
    );

    let ui_handler = Arc::new(CliUserInteraction);

    let mut messages = match load_or_initialize_session(&config, &project_root)? {
        Some(msgs) => msgs,
        None => {
            error!("Failed to load or initialize session messages.");
            return Err(anyhow!("Session initialization failed"));
        }
    };

    if messages.is_empty() || messages[0].role != "system" {
        warn!("Messages list was empty or missing system prompt after init. Re-initializing.");
        messages = vec![ChatMessage {
            role: "system".to_string(),
            content: Some(config.system_prompt.clone()),
            ..Default::default()
        }];
    }

    print_welcome_message();

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

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(trimmed_input.to_string()),
            ..Default::default()
        });

        let recovery_path = project_root.join(RECOVERY_FILE_PATH);
        match serde_json::to_string_pretty(&messages) {
            Ok(state_json) => {
                if let Err(e) = fs::write(&recovery_path, state_json) {
                    warn!("Failed to write recovery state file: {}", e);
                } else {
                    info!("Saved conversation state to {:?}", recovery_path);
                }
            }
            Err(e) => {
                error!(
                    "Failed to serialize conversation state: {}. Cannot guarantee recovery.",
                    e
                );
            }
        }

        match run_agent_session(
            &config,
            &client,
            Arc::clone(&tool_provider),
            messages.clone(),
            &project_root,
            max_iterations,
            Arc::clone(&ui_handler),
        )
        .await
        {
            Ok(agent_output) => {
                if let Some(final_desc) = agent_output.final_state_description {
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: Some(final_desc),
                        ..Default::default()
                    });
                }

                if recovery_path.exists() {
                    if let Err(e) = fs::remove_file(&recovery_path) {
                        warn!(
                            "Failed to remove recovery state file after successful run: {}",
                            e
                        );
                    } else {
                        info!("Removed recovery state file after successful run.");
                    }
                }
            }
            Err(e) => {
                println!("{}: {:?}\n", "Agent run encountered an error".red(), e);
                // Pop the user message that caused the error from history
                messages.pop();
                info!("Removed last user message from history due to agent error.");
            }
        }
    }

    let _ = cleanup_session_state(&project_root);
    println!("{}", "Thanks!".cyan());
    Ok(())
}

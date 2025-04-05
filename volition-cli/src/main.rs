// volition-cli/src/main.rs
mod models;
mod rendering;
mod history; // Add history module

use anyhow::{anyhow, Context, Result};
use colored::*;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf}; // Ensure Path is imported
use std::process::ExitCode;
use std::sync::Arc;
use toml;
use uuid::Uuid; // Import Uuid

use volition_core::{
    agent::Agent,
    async_trait,
    config::AgentConfig,
    errors::AgentError,
    strategies::{
        complete_task::CompleteTaskStrategy,
        plan_execute::PlanExecuteStrategy,
    },
    // ChatMessage removed, UserInteraction kept
    UserInteraction,
};

use crate::models::cli::{Cli, Commands}; // Update import
use crate::rendering::print_formatted;
use crate::history::{
    save_history, load_history, list_histories, delete_history, get_history_preview, ConversationHistory
};

use clap::Parser;
use time::macros::format_description;
// trace removed
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::{
    fmt, fmt::time::LocalTime, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

const CONFIG_FILENAME: &str = "Volition.toml";
const LOG_FILE_NAME: &str = "volition-app.log";

type CliAgent = Agent<CliUserInteraction>;
type CliStrategy = Box<dyn volition_core::Strategy<CliUserInteraction> + Send + Sync>;

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

#[derive(Deserialize, Debug, Default)]
struct GitServerCliConfig {
    allowed_commands: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Default)]
struct CliTomlConfig {
    #[serde(default)]
    git_server: GitServerCliConfig,
}

fn load_git_server_allowed_commands(config_path: &Path) -> Option<Vec<String>> { // Use Path
    match fs::read_to_string(config_path) {
        Ok(toml_content) => match toml::from_str::<CliTomlConfig>(&toml_content) {
            Ok(cli_config) => cli_config.git_server.allowed_commands,
            Err(e) => {
                warn!(path = %config_path.display(), error = %e, "Failed to parse Volition.toml for git_server config. Using default.");
                None
            }
        },
        Err(e) => {
             if config_path.exists() {
                 warn!(path = %config_path.display(), error = %e, "Failed to read Volition.toml for git_server config. Using default.");
             }
            None
        }
    }
}

fn print_welcome_message(history_id: Option<Uuid>) {
    println!(
        "\n{}",
        "Volition - AI Assistant".cyan().bold()
    );
     if let Some(id) = history_id {
        println!("{}", format!("Resuming conversation: {}", id).cyan());
    }
    println!(
        "{}\
{}",
        "Type 'exit' or press Enter on an empty line to quit.".cyan(),
        "Type 'new' to start a fresh conversation.".cyan()
    );
    println!();
}

fn select_base_strategy(config: &AgentConfig) -> CliStrategy {
    // Keep existing logic, assuming it's correct
    let strategy_name = "complete_task"; // Hardcoded for now
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
                Box::new(CompleteTaskStrategy)
            }
        }
    } else {
        info!("Using CompleteTask strategy.");
        Box::new(CompleteTaskStrategy)
    }
}

/// Runs a single turn (non-interactive).
async fn run_single_turn(
    initial_prompt: String,
    mut history: ConversationHistory, // Takes ownership
    config: AgentConfig,
    project_root: PathBuf,
    ui_handler: Arc<CliUserInteraction>,
) -> Result<()> {
    info!(task = %initial_prompt, history_id = %history.id, "Running non-interactive turn.");

    let base_strategy = select_base_strategy(&config);

    // Agent::new expects Option<Vec<ChatMessage>>
    let initial_messages = Some(history.messages.clone()); // Clone messages for agent

    let mut agent = CliAgent::new(
        config.clone(),
        ui_handler,
        base_strategy,
        initial_messages,
        initial_prompt.clone(), // Pass the user's prompt
        None, // provider_registry_override
        None, // mcp_connections_override
    )
    .map_err(|e| AgentError::Config(format!("Failed to create agent instance: {}", e)))?;

    match agent.run(&project_root).await {
        Ok((final_message, updated_state)) => {
            info!("Agent session completed successfully.");
            println!("{}", final_message); // Print raw response for non-interactive

            history.messages = updated_state.messages; // Update with the full history from agent
            history.last_updated_at = chrono::Utc::now(); // Update timestamp
            save_history(&history)?;
            info!(history_id = %history.id, "Saved updated conversation history.");
            Ok(())
        }
        Err(e) => {
            error!("Agent run encountered an error: {}", e);
            // Optionally save history even on error? For now, we don't.
            Err(anyhow!(e))
        }
    }
}


/// Runs an interactive chat session.
async fn run_interactive(
    mut history: ConversationHistory, // Takes ownership
    config: AgentConfig,
    project_root: PathBuf,
    ui_handler: Arc<CliUserInteraction>,
) -> Result<()> {
    print_welcome_message(Some(history.id));

    loop {
        print!("\n{} ", ">".green().bold());
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        let trimmed_input = user_input.trim();

        if trimmed_input.is_empty() || trimmed_input.to_lowercase() == "exit" {
            break;
        }

        if trimmed_input.to_lowercase() == "new" {
            println!("{}", "Starting a new conversation...".cyan());
            save_history(&history).context("Failed to save history before starting new session")?;
            info!(history_id=%history.id, "Saved current history.");
            // Create a completely new history
            history = ConversationHistory::new(Vec::new());
            info!(history_id=%history.id, "Started new conversation history.");
            print_welcome_message(Some(history.id)); // Show new ID
            continue;
        }

        let user_message = trimmed_input.to_string();
        let agent_strategy = select_base_strategy(&config);

        // Agent::new expects Option<Vec<ChatMessage>>
        let current_messages = Some(history.messages.clone());

        let mut agent = CliAgent::new(
            config.clone(),
            Arc::clone(&ui_handler),
            agent_strategy,
            current_messages,
            user_message.clone(),
            None, // provider_registry_override
            None, // mcp_connections_override
        )
        .map_err(|e| AgentError::Config(format!("Failed to create agent instance: {}", e)))?;

        match agent.run(&project_root).await {
            Ok((final_message, updated_state)) => {
                info!("Agent turn completed successfully.");
                println!("\n{}", "--- Agent Response ---".bold());
                if let Err(e) = print_formatted(&final_message) {
                    error!("Failed to render final AI message markdown: {}. Printing raw.", e);
                    println!("{}", final_message);
                } else {
                    println!(); // Add newline after formatted output
                }
                println!("----------------------");

                // Update history with the state returned by the agent
                history.messages = updated_state.messages;
                history.last_updated_at = chrono::Utc::now(); // Update timestamp
                // Save after each successful turn
                if let Err(e) = save_history(&history) {
                    error!(history_id=%history.id, "Failed to save history: {}", e);
                    println!("{}", "Error: Failed to save conversation history.".red());
                } else {
                     info!(history_id=%history.id, "Saved updated conversation history.");
                }
            }
            Err(e) => {
                error!("Agent run encountered an error: {}", e);
                println!(
                    "{}: {}\n",
                    "Agent run encountered an error".red(),
                    e
                );
                 // Decide whether to save history on error. Let's save it to avoid losing context.
                 history.last_updated_at = chrono::Utc::now(); // Update timestamp even on error?
                 if let Err(save_err) = save_history(&history) {
                    error!(history_id=%history.id, "Failed to save history after error: {}", save_err);
                 }
            }
        }
    }
    // Save final state on exit
    save_history(&history).context("Failed to save history on exit")?;
     info!(history_id=%history.id, "Saved final conversation history on exit.");
    println!(
        "\n{}",
        "Conversation saved. Thanks!".cyan()
    );
    Ok(())
}

// --- New functions for list, view, delete ---

fn handle_list_conversations(limit: usize) -> Result<()> {
    let histories = list_histories()?;
    if histories.is_empty() {
        println!("No conversation histories found.");
        return Ok(());
    }

    println!("{}", "Recent Conversations:".bold());
    println!("{:<36} {:<25} {}", "ID".underline(), "Last Updated".underline(), "Preview".underline());
    for history in histories.iter().take(limit) {
         let local_time = history.last_updated_at.with_timezone(&chrono::Local);
         let preview = get_history_preview(history);
        println!(
            "{:<36} {:<25} {}",
            history.id.to_string(),
            local_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            preview.dimmed()
        );
    }
    Ok(())
}

fn handle_view_conversation(id: Uuid, full: bool) -> Result<()> {
    let history = load_history(id)?;
    let created_local = history.created_at.with_timezone(&chrono::Local);
    let updated_local = history.last_updated_at.with_timezone(&chrono::Local);

    println!("{}", format!("Conversation ID: {}", history.id).bold());
    println!("Created:         {}", created_local.format("%Y-%m-%d %H:%M:%S %Z"));
    println!("Last Updated:    {}", updated_local.format("%Y-%m-%d %H:%M:%S %Z"));
    println!("Messages:        {}", history.messages.len());
    println!("{}", "--- Messages ---".bold());

    for message in &history.messages {
        println!("\n[{}]", message.role.to_uppercase().cyan());
        // Safely get content as a string slice, default to empty string if None
        let content_str = message.content.as_deref().unwrap_or("");

        if full {
            // Print the full content (or empty string if it was None)
            println!("{}", content_str);
        } else {
            // Generate preview from the content_str
            let preview: String = content_str.lines().next().unwrap_or("").chars().take(100).collect();
            let line_count = content_str.lines().count();
            let char_count = content_str.chars().count();

            if line_count > 1 || char_count > 100 {
                 println!("{}...", preview.trim());
             } else {
                 println!("{}", preview.trim());
             }
        }
    }
     println!("\n{}", "--- End ---".bold());
     // Check if any message content was truncated in the non-full view
     let was_truncated = history.messages.iter().any(|m| {
         let c = m.content.as_deref().unwrap_or("");
         c.lines().count() > 1 || c.chars().count() > 100
     });
     if !full && was_truncated {
         println!("{}", "(Pass --full to see complete message content)".dimmed());
     }

    Ok(())
}

fn handle_delete_conversation(id: Uuid) -> Result<()> {
    // Ask for confirmation
    print!("{} Are you sure you want to delete conversation {}? (y/N) ", "Warning:".yellow().bold(), id);
    io::stdout().flush()?;
    let mut confirmation = String::new();
    io::stdin().read_line(&mut confirmation)?;

    if confirmation.trim().to_lowercase() == "y" {
        delete_history(id)?;
        println!("Conversation {} deleted.", id);
    } else {
        println!("Deletion cancelled.");
    }
    Ok(())
}


// --- Main Function Refactored ---

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // --- Logging Setup (identical to before) ---
    let default_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE, // Note: trace macro was removed, but level can still exist
    };
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::default().add_directive(default_level.into()));

    let log_dir = env::temp_dir();
    let log_path = log_dir.join(LOG_FILE_NAME);
    let file_appender = tracing_appender::rolling::never(log_dir, LOG_FILE_NAME);
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_writer(non_blocking_writer)
        .with_ansi(false)
        .with_target(true)
        .with_line_number(true);

    let time_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let local_timer = LocalTime::new(time_format);
    let stderr_layer = fmt::layer()
        .with_writer(io::stderr)
        .with_timer(local_timer)
        .with_target(false);

    if let Err(e) = tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .try_init()
    {
        eprintln!("Failed to set global tracing subscriber: {}", e);
        return ExitCode::FAILURE;
    }
    info!(
        "Logging initialized. Level determined by RUST_LOG or -v flags (default: {}). Logging to stderr and {}",
        default_level,
        log_path.display()
    );
    // --- End Logging Setup ---

    // --- Load Config (identical to before, but handle potential errors) ---
    let config_result = load_cli_config();
     let mut config;
     let project_root;

     match config_result {
         Ok((loaded_config, loaded_root)) => {
             config = loaded_config;
             project_root = loaded_root;

             // Modify config with git server args (identical to before)
            let config_toml_path = project_root.join(CONFIG_FILENAME);
            if let Some(allowed_commands) = load_git_server_allowed_commands(&config_toml_path) {
                if let Some(git_server_conf) = config.mcp_servers.get_mut("git") { 
                    if !allowed_commands.is_empty() {
                        info!(commands = ?allowed_commands, "Found git allowed_commands in config. Passing to server.");
                        let commands_str = allowed_commands.join(",");
                        git_server_conf.args.push("--allowed-commands".to_string());
                        git_server_conf.args.push(commands_str);
                        debug!(server_id = "git", args = ?git_server_conf.args, "Updated git server args");
                    } else {
                        info!("Empty git allowed_commands list found in config. Server will use its default.");
                    }
                } else {
                    warn!("git_server.allowed_commands found in TOML, but no MCP server with ID 'git' defined in config.");
                }
            } else {
                info!("No git_server.allowed_commands found in config. Server will use its default.");
            }
         }
         Err(e) => {
             // Allow list/view/delete even without config? Maybe not, they need history dir.
             // Let's require config for all operations for now.
             error!("Failed to load configuration: {}", e);
             eprintln!("Error: Could not find or load '{}'. Ensure you are in a project with this configuration file.", CONFIG_FILENAME.red());
             return ExitCode::FAILURE;
         }
     }
    // --- End Config Loading ---


    let ui_handler: Arc<CliUserInteraction> = Arc::new(CliUserInteraction);

    // --- Command Handling Logic ---
    let result = match cli.command {
        // --- list ---
        Some(Commands::List { limit }) => {
            handle_list_conversations(limit)
        }
        // --- view ---
        Some(Commands::View { id, full }) => {
             handle_view_conversation(id, full)
        }
        // --- delete ---
        Some(Commands::Delete { id }) => {
             handle_delete_conversation(id)
        }
        // --- resume ---
        Some(Commands::Resume { id, turn }) => {
            match load_history(id) {
                Ok(history) => {
                    if let Some(prompt) = turn {
                        // Resume + Single Turn (Non-interactive)
                         run_single_turn(prompt, history, config, project_root, ui_handler).await
                    } else {
                        // Resume Interactive
                         run_interactive(history, config, project_root, ui_handler).await
                    }
                }
                Err(e) => {
                    error!("Failed to load history {}: {}", id, e);
                    eprintln!("{}", format!("Error: Could not load conversation history for ID: {}", id).red());
                    Err(anyhow!("Failed to load history {}", id)) // Return error
                }
            }
        }
        // --- No Subcommand (Default behavior) ---
        None => {
             let initial_history = ConversationHistory::new(Vec::new()); // Start fresh
             info!(history_id=%initial_history.id, "Starting new conversation.");
            if let Some(prompt) = cli.turn {
                 // New Single Turn (Non-interactive)
                 run_single_turn(prompt, initial_history, config, project_root, ui_handler).await
            } else {
                 // New Interactive
                 run_interactive(initial_history, config, project_root, ui_handler).await
            }
        }
    };
    // --- End Command Handling ---

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            // Avoid printing duplicate errors if already handled by specific commands
            if !e.to_string().contains("Failed to load history") // Check for specific handled errors
               && !e.to_string().contains("Agent run encountered an error")
            {
                 eprintln!("Operation failed: {}", e);
            }
            ExitCode::FAILURE
        }
    }
}

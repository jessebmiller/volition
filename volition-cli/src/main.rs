// volition-cli/src/main.rs
mod models;
mod rendering;
mod history;

use anyhow::{anyhow, Context, Result};
use colored::*;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use toml;
use uuid::Uuid;
use chrono;

use rustyline::error::ReadlineError;
use rustyline::{Config, DefaultEditor};
use indicatif::{ProgressBar, ProgressStyle};
use dirs;
use dialoguer::{Confirm, theme::ColorfulTheme};

use volition_core::{
    agent::Agent,
    async_trait,
    config::AgentConfig,
    errors::AgentError,
    strategies::{
        complete_task::CompleteTaskStrategy,
        plan_execute::PlanExecuteStrategy,
    },
    UserInteraction,
};

// Use models::cli::Cli directly since Commands is unused now
use crate::models::cli::{Commands}; // Keep Commands import for matching
use crate::rendering::print_formatted;
use crate::history::{ // Keep ConversationHistory import
    save_history, load_history, list_histories, delete_history, get_history_preview, ConversationHistory
};

use clap::Parser;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::{
    fmt::{self, time::LocalTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};


const CONFIG_FILENAME: &str = "Volition.toml";
const LOG_FILE_NAME: &str = "volition-app.log";

type CliAgent = Agent<CliUserInteraction>;
type CliStrategy = Box<dyn volition_core::Strategy<CliUserInteraction> + Send + Sync>;

struct CliUserInteraction;

#[async_trait]
impl UserInteraction for CliUserInteraction {
    async fn ask(&self, prompt: String, _options: Vec<String>) -> Result<String> {
        // TODO: Consider using dialoguer::Input here for a nicer prompt

        // Current simple implementation:
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
    info!("Found configuration file at: {:?}", config_path);
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
                 // Use warn! now that logging is likely initialized
                 warn!(path = %config_path.display(), error = %e, "Failed to parse TOML for git_server config. Using default.");
                None
            }
        },
        Err(e) => {
             if config_path.exists() {
                  warn!(path = %config_path.display(), error = %e, "Failed to read TOML for git_server config. Using default.");
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
        println!("{}: {}", "Current conversation".cyan(), id.to_string().dimmed());
    }
    println!(
        "{}\n{}",
        "Type 'exit', 'quit', Ctrl-D, or press Enter on an empty line to quit.".dimmed(),
        "Type 'new' to start a fresh conversation.".dimmed()
    );
    println!(); // Add newline for spacing
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
                warn!("\'plan_execute\' strategy selected but config is missing or incomplete. Falling back to \'CompleteTask\'.");
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
    project_root: PathBuf, // Keep PathBuf ownership
    ui_handler: Arc<CliUserInteraction>,
) -> Result<()> {
    info!(task = %initial_prompt, history_id = %history.id, "Running non-interactive turn.");

    let base_strategy = select_base_strategy(&config);
    let initial_messages = Some(history.messages.clone());

    // --- Add Spinner ---
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")?
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "-"]),
    );
    pb.set_message("Thinking...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    // --- End Spinner ---

    // Scope agent creation and run
    let agent_result = {
        let mut agent = CliAgent::new(
            config.clone(),
            ui_handler,
            base_strategy,
            initial_messages,
            initial_prompt.clone(),
            None, // provider_registry_override
            None, // mcp_connections_override
        )
        .map_err(|e| AgentError::Config(format!("Failed to create agent instance: {}", e)))?;
         agent.run(&project_root).await // Pass project_root reference here
     };

    pb.finish_and_clear(); // Stop spinner

    match agent_result {
        Ok((final_message, updated_state)) => {
            info!("Agent session completed successfully.");
            println!("{}", final_message); // Print raw response for non-interactive

            history.messages = updated_state.messages;
            history.last_updated_at = chrono::Utc::now();
            save_history(&project_root, &history)?; // Pass project_root reference
            info!(history_id = %history.id, "Saved updated conversation history.");
            Ok(())
        }
        Err(e) => {
            error!("Agent run encountered an error: {}", e);
            // Don't save history on error in non-interactive mode
            Err(anyhow!(e))
        }
    }
}


// --- run_interactive with rustyline ---
/// Runs an interactive chat session using rustyline for a REPL experience.
async fn run_interactive(
    mut history: ConversationHistory, // Takes ownership
    config: AgentConfig,
    project_root: PathBuf, // Keep PathBuf ownership
    ui_handler: Arc<CliUserInteraction>,
) -> Result<()> {
    print_welcome_message(Some(history.id));

    // --- Rustyline Setup ---
    let rl_config = Config::builder()
        .history_ignore_space(true)
        .completion_type(rustyline::CompletionType::List)
        .edit_mode(rustyline::EditMode::Emacs)
        .auto_add_history(true)
        .build();

    let mut rl = DefaultEditor::with_config(rl_config)?;

    // --- CLI History File Setup ---
    let history_dir = dirs::cache_dir()
         .map(|d| d.join("volition"))
         .ok_or_else(|| anyhow!("Could not determine cache directory for history file"))?;
    fs::create_dir_all(&history_dir).context("Failed to create CLI history directory")?;
    let history_file_path = history_dir.join("cli_history.txt");
    if rl.load_history(&history_file_path).is_err() {
        debug!(path = %history_file_path.display(), "No previous CLI history found or error loading.");
    }
    // --- End Rustyline Setup ---

    let prompt = format!("{} ", ">".green().bold());

    loop {
        let readline_result = rl.readline(&prompt);

        match readline_result {
            Ok(line) => {
                let trimmed_input = line.trim();

                // Handle exit conditions
                if trimmed_input.is_empty() || trimmed_input.to_lowercase() == "exit" || trimmed_input.to_lowercase() == "quit" {
                    info!("Exit command or empty line entered, exiting interactive mode.");
                    break;
                }

                // Handle 'new' command
                if trimmed_input.to_lowercase() == "new" {
                    println!("\n{}", "Starting a new conversation...".cyan());
                    // Save the *current* conversation before starting new
                    if let Err(e) = save_history(&project_root, &history) { // Pass project_root
                         error!(history_id=%history.id, "Failed to save history before starting new session: {}", e);
                         eprintln!("{}", "Error: Failed to save previous conversation history.".red());
                         // Continue anyway
                    } else {
                        info!(history_id=%history.id, "Saved current history before starting new.");
                    }

                    history = ConversationHistory::new(Vec::new());
                    info!(history_id=%history.id, "Started new conversation history.");
                    print_welcome_message(Some(history.id)); // Show new ID
                    continue; // Go to next loop iteration for new input
                }

                // --- Agent Execution Logic ---
                let user_message = trimmed_input.to_string();
                let agent_strategy = select_base_strategy(&config);
                let current_messages = Some(history.messages.clone());

                // --- Add Spinner ---
                let pb = ProgressBar::new_spinner();
                 pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.cyan} {msg}")?
                        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "-"]),
                );
                pb.set_message("Thinking...");
                pb.enable_steady_tick(std::time::Duration::from_millis(100));
                // --- End Spinner ---

                let agent_result = { // Scope agent
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
                    agent.run(&project_root).await // Pass project_root
                };

                 pb.finish_and_clear(); // Stop spinner

                match agent_result {
                    Ok((final_message, updated_state)) => {
                        info!("Agent turn completed successfully.");
                        println!("\n{}\n", "--- Agent Response ---".bold());
                        if let Err(e) = print_formatted(&final_message) {
                            error!("Failed to render final AI message markdown: {}. Printing raw.", e);
                            println!("{}", final_message);
                        }
                        println!("\n----------------------");

                        history.messages = updated_state.messages;
                        history.last_updated_at = chrono::Utc::now();
                        if let Err(e) = save_history(&project_root, &history) { // Pass project_root
                            error!(history_id=%history.id, "Failed to save conversation history: {}", e);
                            eprintln!("{}", "Error: Failed to save conversation history.".red());
                        } else {
                            info!(history_id=%history.id, "Saved updated conversation history.");
                        }
                    }
                    Err(e) => {
                        error!("Agent run encountered an error: {}", e);
                        eprintln!(
                            "\n{}: {}", // Add newline before error
                            "Agent run encountered an error".red(),
                            e
                        );
                        // Save history even on error
                        history.last_updated_at = chrono::Utc::now();
                        if let Err(save_err) = save_history(&project_root, &history) { // Pass project_root
                            error!(history_id=%history.id, "Failed to save conversation history after error: {}", save_err);
                        }
                    }
                }
                // --- End Agent Execution ---
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C".yellow());
                continue;
            }
            Err(ReadlineError::Eof) => {
                info!("EOF detected, exiting interactive mode.");
                break;
            }
            Err(err) => {
                error!("Readline error: {:?}", err);
                eprintln!("Error reading input: {}", err.to_string().red());
                break;
            }
        }
    }

    // --- Save Rustyline History ---
    if let Err(e) = rl.save_history(&history_file_path) {
         warn!(path = %history_file_path.display(), error = %e, "Failed to save CLI history.");
     } else {
         debug!(path = %history_file_path.display(), "Saved CLI history.");
     }
    // --- End Save Rustyline History ---

    // Save final conversation state on exit
     if let Err(e) = save_history(&project_root, &history) { // Pass project_root
         error!(history_id=%history.id, "Failed to save final conversation history on exit: {}", e);
         eprintln!("{}", "Error: Failed to save final conversation history.".red());
     } else {
        info!(history_id=%history.id, "Saved final conversation history on exit.");
     }

    println!(
        "\n{}\n", // Add surrounding newlines
        "Conversation saved. Exiting.".cyan()
    );
    Ok(())
}
// --- End run_interactive ---

// --- Updated functions for list, view, delete ---

fn handle_list_conversations(project_root: &Path, limit: usize) -> Result<()> { // Accept project_root
    let histories = list_histories(project_root)?; // Pass project_root
    if histories.is_empty() {
        println!("No conversation histories found in this project.");
        return Ok(());
    }

    println!("{}: {}", "Project".bold(), project_root.display());
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
    println!("\n{}", "(Use 'volition view <ID>' to see details)".dimmed());
    Ok(())
}

fn handle_view_conversation(project_root: &Path, id: Uuid, full: bool) -> Result<()> { // Accept project_root
    let history = load_history(project_root, id)?; // Pass project_root
    let created_local = history.created_at.with_timezone(&chrono::Local);
    let updated_local = history.last_updated_at.with_timezone(&chrono::Local);

    println!("{}: {}", "Project".bold(), project_root.display());
    println!("{}: {}", "Conversation ID".bold(), history.id);
    println!("Created:         {}", created_local.format("%Y-%m-%d %H:%M:%S %Z"));
    println!("Last Updated:    {}", updated_local.format("%Y-%m-%d %H:%M:%S %Z"));
    println!("Messages:        {}", history.messages.len());
    println!("{}", "--- Messages ---".bold());

    for message in &history.messages {
        println!("\n[{}]", message.role.to_uppercase().cyan());
        // Safely get content as a string slice, default to empty string if None
        let content_str = message.content.as_deref().unwrap_or("");

        if full {
             if let Err(e) = print_formatted(content_str) { // Try formatting even in full view
                 error!("Failed to render message markdown in full view: {}. Printing raw.", e);
                 println!("{}", content_str);
             }
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
     let was_truncated = !full && history.messages.iter().any(|m| {
         let c = m.content.as_deref().unwrap_or("");
         c.lines().count() > 1 || c.chars().count() > 100
     });
     if was_truncated {
         println!("{}", "(Pass --full to see complete message content)".dimmed());
     }

    Ok(())
}

// --- handle_delete_conversation UPDATED with dialoguer and project_root ---
fn handle_delete_conversation(project_root: &Path, id: Uuid) -> Result<()> { // Accept project_root
    // Use dialoguer for confirmation
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Are you sure you want to delete conversation {} from project {}?", id, project_root.display()))
        .default(false)
        .interact()? // Show the prompt
    {
        delete_history(project_root, id)?; // Pass project_root
        println!("Conversation {} deleted.", id);
    } else {
        println!("Deletion cancelled.");
    }
    Ok(())
}
// --- End handle_delete_conversation ---


// --- Main Function ---

#[tokio::main]
async fn main() -> ExitCode {
    colored::control::set_override(true);
    dotenvy::dotenv().ok();
    let cli = models::cli::Cli::parse();

    // --- Logging Setup ---
    let default_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::default().add_directive(default_level.into()));

    let log_dir = match dirs::cache_dir().or_else(dirs::runtime_dir).or_else(|| Some(env::temp_dir())).map(|d| d.join("volition")) {
        Some(dir) => dir,
        None => {
            eprintln!("{}", "Error: Could not determine a suitable directory for log files.".red());
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = fs::create_dir_all(&log_dir) {
         eprintln!("{} Failed to create log directory {}: {}", "Error:".red(), log_dir.display(), e);
         return ExitCode::FAILURE;
    }
    let log_path = log_dir.join(LOG_FILE_NAME);
    let file_appender = tracing_appender::rolling::never(&log_dir, LOG_FILE_NAME);
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_writer(non_blocking_writer)
        .with_ansi(false)
        .with_target(true)
        .with_line_number(true);
    let time_format_desc = match time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
    ) {
        Ok(desc) => desc,
        Err(e) => {
            eprintln!("Warning: Failed to parse time format, using default: {}", e);
            time::format_description::parse("[hour]:[minute]:[second]").expect("Fallback time format failed")
        }
    };
    let local_timer = LocalTime::new(time_format_desc);
    let stderr_layer = fmt::layer()
        .with_writer(io::stderr)
        .with_timer(local_timer.clone())
        .with_target(false)
        .with_level(true);
    let file_layer = file_layer.with_timer(local_timer);
    if let Err(e) = tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .try_init()
    {
        eprintln!("{} Failed to initialize logging: {}", "Error:".red(), e);
        return ExitCode::FAILURE;
    }
    colored::control::unset_override();
    info!(
        "Logging initialized. Level determined by RUST_LOG or -v flags (default: {}). Logging to stderr and {}",
        default_level,
        log_path.display()
    );
    // --- End Logging Setup ---

    // --- Load Config ---
    let config_result = load_cli_config();
     let mut config;
     let project_root; // Keep ownership here

     match config_result {
         Ok((loaded_config, loaded_root)) => {
             config = loaded_config;
             project_root = loaded_root;

             // Modify config with git server args
            let config_toml_path = project_root.join(CONFIG_FILENAME);
             // Use warn! now that logging is initialized
             if let Some(allowed_commands) = load_git_server_allowed_commands(&config_toml_path) {
                if let Some(git_server_conf) = config.mcp_servers.get_mut("git") { 
                    if !allowed_commands.is_empty() {
                        info!(commands = ?allowed_commands, "Found git allowed_commands in config. Passing to server.");
                        let commands_str = allowed_commands.join(",");
                        if !git_server_conf.args.contains(&"--allowed-commands".to_string()) {
                            git_server_conf.args.push("--allowed-commands".to_string());
                            git_server_conf.args.push(commands_str);
                            debug!(server_id = "git", args = ?git_server_conf.args, "Updated git server args");
                        }
                    } else {
                        info!("Empty git allowed_commands list found in config. Server will use its default.");
                    }
                } else {
                     if !allowed_commands.is_empty() {
                        warn!("git_server.allowed_commands found in TOML, but no MCP server with ID 'git' defined in config.");
                     }
                }
            } else {
                info!("No git_server.allowed_commands found in config. Server will use its default.");
            }
         }
         Err(e) => {
             // Config loading failed *after* logging was initialized
             error!("Failed to load configuration: {}", e); // Log the detailed error
             eprintln!("{} Could not find or load '{}'. Ensure you are in a Volition project directory.", "Error:".red(), CONFIG_FILENAME); // User-friendly error
             return ExitCode::FAILURE;
         }
     }
    // --- End Config Loading ---

    let ui_handler: Arc<CliUserInteraction> = Arc::new(CliUserInteraction);

    // --- Command Handling Logic ---
    let result = match cli.command {
        // --- list ---
        Some(Commands::List { limit }) => {
            handle_list_conversations(&project_root, limit) // Pass reference
        }
        // --- view ---
        Some(Commands::View { id, full }) => {
             handle_view_conversation(&project_root, id, full) // Pass reference
        }
        // --- delete ---
        Some(Commands::Delete { id }) => {
             handle_delete_conversation(&project_root, id) // Pass reference (now uses dialoguer internally)
        }
        // --- resume ---
        Some(Commands::Resume { id, turn }) => {
            match load_history(&project_root, id) { // Pass reference
                Ok(history) => {
                    if let Some(prompt) = turn {
                        // Resume + Single Turn (Non-interactive)
                         run_single_turn(prompt, history, config, project_root, ui_handler).await // Pass ownership
                    } else {
                        // Resume Interactive (with rustyline)
                         run_interactive(history, config, project_root, ui_handler).await // Pass ownership
                    }
                }
                Err(e) => {
                    error!("Failed to load history {}: {}", id, e); // Log detailed error
                    eprintln!("{} Could not load conversation history for ID: {}", "Error:".red(), id); // User-friendly error
                    Err(anyhow!("Failed to load history {}", id)) // Return error for main handler
                }
            }
        }
        // --- No Subcommand (Default behavior) ---
        None => {
             let initial_history = ConversationHistory::new(Vec::new()); // Start fresh
             info!(history_id=%initial_history.id, "Starting new conversation.");
            if let Some(prompt) = cli.turn {
                 // New Single Turn (Non-interactive)
                 run_single_turn(prompt, initial_history, config, project_root, ui_handler).await // Pass ownership
            } else {
                 // New Interactive (with rustyline)
                 run_interactive(initial_history, config, project_root, ui_handler).await // Pass ownership
            }
        }
    };
    // --- End Command Handling ---

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            // Use improved error checking from HEAD
            let error_string = e.to_string();
             let is_dialoguer_error = matches!(e.downcast_ref::<dialoguer::Error>(), Some(_));
            let already_handled = error_string.contains("Could not load conversation history")
               || error_string.contains("Agent run encountered an error")
               || error_string.contains("Failed to load history") // Includes "History file not found"
               || error_string.contains("Error reading input") // From rustyline
               || is_dialoguer_error;

            if !already_handled {
                 error!("Operation failed: {}", e);
                 eprintln!("{} Operation failed: {}", "Error:".red(), e);
            }
            ExitCode::FAILURE
        }
    }
}

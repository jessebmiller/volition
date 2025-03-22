mod api;
mod config;
mod models;
mod tools;
mod utils;
mod strategies; // New module for strategies

use anyhow::{anyhow, Result};
use colored::*;
use std::io::{self, Write};
use tokio::time::Duration;

use crate::config::{load_config, configure};
use crate::models::chat::ResponseMessage;
use crate::models::cli::{Commands, Cli};
use crate::utils::DebugLevel;
use crate::strategies::linear::linear_strategy; // Import linear strategy

use clap::Parser;

const SYSTEM_PROMPT: &str = r#"
You are Volition, an AI-powered software engineering assistant specializing in code analysis, refactoring, and product engineering.
Your goal is to help developers understand, modify, and improve products through expert analysis, precise code edits, and feature implementation.
Your goal for any edit is to do a full and complete job. You have met your goal when the changes are done and the code is shippable.

You have access to powerful tools:
1. shell - Execute shell commands
2. read_file - Read file contents
3. write_file - Write/edit files
4. search_code - Search for patterns in code
5. find_definition - Locate symbol definitions
6. user_input - Ask users for decisions

When a user asks you to help with a codebase:
1. Gather information about the codebase structure and key files
2. Analyze code for patterns, architecture, and potential issues
3. Make a plan for implementing requested changes
4. Execute the plan using your tools
5. Provide clear explanations about what you're doing
6. Ask for user confirmation via user_input before making significant changes
7. Always look for the answer to any questions you may have using your tools before asking the user

Best practices to follow:
- Use search_code to find relevant code sections
- Use find_definition to locate where symbols are defined
- Always read files before suggesting edits
- Create git commits we can roll back to before modifying important files
- Verify changes with targeted tests when possible
- Explain complex code sections in simple accurate terms
- Specifically ask for user confirmation before:
  * Making structural changes to the codebase
  * Modifying core functionality
  * Introducing new dependencies

Provide concise explanations of your reasoning and detailed comments for any code you modify or create.
"#;

async fn handle_conversation(config: &config::Config, query: &str, debug_level: DebugLevel) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    // Print welcome message
    println!("\n{}", "\x1b[1;36m");
    println!("\n{}", "[1;36m Volition - AI Software Engineering Assistant".cyan().bold());
    println!("{}", "Ready to help you understand and improve your codebase.".cyan());
    println!("{}", "Type 'exit' or press Enter on an empty line to quit at any time.".cyan());
    println!("");

    let mut messages: Vec<ResponseMessage> = vec![
        ResponseMessage {
            role: "system".to_string(),
            content: Some(SYSTEM_PROMPT.to_string()),
            tool_calls: None,
            tool_call_id: None,
        },
        ResponseMessage {
            role: "user".to_string(),
            content: Some(query.to_string()),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    loop {
        messages = linear_strategy(
            &client,
            config,
            vec!["shell".to_string(), "read_file".to_string(), "write_file".to_string(), "search_code".to_string(), "find_definition".to_string(), "user_input".to_string()],
            query,
            SYSTEM_PROMPT,
            debug_level,
            messages,
        ).await?;

        // Ask for follow-up input from user
        println!("\n{}", "Enter a follow-up question or press Enter to exit:".cyan().bold());
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        // Exit if user enters empty string or "exit"
        if input.is_empty() || input.to_lowercase() == "exit" {
            println!("\n{}", "Goodbye! Thank you for using Volition.".cyan());
            break;
        } else {
            // Add user's follow-up input to messages
            messages.push(ResponseMessage {
                role: "user".to_string(),
                content: Some(input.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine debug level from command line flags
    let debug_level = if cli.verbose {
        DebugLevel::Verbose
    } else if cli.debug {
        DebugLevel::Minimal
    } else {
        DebugLevel::None
    };

    match &cli.command {
        Some(Commands::Configure) => configure()?,
        Some(Commands::Run { args, verbose, debug }) => {
            // Override debug level from subcommand flags if specified
            let debug_level = if *verbose {
                DebugLevel::Verbose
            } else if *debug {
                DebugLevel::Minimal
            } else {
                debug_level
            };

            let query = args.join(" ");
            if query.is_empty() {
                return Err(anyhow!("Please provide a command to run"));
            }

            let config = load_config()?;
            handle_conversation(&config, &query, debug_level).await?;
        }
        None => {
            if cli.rest.is_empty() {
                println!("Welcome to Volition - AI Software Engineering Assistant");
                println!("Usage: volition <command> [arguments]");
                println!("Examples:");
                println!("  volition \"Analyze the src directory and list the main components\"");
                println!("  volition \"Find all usages of the login function and refactor it to use async/await\"");
                println!("  volition \"Help me understand how the routing system works in this codebase\"");
                println!("  volition configure    - Set up your API key");
                println!("  volition --help       - Show more information");
                return Ok(());
            }

            let query = cli.rest.join(" ");
            let config = load_config()?;
            handle_conversation(&config, &query, debug_level).await?;
        }
    }

    Ok(())
}

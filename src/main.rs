mod api;
mod config;
mod models;
mod tools;

use anyhow::{anyhow, Result};
use colored::*;
use std::io::{self, Write};
use tokio::time::Duration;

use crate::api::chat_with_api;
use crate::config::load_config;
use crate::models::chat::ResponseMessage;
use crate::models::cli::{Commands, Cli};
use crate::tools::handle_tool_calls;

use clap::Parser;
use tracing::{Level};
use tracing_subscriber::FmtSubscriber;

const SYSTEM_PROMPT: &str = r#"
You are Volition, an AI-powered software engineering assistant specializing in code analysis, refactoring, and product engineering.
Your goal is to help developers understand, modify, and improve products through expert analysis, precise code edits, and feature implementation.

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
7. Always try to answer questions yourslef before asking the user

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

async fn handle_conversation(config: &config::Config, query: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    // Print welcome message
    println!("\n{}", "Volition - AI Software Engineering Assistant".cyan().bold());
    println!("{}", "Ready to help you understand and improve your codebase.".cyan());
    println!("{}", "Type 'exit' or press Enter on an empty line to quit".cyan());
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

    let mut conversation_active = true;

    while conversation_active {
        tracing::debug!("Current message history: {:?}", messages);

        let response = chat_with_api(&client, config, messages.clone(), None).await?;

        let message = &response.choices[0].message;

        // Print content if there is any
        if let Some(content) = &message.content {
            if !content.is_empty() {
                println!("\n{}", content);
            }
        }

        // Store the original response message as received
        messages.push(ResponseMessage {
            role: "assistant".to_string(),
            content: message.content.clone(),
            tool_calls: message.tool_calls.clone(),
            tool_call_id: None,
        });

        // Process tool calls if any
        if let Some(tool_calls) = &message.tool_calls {
            tracing::info!("Processing {} tool calls", tool_calls.len());

            handle_tool_calls(&client, &config.openai.api_key, tool_calls.to_vec(), &mut messages).await?;
        } else {
            // No tool calls - get follow-up input from user
            println!("\n{}", "Enter a follow-up question or press Enter to exit:".cyan().bold());
            print!("{} ", ">".green().bold());
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_string();
            
            // Exit if user enters empty string or "exit"
            if input.is_empty() || input.to_lowercase() == "exit" {
                println!("\n{}", "Goodbye! Thank you for using Volition.".cyan());
                conversation_active = false;
            } else {
                // Add user's follow-up input to messages
                messages.push(ResponseMessage {
                    role: "user".to_string(),
                    content: Some(input),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let level = if cli.verbose {
        Level::DEBUG
    } else if cli.debug {
        Level::INFO
    } else {
        Level::WARN
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    match &cli.command {
        Some(Commands::Configure) => println!("Configure command is not implemented."),
        Some(Commands::Run { args, verbose: _, debug: _ }) => {
            let query = args.join(" ");
            if query.is_empty() {
                return Err(anyhow!("Please provide a command to run"));
            }
            let config = load_config()?;
            handle_conversation(&config, &query).await?;
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
            handle_conversation(&config, &query).await?;
        }
    }

    Ok(())
}

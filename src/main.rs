mod api;
mod config;
mod models;
mod tools;
mod utils;
mod strategies;
mod constants;

use anyhow::{anyhow, Result};
use colored::*;
use std::io::{self, Write};
use tokio::time::Duration;

use crate::config::{load_config, configure};
use crate::models::chat::ResponseMessage;
use crate::models::cli::{Commands, Cli};
use crate::models::tools::Tools;
use crate::strategies::linear::linear_strategy;

use clap::Parser;
use log::LevelFilter;

use crate::constants::SYSTEM_PROMPT;

async fn handle_conversation(config: &config::Config, query: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    println!("\n{}", "\x1b[1;36m");
    println!("\n{}", "Volition - AI Software Engineering Assistant".cyan().bold());
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
            vec![
                Tools::shell_definition(),
                Tools::read_file_definition(),
                Tools::write_file_definition(),
                Tools::search_code_definition(),
                Tools::find_definition_definition(),
                Tools::user_input_definition(),
            ],
            query,
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
            println!("\n{}", "o/ Thanks.".cyan());
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
    let mut builder = env_logger::Builder::from_default_env();
    if std::env::var("RUST_LOG").is_err() {
        builder.filter(None, LevelFilter::Info);
    }
    builder.init();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Configure) => configure()?,
        Some(Commands::Run { args, verbose, debug }) => {
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

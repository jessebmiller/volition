use std::io::{self, Write};
use anyhow::Result;
use colored::*;
use crate::models::tools::UserInputArgs;

pub fn get_user_input(args: UserInputArgs) -> Result<String> {
    // Display the prompt to the user
    println!("\n{}", args.prompt.cyan().bold());

    // Display options if provided
    if let Some(options) = args.options {
        for (idx, option) in options.iter().enumerate() {
            println!("  {}. {}", idx + 1, option);
        }
        println!();
    }

    // Get user input
    print!("{} ", ">".green().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_string();

    Ok(input)
}

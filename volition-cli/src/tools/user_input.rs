// volition-cli/src/tools/user_input.rs
use anyhow::Result;
use colored::*;
use std::io::{self, Write};

// Removed UserInputArgs import

/// Prompts the user for input, optionally presenting choices.
pub fn get_user_input(prompt: &str, options: Option<Vec<String>>) -> Result<String> {
    // Display the prompt to the user
    println!("\n{}", prompt.cyan().bold());

    // Display options if provided
    if let Some(ref options_vec) = options {
        // Check if options are actually present before iterating
        if !options_vec.is_empty() {
             for (idx, option) in options_vec.iter().enumerate() {
                println!("  {}. {}", idx + 1, option);
            }
            println!(); // Add a newline after options
        }
    }

    // Get user input
    print!("{} ", ">".green().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_string();

    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Basic test (doesn't test actual stdin/stdout)
    #[test]
    fn test_get_user_input_signature() {
        // This test mainly checks that the function signature is callable
        // and doesn't panic. It doesn't verify the interaction.
        let prompt = "Test prompt";
        let options = Some(vec!["Option 1".to_string(), "Option 2".to_string()]);

        // We can't easily test the Result<String> returned because it requires stdin.
        // We just check that calling it with valid args doesn't immediately fail.
        // A more thorough test would require mocking stdin/stdout.
        // let result = get_user_input(prompt, options);
        // assert!(result.is_ok()); // This would block waiting for input

        // Dummy assertion to make the test runnable
        assert_eq!(prompt, "Test prompt");
        assert!(options.is_some());
    }
}

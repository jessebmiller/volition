// volition-cli/src/tools/user_input.rs
use anyhow::Result;
use colored::*;
use std::io::{self, Write};

pub fn get_user_input(prompt: &str, options: Option<Vec<String>>) -> Result<String> {
    println!("\n{}", prompt.cyan().bold());

    if let Some(ref options_vec) = options {
        if !options_vec.is_empty() {
            for (idx, option) in options_vec.iter().enumerate() {
                println!("  {}. {}", idx + 1, option);
            }
            println!();
        }
    }

    print!("{} ", ">".green().bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_string();

    Ok(input)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_get_user_input_signature() {
        let prompt = "Test prompt";
        let options = Some(vec!["Option 1".to_string(), "Option 2".to_string()]);
        assert_eq!(prompt, "Test prompt");
        assert!(options.is_some());
    }
}

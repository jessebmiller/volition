use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub openai_api_key: String,
    pub service: String, // "openai" or "ollama"
    pub model_name: String, // Model name for the chosen service
}

pub fn get_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home_dir.join(".config").join("volition").join("config.json"))
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Err(anyhow!("Configuration not found. Run 'volition configure' first."));
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

    let config: Config = serde_json::from_str(&config_str)
        .with_context(|| "Failed to parse config file")?;

    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;

    // Ensure the directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let config_str = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, config_str)?;

    Ok(())
}

pub fn configure() -> Result<()> {
    let mut api_key = String::new();
    let mut service = String::new();
    let mut model_name = String::new();

    print!("Enter your OpenAI API key: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut api_key)?;
    let api_key = api_key.trim().to_string();

    print!("Choose service (openai/ollama): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut service)?;
    let service = service.trim().to_string();

    print!("Enter model name: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut model_name)?;
    let model_name = model_name.trim().to_string();

    let config = Config {
        openai_api_key: api_key,
        service,
        model_name,
    };

    save_config(&config)?;
    println!("Configuration saved successfully!");

    Ok(())
}

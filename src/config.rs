use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use toml;

#[derive(Deserialize, Debug, Clone)]
pub struct ActiveService {
    pub service: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub active_service: ActiveService,
    pub openai: OpenAIConfig,
    // Removed gemini field
    pub models: HashMap<String, ModelConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub selected_model: String,
}

// Removed GeminiConfig struct

#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub service: String, // e.g., "openai", "ollama", etc.
    pub parameters: toml::Value,
    #[serde(default)]
    pub endpoint_override: Option<String>,
}

pub fn get_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home_dir.join(".config").join("volition").join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Err(anyhow!("Configuration not found. Please set up your configuration."));
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

    let config: Config = toml::from_str(&config_str)
        .with_context(|| "Failed to parse config file")?;

    Ok(config)
}

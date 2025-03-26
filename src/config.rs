use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use toml;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub openai: OpenAIConfig,
    pub models: HashMap<String, ModelConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub selected_model: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub service: String,
    pub parameters: toml::Value,
}

pub fn get_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home_dir.join(".config").join("volition").join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Err(anyhow!("Configuration not found. Run 'volition configure' first."));
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

    let config: Config = toml::from_str(&config_str)
        .with_context(|| "Failed to parse config file")?;

    Ok(config)
}

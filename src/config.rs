use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::env; // Import env module
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use toml;

#[derive(Deserialize, Debug, Clone)]
pub struct ActiveService {
    pub service: String,
}

// Add #[serde(skip)] to api_key, it will be loaded from env var
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub active_service: ActiveService,
    pub openai: OpenAIConfig,
    // Removed gemini field
    pub models: HashMap<String, ModelConfig>,
    #[serde(skip)] // This field is not loaded from the TOML file
    pub api_key: String,
}

// Removed api_key field, it will be loaded from env var into Config struct
#[derive(Deserialize, Debug, Clone)]
pub struct OpenAIConfig {
    // pub api_key: String, // Removed: Load from environment variable API_KEY
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
    // Keep using ~/.config for Linux as requested
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home_dir.join(".config").join("volition").join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    // --- Load API Key from Environment Variable ---
    let api_key = env::var("API_KEY")
        .context("Failed to read API_KEY environment variable. Please ensure it is set.")?;
    if api_key.is_empty() {
        return Err(anyhow!("API_KEY environment variable is set but empty."));
    }

    // --- Load Configuration File ---
    let config_path = get_config_path()?;
    if !config_path.exists() {
        // Provide more guidance on config file creation
        return Err(anyhow!(
            "Configuration file not found at {:?}. Please create it and set required values.",
            config_path
        ));
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

    let mut config: Config = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

    // --- Populate API Key ---
    config.api_key = api_key; // Assign the key read from the environment

    // --- Validation ---
    let active_service_name = &config.active_service.service;
    // Assuming the config section name (e.g., `openai`) matches the active service name for now.
    // TODO: Make this more dynamic if multiple service configs (like openai, gemini, anthropic) exist.
    let selected_model_name = match active_service_name.as_str() {
        "openai" => &config.openai.selected_model,
        // Add other services here if needed in the future
        _ => return Err(anyhow!("Active service '{}' specified in config is not currently supported or has no corresponding configuration section.", active_service_name)),
    };


    // Check if the selected model exists in the models map
    let model_config = config.models.get(selected_model_name)
        .ok_or_else(|| anyhow!(
            "Selected model '{}' not found in the [models] section of the config file.",
            selected_model_name
        ))?;

    // Check if the selected model belongs to the active service
    if model_config.service != *active_service_name {
        return Err(anyhow!(
            "Selected model '{}' is configured for service '{}', but the active service is '{}'.",
            selected_model_name, model_config.service, active_service_name
        ));
    }

    // --- Validation Passed ---
    tracing::info!("Successfully loaded and validated configuration from {:?}", config_path);
    Ok(config)
}

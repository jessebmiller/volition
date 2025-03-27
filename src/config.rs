use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf}; // Added Path
use std::env;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use toml;

// --- Existing Config Structs (unchanged) ---

#[derive(Deserialize, Debug, Clone)]
pub struct ActiveService {
    pub service: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub active_service: ActiveService,
    pub openai: OpenAIConfig,
    pub models: HashMap<String, ModelConfig>,
    #[serde(skip)]
    pub api_key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OpenAIConfig {
    pub selected_model: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub service: String,
    pub parameters: toml::Value,
    #[serde(default)]
    pub endpoint_override: Option<String>,
}

// --- Existing Loading Logic (unchanged) ---

pub fn get_config_path() -> Result<PathBuf> {
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
    config.api_key = api_key;

    // --- Validation ---
    let active_service_name = &config.active_service.service;
    let selected_model_name = match active_service_name.as_str() {
        "openai" => &config.openai.selected_model,
        _ => return Err(anyhow!("Active service '{}' specified in config is not currently supported or has no corresponding configuration section.", active_service_name)),
    };

    let model_config = config.models.get(selected_model_name)
        .ok_or_else(|| anyhow!(
            "Selected model '{}' not found in the [models] section of the config file.",
            selected_model_name
        ))?;

    if model_config.service != *active_service_name {
        return Err(anyhow!(
            "Selected model '{}' is configured for service '{}', but the active service is '{}'.",
            selected_model_name, model_config.service, active_service_name
        ));
    }

    tracing::info!("Successfully loaded and validated configuration from {:?}", config_path);
    Ok(config)
}

// --- New Struct and Loading Logic for Volitionfile.toml ---

/// Represents the configuration loaded from Volitionfile.toml
#[derive(Deserialize, Debug, Clone)]
pub struct VolitionFileConfig {
    pub system_prompt: String,
    // Add other project-specific configurations here later if needed
}

/// Loads configuration from Volitionfile.toml in the current directory.
pub fn load_volition_file_config() -> Result<VolitionFileConfig> {
    let config_path = Path::new("./Volitionfile.toml");
    if !config_path.exists() {
        return Err(anyhow!(
            "Configuration file not found at {:?}. Please create it with a 'system_prompt' key.",
            config_path
        ));
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

    let config: VolitionFileConfig = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

    if config.system_prompt.trim().is_empty() {
         return Err(anyhow!("'system_prompt' key found in {:?} but it is empty.", config_path));
    }

    tracing::info!("Successfully loaded Volitionfile configuration from {:?}", config_path);
    Ok(config)
}

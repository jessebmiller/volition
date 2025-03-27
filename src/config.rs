use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::env;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use toml;
use tracing; // Ensure tracing is in scope
use url::Url; // Added for URL validation

// --- Combined Configuration Structure ---

/// Represents the combined configuration loaded from Volition.toml and environment variables.
#[derive(Deserialize, Debug, Clone)]
pub struct RuntimeConfig {
    pub system_prompt: String,
    // Removed active_service field
    pub openai: OpenAIConfig, // Specifies the default selected model via its key in the [models] map
    pub models: HashMap<String, ModelConfig>, // Map of model identifier -> model config

    #[serde(skip)] // API key is loaded from environment, not the file
    pub api_key: String,
}

// Removed ActiveService struct

#[derive(Deserialize, Debug, Clone)]
pub struct OpenAIConfig {
    pub selected_model: String, // Identifier (key) for the default model in the [models] map
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String, // The actual model name to be sent in the API request (e.g., "gpt-4-turbo")
    // Removed service field
    pub parameters: toml::Value, // Model-specific parameters (e.g., temperature, max_tokens)
    pub endpoint: String, // The base URL endpoint for the API providing this model (REQUIRED, e.g., "https://api.openai.com/v1")
}

/// Loads configuration from Volition.toml in the current directory and API key from environment.
pub fn load_runtime_config() -> Result<RuntimeConfig> {
    // --- Load API Key from Environment Variable ---
    let api_key = env::var("API_KEY")
        .context("Failed to read API_KEY environment variable. Please ensure it is set.")?;
    if api_key.is_empty() {
        return Err(anyhow!("API_KEY environment variable is set but empty."));
    }

    // --- Load Configuration File (Volition.toml) ---
    let config_path = Path::new("./Volition.toml");
    if !config_path.exists() {
        return Err(anyhow!(
            "Project configuration file not found at {:?}. Please create it.",
            config_path
        ));
    }

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read project config file: {:?}", config_path))?;

    // Deserialize the configuration file.
    // This will automatically fail if any model in [models] is missing the required 'endpoint' field.
    let mut config: RuntimeConfig = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse project config file: {:?}. Check syntax and ensure all models have 'model_name', 'parameters', and 'endpoint' fields.", config_path))?;

    // --- Populate API Key ---
    config.api_key = api_key;

    // --- Validation ---
    if config.system_prompt.trim().is_empty() {
        return Err(anyhow!("'system_prompt' in {:?} is empty.", config_path));
    }
    if config.openai.selected_model.trim().is_empty() {
        return Err(anyhow!("'selected_model' in [openai] section of {:?} is empty.", config_path));
    }
     if config.models.is_empty() {
        return Err(anyhow!("The [models] section in {:?} is empty. Define at least one model.", config_path));
    }

    // Determine the selected model name (key) directly from the openai config.
    let selected_model_key = &config.openai.selected_model;

    // Check if the selected model key exists in the models map
    if !config.models.contains_key(selected_model_key) {
         return Err(anyhow!(
            "Selected model key '{}' specified in [openai] not found in the [models] section of {:?}.",
            selected_model_key, config_path
        ));
    }

    // Validate all defined models
    for (key, model) in &config.models {
        if model.model_name.trim().is_empty() {
             return Err(anyhow!(
                "Model definition '{}' in {:?} has an empty 'model_name'.",
                key, config_path
            ));
        }
        if model.endpoint.trim().is_empty() {
            // This check is technically redundant due to Url::parse below, but kept for clarity
            return Err(anyhow!(
                "Model definition '{}' in {:?} has an empty 'endpoint'.",
                key, config_path
            ));
        }
        // Add URL parsing validation
        Url::parse(&model.endpoint)
            .with_context(|| format!("Invalid URL format for endpoint ('{}') in model definition '{}' in {:?}", model.endpoint, key, config_path))?;
    }

    tracing::info!("Successfully loaded and validated configuration from {:?} and environment", config_path);
    Ok(config)
}

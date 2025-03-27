use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::env;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use toml;
use tracing; // Ensure tracing is in scope

// --- Combined Configuration Structure ---

/// Represents the combined configuration loaded from Volition.toml and environment variables.
#[derive(Deserialize, Debug, Clone)]
pub struct RuntimeConfig {
    pub system_prompt: String,
    pub active_service: ActiveService,
    pub openai: OpenAIConfig, // Keep for OpenAI-specific settings like selected_model
    // Add other service-specific config structs here if needed (e.g., pub ollama: OllamaConfig)
    pub models: HashMap<String, ModelConfig>,

    #[serde(skip)] // API key is loaded from environment, not the file
    pub api_key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ActiveService {
    pub service: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OpenAIConfig {
    pub selected_model: String,
}

// Add other service-specific config structs here (e.g., OllamaConfig) if they have unique fields

#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub service: String, // Indicates which service configuration this model uses (e.g., "openai", "ollama")
    pub parameters: toml::Value,
    #[serde(default)]
    pub endpoint_override: Option<String>,
}

/// Loads configuration from Volition.toml in the current directory and API key from environment.
pub fn load_runtime_config() -> Result<RuntimeConfig> {
    // --- Load API Key from Environment Variable ---
    // IMPORTANT: Keep loading API key from environment for security.
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

    let mut config: RuntimeConfig = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse project config file: {:?}", config_path))?;

    // --- Populate API Key ---
    config.api_key = api_key;

    // --- Validation ---
    if config.system_prompt.trim().is_empty() {
        return Err(anyhow!("'system_prompt' key found in {:?} but it is empty.", config_path));
    }

    let active_service_name = &config.active_service.service;

    // Determine the selected model name based on the active service.
    // This requires the corresponding service config section (e.g., [openai]) to exist.
    let selected_model_name = match active_service_name.as_str() {
        "openai" => &config.openai.selected_model,
        // Add cases for other services if they have a 'selected_model' field
        // "ollama" => &config.ollama.selected_model, // Example
        _ => {
            // If the service doesn't have a dedicated section with 'selected_model',
            // we might infer it or require a specific model definition.
            // For now, let's assume services needing selection have a dedicated section.
            return Err(anyhow!(
                "Active service '{}' is specified, but its configuration section (e.g., [{}]) with a 'selected_model' field is missing or the service is not supported for model selection this way.",
                active_service_name, active_service_name
            ));
        }
    };

    // Check if the selected model exists in the models map
    let model_config = config.models.get(selected_model_name)
        .ok_or_else(|| anyhow!(
            "Selected model '{}' not found in the [models] section of the config file.",
            selected_model_name
        ))?;

    // Check if the selected model's service matches the active service
    // Use case-insensitive comparison for flexibility
    if !model_config.service.eq_ignore_ascii_case(active_service_name) {
        return Err(anyhow!(
            "Selected model '{}' is configured for service '{}', but the active service is '{}'.",
            selected_model_name, model_config.service, active_service_name
        ));
    }

    tracing::info!("Successfully loaded and validated configuration from {:?} and environment", config_path);
    Ok(config)
}

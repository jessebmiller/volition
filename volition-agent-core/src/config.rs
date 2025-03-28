// volition-agent-core/src/config.rs
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

// --- Combined Configuration Structure ---

/// Represents the combined configuration loaded from Volition.toml and environment variables.
#[derive(Deserialize, Debug, Clone)]
pub struct RuntimeConfig {
    pub system_prompt: String,
    pub selected_model: String,
    pub models: HashMap<String, ModelConfig>,

    #[serde(skip)]
    pub api_key: String,

    #[serde(skip)]
    pub project_root: PathBuf,
}

/// Represents the configuration for a specific AI model.
#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub parameters: toml::Value,
    pub endpoint: String,
}

impl RuntimeConfig {
    /// Returns a reference to the currently selected ModelConfig.
    ///
    /// # Errors
    ///
    /// Returns an error if the `selected_model` key does not exist in the `models` map.
    /// This should ideally not happen if `load_runtime_config` validation passed.
    pub fn selected_model_config(&self) -> Result<&ModelConfig> {
        self.models.get(&self.selected_model).ok_or_else(|| {
            anyhow!(
                "Internal inconsistency: Selected model key '{}' not found in models map.",
                self.selected_model
            )
        })
    }
}

/// Loads configuration from Volition.toml in the current directory and API key from environment.
pub fn load_runtime_config() -> Result<RuntimeConfig> {
    let api_key = env::var("API_KEY")
        .context("Failed to read API_KEY environment variable. Please ensure it is set.")?;
    if api_key.is_empty() {
        return Err(anyhow!("API_KEY environment variable is set but empty."));
    }

    let config_path = Path::new("./Volition.toml");
    if !config_path.exists() {
        return Err(anyhow!(
            "Project configuration file not found at {:?}. Please create it.",
            config_path
        ));
    }

    let absolute_config_path = config_path
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize config file path: {:?}.", config_path))?;
    let project_root = absolute_config_path
        .parent()
        .ok_or_else(|| {
            anyhow!(
                "Failed to determine project root directory from config path: {:?}",
                absolute_config_path
            )
        })?
        .to_path_buf();
    tracing::debug!("Determined project root: {:?}", project_root);

    let config_str = fs::read_to_string(&absolute_config_path)
        .with_context(|| format!("Failed to read project config file: {:?}", absolute_config_path))?;

    let partial_config: RuntimeConfigPartial = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse project config file: {:?}. Check TOML syntax.", absolute_config_path))?;

    let config = RuntimeConfig {
        system_prompt: partial_config.system_prompt,
        selected_model: partial_config.selected_model,
        models: partial_config.models,
        api_key,
        project_root,
    };

    // --- Validation ---
    if config.system_prompt.trim().is_empty() {
        return Err(anyhow!("'system_prompt' in {:?} is empty.", absolute_config_path));
    }
    if config.selected_model.trim().is_empty() {
        return Err(anyhow!("Top-level 'selected_model' key in {:?} is empty.", absolute_config_path));
    }
    if config.models.is_empty() {
        return Err(anyhow!("The [models] section in {:?} is empty. Define at least one model.", absolute_config_path));
    }

    // Check if selected model exists (using the new method for consistency)
    config.selected_model_config().with_context(|| format!("Validation failed for selected model specified in {:?}", absolute_config_path))?;

    for (key, model) in &config.models {
        if model.model_name.trim().is_empty() {
            return Err(anyhow!("Model definition '{}' in {:?} has an empty 'model_name'.", key, absolute_config_path));
        }
        if model.endpoint.trim().is_empty() {
            return Err(anyhow!("Model definition '{}' in {:?} has an empty 'endpoint'.", key, absolute_config_path));
        }
        Url::parse(&model.endpoint).with_context(|| {
            format!("Invalid URL format for endpoint ('{}') in model definition '{}' in {:?}.", model.endpoint, key, absolute_config_path)
        })?;
    }

    tracing::info!("Successfully loaded and validated configuration from {:?} and environment", absolute_config_path);
    Ok(config)
}

#[derive(Deserialize)]
struct RuntimeConfigPartial {
    system_prompt: String,
    selected_model: String,
    models: HashMap<String, ModelConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use toml;

    fn create_valid_config_toml(dir: &Path) -> PathBuf {
        let config_path = dir.join("Volition.toml");
        let content = r#"
            system_prompt = "You are a helpful assistant."
            selected_model = "gpt4"
            [models.gpt4]
            model_name = "gpt-4-turbo"
            endpoint = "https://api.openai.com/v1"
            parameters = { temperature = 0.7 }
            [models.ollama_llama3]
            model_name = "llama3"
            endpoint = "http://localhost:11434/api"
            parameters = { temperature = 0.5, top_p = 0.9 }
        "#;
        fs::write(&config_path, content).expect("Failed to write dummy config file");
        config_path
    }

    #[test]
    fn test_selected_model_config_success() {
        let mut models = HashMap::new();
        let gpt4_config = ModelConfig {
            model_name: "gpt-4-turbo".to_string(),
            endpoint: "https://api.openai.com/v1".to_string(),
            parameters: toml::Value::Table(Default::default()),
        };
        models.insert("gpt4".to_string(), gpt4_config.clone());

        let config = RuntimeConfig {
            system_prompt: "Test".to_string(),
            selected_model: "gpt4".to_string(),
            models,
            api_key: "dummy".to_string(),
            project_root: PathBuf::from("."),
        };

        let selected = config.selected_model_config();
        assert!(selected.is_ok());
        let selected = selected.unwrap();
        assert_eq!(selected.model_name, gpt4_config.model_name);
        assert_eq!(selected.endpoint, gpt4_config.endpoint);
    }

    #[test]
    fn test_selected_model_config_failure() {
        let config = RuntimeConfig {
            system_prompt: "Test".to_string(),
            selected_model: "nonexistent".to_string(), // Key not in models
            models: HashMap::new(), // Empty map
            api_key: "dummy".to_string(),
            project_root: PathBuf::from("."),
        };

        let selected = config.selected_model_config();
        assert!(selected.is_err());
        let error_msg = selected.err().unwrap().to_string();
        assert!(error_msg.contains("Selected model key 'nonexistent' not found"));
    }

    // --- Existing load_runtime_config tests (remain ignored due to env var issues) ---
    #[test]
    #[ignore]
    fn test_load_config_success() {
        let dir = tempdir().expect("Failed to create temp dir");
        let _config_path = create_valid_config_toml(dir.path());
        let api_key = "test_api_key_123";
        env::set_var("API_KEY", api_key);
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change current dir");
        let result = load_runtime_config();
        env::remove_var("API_KEY");
        env::set_current_dir(&original_dir).expect("Failed to restore current dir");
        assert!(result.is_ok(), "Load failed: {:?}", result.err());
        let config = result.unwrap();
        assert_eq!(config.api_key, api_key);
        assert_eq!(config.selected_model, "gpt4");
        // Check using the new method
        assert!(config.selected_model_config().is_ok());
        assert_eq!(config.selected_model_config().unwrap().model_name, "gpt-4-turbo");
    }

    #[test]
    #[ignore]
    fn test_load_config_missing_file() { /* ... unchanged ... */ }
    #[test]
    #[ignore]
    fn test_load_config_missing_api_key() { /* ... unchanged ... */ }
    #[test]
    #[ignore]
    fn test_load_config_empty_api_key() { /* ... unchanged ... */ }
    #[test]
    #[ignore]
    fn test_load_config_invalid_toml() { /* ... unchanged ... */ }
    #[test]
    #[ignore]
    fn test_load_config_validation_missing_selected() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_path = dir.path().join("Volition.toml");
        let content = r#"
            system_prompt = "Valid prompt"
            selected_model = "nonexistent"
            [models.gpt4]
            model_name = "gpt-4-turbo"
            endpoint = "https://api.openai.com/v1"
            parameters = { temperature = 0.7 }
        "#;
        fs::write(&config_path, content).expect("Failed to write config file");
        env::set_var("API_KEY", "dummy_key");
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change current dir");
        let result = load_runtime_config();
        env::remove_var("API_KEY");
        env::set_current_dir(&original_dir).expect("Failed to restore current dir");
        assert!(result.is_err());
        // Check that the error comes from the selected_model_config check
        assert!(result.err().unwrap().to_string().contains("Selected model key 'nonexistent' not found"));
    }
    #[test]
    #[ignore]
    fn test_load_config_validation_empty_field() { /* ... unchanged ... */ }
    #[test]
    #[ignore]
    fn test_load_config_validation_invalid_endpoint_url() { /* ... unchanged ... */ }
}

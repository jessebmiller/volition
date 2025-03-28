// volition-agent-core/src/config.rs

//! Handles configuration structures and parsing for the agent library.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

// --- Configuration Structures ---

/// Represents the validated runtime configuration needed by the [`Agent`].
///
/// This struct is typically created by parsing a TOML configuration source
/// using [`RuntimeConfig::from_toml_str`]. It does not include environment-specific
/// details like the project root path.
#[derive(Deserialize, Debug, Clone)]
pub struct RuntimeConfig {
    /// The system prompt to guide the AI model's behavior.
    pub system_prompt: String,
    /// The key selecting the default model from the `models` map.
    pub selected_model: String,
    /// A map containing configurations for available AI models, keyed by a user-defined identifier.
    pub models: HashMap<String, ModelConfig>,
    /// The API key used for authenticating with the AI model endpoint.
    /// This is not deserialized from TOML but provided separately.
    #[serde(skip)]
    pub api_key: String,
}

/// Represents the configuration for a specific AI model endpoint.
#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    /// The specific model name expected by the AI endpoint (e.g., "gpt-4-turbo").
    pub model_name: String,
    /// Additional parameters specific to the model (e.g., temperature, max_tokens).
    /// Expected to be a TOML table.
    pub parameters: toml::Value,
    /// The full URL of the AI model's chat completion endpoint.
    pub endpoint: String,
}

impl RuntimeConfig {
    /// Returns a reference to the currently selected [`ModelConfig`].
    ///
    /// # Errors
    ///
    /// Returns an error if the `selected_model` key stored in this `RuntimeConfig`
    /// does not exist in the `models` map.
    pub fn selected_model_config(&self) -> Result<&ModelConfig> {
        self.models.get(&self.selected_model).ok_or_else(|| {
            anyhow!(
                "Selected model key '{}' not found in models map.",
                self.selected_model
            )
        })
    }

    /// Parses TOML configuration content and validates it against the provided API key.
    ///
    /// This is the primary way to create a [`RuntimeConfig`]. It ensures the TOML
    /// is valid, required fields are present, the selected model exists, and URLs are valid.
    ///
    /// # Arguments
    ///
    /// * `config_toml_content`: A string slice containing the TOML configuration.
    /// * `api_key`: The API key (read from environment or other source by the caller).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The `api_key` is empty.
    /// * The `config_toml_content` is not valid TOML.
    /// * Required fields (`system_prompt`, `selected_model`, `models` table) are missing or empty.
    /// * The `selected_model` key does not correspond to an entry in the `models` table.
    /// * Any model definition is missing `model_name` or `endpoint`.
    /// * Any model endpoint URL is invalid.
    /// * Any model `parameters` field is not a TOML table.
    pub fn from_toml_str(config_toml_content: &str, api_key: String) -> Result<RuntimeConfig> {
        if api_key.is_empty() {
            return Err(anyhow!("Provided API key is empty."));
        }

        let partial_config: RuntimeConfigPartial = toml::from_str(config_toml_content)
            .context("Failed to parse configuration TOML content. Check TOML syntax.")?;

        let config = RuntimeConfig {
            system_prompt: partial_config.system_prompt,
            selected_model: partial_config.selected_model,
            models: partial_config.models,
            api_key,
        };

        // --- Validation ---
        if config.system_prompt.trim().is_empty() {
            return Err(anyhow!("'system_prompt' in config content is empty."));
        }
        if config.selected_model.trim().is_empty() {
            return Err(anyhow!(
                "Top-level 'selected_model' key in config content is empty."
            ));
        }
        if config.models.is_empty() {
            return Err(anyhow!("The [models] section in config content is empty."));
        }

        // Check selected model exists
        config
            .selected_model_config()
            .context("Validation failed for selected model")?;

        for (key, model) in &config.models {
            if model.model_name.trim().is_empty() {
                return Err(anyhow!(
                    "Model definition '{}' has an empty 'model_name'.",
                    key
                ));
            }
            if model.endpoint.trim().is_empty() {
                return Err(anyhow!(
                    "Model definition '{}' has an empty 'endpoint'.",
                    key
                ));
            }
            Url::parse(&model.endpoint).with_context(|| {
                format!(
                    "Invalid URL format for endpoint ('{}') in model definition '{}'.",
                    model.endpoint, key
                )
            })?;
            if !model.parameters.is_table()
                && !model.parameters.is_str()
                && model.parameters.as_str() != Some("{}")
            {
                return Err(anyhow!(
                    "Model definition '{}' has invalid 'parameters'. Expected a TOML table.",
                    key
                ));
            }
        }

        tracing::info!("Successfully parsed and validated configuration content.");
        Ok(config)
    }
}

/// Helper for initial deserialization from TOML content.
#[derive(Deserialize)]
struct RuntimeConfigPartial {
    system_prompt: String,
    selected_model: String,
    models: HashMap<String, ModelConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml;

    fn valid_config_content() -> String {
        r#"
            system_prompt = "You are a helpful assistant."
            selected_model = "gpt4"
            [models.gpt4]
            model_name = "gpt-4-turbo"
            endpoint = "https://api.openai.com/v1"
            parameters = { temperature = 0.7 }
            [models.ollama_llama3]
            model_name = "llama3"
            endpoint = "http://localhost:11434/api"
            parameters = { top_p = 0.9 }
        "#
        .to_string()
    }

    fn create_dummy_runtime_config(
        selected_key: &str,
        models_map: HashMap<String, ModelConfig>,
        api_key: String,
    ) -> RuntimeConfig {
        RuntimeConfig {
            system_prompt: "Dummy prompt".to_string(),
            selected_model: selected_key.to_string(),
            models: models_map,
            api_key,
        }
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
        let config = create_dummy_runtime_config("gpt4", models, "dummy".to_string());
        let selected = config.selected_model_config().unwrap();
        assert_eq!(selected.model_name, gpt4_config.model_name);
    }

    #[test]
    fn test_selected_model_config_failure() {
        let config =
            create_dummy_runtime_config("nonexistent", HashMap::new(), "dummy".to_string());
        let selected = config.selected_model_config();
        assert!(selected.is_err());
        let err_msg = selected.err().unwrap().to_string();
        println!("test_selected_model_config_failure Error: {}", err_msg);
        assert!(err_msg.contains("Selected model key 'nonexistent' not found in models map."));
    }

    #[test]
    fn test_from_toml_str_success() {
        let content = valid_config_content();
        let api_key = "test_api_key_123".to_string();
        let result = RuntimeConfig::from_toml_str(&content, api_key.clone());
        assert!(result.is_ok(), "Parse/validate failed: {:?}", result.err());
        let config = result.unwrap();
        assert_eq!(config.api_key, api_key);
        assert_eq!(config.selected_model, "gpt4");
    }

    #[test]
    fn test_from_toml_str_empty_api_key() {
        let content = valid_config_content();
        let result = RuntimeConfig::from_toml_str(&content, "".to_string());
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("Provided API key is empty"));
    }

    #[test]
    fn test_from_toml_str_invalid_toml() {
        let content = "this is not valid toml";
        let result = RuntimeConfig::from_toml_str(content, "dummy_key".to_string());
        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("Failed to parse configuration TOML content"));
    }

    #[test]
    fn test_from_toml_str_missing_selected_key() {
        let content = r#"
            system_prompt = "Valid"
            selected_model = "nonexistent"
            [models.gpt4]
            model_name = "g"
            endpoint = "http://example.com"
            parameters = {}
        "#;
        let result = RuntimeConfig::from_toml_str(content, "dummy_key".to_string());
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        println!("test_from_toml_str_missing_selected_key Error: {}", err_msg);
        assert!(err_msg.contains("Validation failed for selected model"));
    }

    #[test]
    fn test_from_toml_str_empty_system_prompt() {
        let content = r#"
            system_prompt = "" 
            selected_model = "gpt4"
            [models.gpt4]
            model_name = "g"
            endpoint = "http://example.com"
            parameters = {}
        "#;
        let result = RuntimeConfig::from_toml_str(content, "dummy_key".to_string());
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        println!("test_from_toml_str_empty_system_prompt Error: {}", err_msg);
        assert!(err_msg.contains("'system_prompt' in config content is empty."));
    }

    #[test]
    fn test_from_toml_str_invalid_endpoint() {
        let content = r#"
            system_prompt = "Valid"
            selected_model = "gpt4"
            [models.gpt4]
            model_name = "g"
            endpoint = "invalid url"
            parameters = {}
        "#;
        let result = RuntimeConfig::from_toml_str(content, "dummy_key".to_string());
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        println!("test_from_toml_str_invalid_endpoint Error: {}", err_msg);
        assert!(err_msg.contains("Invalid URL format for endpoint ('invalid url')"));
    }
}

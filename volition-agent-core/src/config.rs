// volition-agent-core/src/config.rs
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

// --- Configuration Structures ---

#[derive(Deserialize, Debug, Clone)]
pub struct RuntimeConfig {
    pub system_prompt: String,
    pub selected_model: String,
    pub models: HashMap<String, ModelConfig>,
    #[serde(skip)]
    pub api_key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub parameters: toml::Value,
    pub endpoint: String,
}

impl RuntimeConfig {
    pub fn selected_model_config(&self) -> Result<&ModelConfig> {
        self.models.get(&self.selected_model).ok_or_else(|| {
            anyhow!(
                "Internal inconsistency: Selected model key '{}' not found in models map.",
                self.selected_model
            )
        })
    }
}

pub fn parse_and_validate_config(
    config_toml_content: &str,
    api_key: String,
) -> Result<RuntimeConfig> {
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
        return Err(anyhow!(
            "'system_prompt' in config content is empty."
        ));
    }
    if config.selected_model.trim().is_empty() {
        return Err(anyhow!(
            "Top-level 'selected_model' key in config content is empty."
        ));
    }
    if config.models.is_empty() {
        return Err(anyhow!(
            "The [models] section in config content is empty."
        ));
    }

    config.selected_model_config().context("Validation failed for selected model in config content")?;

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
                model.endpoint,
                key
            )
        })?;
        // Use is_str() instead of is_string()
        if !model.parameters.is_table() && !model.parameters.is_str() && model.parameters.as_str() != Some("{}") {
             return Err(anyhow!(
                "Model definition '{}' has invalid 'parameters'. Expected a TOML table.",
                key
            ));
        }
    }

    tracing::info!("Successfully parsed and validated configuration content.");
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
    use std::path::PathBuf; // Keep for RuntimeConfig creation even if unused
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
        "#.to_string()
    }

    // Helper to create RuntimeConfig without project_root
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
            // project_root: PathBuf::from("."), // Removed
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

        let selected = config.selected_model_config();
        assert!(selected.is_ok());
        let selected = selected.unwrap();
        assert_eq!(selected.model_name, gpt4_config.model_name);
    }

    #[test]
    fn test_selected_model_config_failure() {
        let config = create_dummy_runtime_config("nonexistent", HashMap::new(), "dummy".to_string());
        let selected = config.selected_model_config();
        assert!(selected.is_err());
        assert!(selected.err().unwrap().to_string().contains("Selected model key 'nonexistent' not found"));
    }

    #[test]
    fn test_parse_validate_success() {
        let content = valid_config_content();
        let api_key = "test_api_key_123".to_string();
        let result = parse_and_validate_config(&content, api_key.clone());
        assert!(result.is_ok(), "Parse/validate failed: {:?}", result.err());
        let config = result.unwrap();
        assert_eq!(config.api_key, api_key);
        assert_eq!(config.selected_model, "gpt4");
        assert!(config.models.contains_key("gpt4"));
        assert!(config.models.contains_key("ollama_llama3"));
        assert_eq!(config.models["gpt4"].model_name, "gpt-4-turbo");
    }

    #[test]
    fn test_parse_validate_empty_api_key() {
        let content = valid_config_content();
        let result = parse_and_validate_config(&content, "".to_string());
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("Provided API key is empty"));
    }

    #[test]
    fn test_parse_validate_invalid_toml() {
        let content = "this is not valid toml";
        let result = parse_and_validate_config(content, "dummy_key".to_string());
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("Failed to parse configuration TOML content"));
    }

    #[test]
    fn test_parse_validate_missing_selected_key() {
        let content = r#"system_prompt = "Valid" selected_model = "nonexistent" [models.gpt4] model_name = "g" endpoint = "e" parameters = {}"#;
        let result = parse_and_validate_config(content, "dummy_key".to_string());
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("Selected model key 'nonexistent' not found"));
    }

    #[test]
    fn test_parse_validate_empty_system_prompt() {
        let content = r#"system_prompt = "" selected_model = "gpt4" [models.gpt4] model_name = "g" endpoint = "e" parameters = {}"#;
        let result = parse_and_validate_config(content, "dummy_key".to_string());
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("'system_prompt' in config content is empty"));
    }

    #[test]
    fn test_parse_validate_invalid_endpoint() {
        let content = r#"system_prompt = "Valid" selected_model = "gpt4" [models.gpt4] model_name = "g" endpoint = "invalid url" parameters = {}"#;
        let result = parse_and_validate_config(content, "dummy_key".to_string());
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("Invalid URL format for endpoint"));
    }
}

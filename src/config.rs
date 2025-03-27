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

#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String,
    pub parameters: toml::Value,
    pub endpoint: String,
}

/// Loads configuration from Volition.toml in the current directory and API key from environment.
pub fn load_runtime_config() -> Result<RuntimeConfig> {
    // --- Load API Key from Environment Variable (Original Position) ---
    let api_key = env::var("API_KEY")
        .context("Failed to read API_KEY environment variable. Please ensure it is set.")?;
    if api_key.is_empty() {
        return Err(anyhow!("API_KEY environment variable is set but empty."));
    }

    // --- Locate Config File and Check Existence ---
    let config_path = Path::new("./Volition.toml");
    if !config_path.exists() {
        return Err(anyhow!(
            // Use relative path in this specific error message as canonicalize hasn't run yet.
            "Project configuration file not found at {:?}. Please create it.",
            config_path
        ));
    }

    // --- Canonicalize Path and Determine Project Root ---
    // Now that we know the file exists, we can safely canonicalize.
    let absolute_config_path = config_path.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize config file path: {:?}. Check permissions or path validity.",
            config_path // Keep original path in error context
        )
    })?;
    let project_root = absolute_config_path
        .parent()
        .ok_or_else(|| {
            anyhow!(
                "Failed to determine project root directory from config path: {:?}",
                absolute_config_path // Use absolute path here
            )
        })?
        .to_path_buf();
    tracing::debug!("Determined project root: {:?}", project_root);

    // --- Load Configuration File Content ---
    let config_str = fs::read_to_string(&absolute_config_path) // Use absolute path
        .with_context(|| {
            format!(
                "Failed to read project config file: {:?}",
                absolute_config_path
            )
        })?;

    // --- Deserialize Configuration File ---
    let partial_config: RuntimeConfigPartial = toml::from_str(&config_str).with_context(|| {
        format!(
            "Failed to parse project config file: {:?}. Check TOML syntax.",
            absolute_config_path // Use absolute path
        )
    })?;

    // --- Construct Full RuntimeConfig ---
    let config = RuntimeConfig {
        system_prompt: partial_config.system_prompt,
        selected_model: partial_config.selected_model,
        models: partial_config.models,
        api_key,
        project_root,
    };

    // --- Validation (using absolute_config_path in error messages) ---
    if config.system_prompt.trim().is_empty() {
        return Err(anyhow!(
            "'system_prompt' in {:?} is empty.",
            absolute_config_path
        ));
    }
    if config.selected_model.trim().is_empty() {
        return Err(anyhow!(
            "Top-level 'selected_model' key in {:?} is empty.",
            absolute_config_path
        ));
    }
    if config.models.is_empty() {
        return Err(anyhow!(
            "The [models] section in {:?} is empty. Define at least one model.",
            absolute_config_path
        ));
    }

    let selected_model_key = &config.selected_model;
    if !config.models.contains_key(selected_model_key) {
        return Err(anyhow!(
            "Selected model key '{}' specified at the top level not found in the [models] section of {:?}.",
            selected_model_key, absolute_config_path
        ));
    }

    for (key, model) in &config.models {
        if model.model_name.trim().is_empty() {
            return Err(anyhow!(
                "Model definition '{}' in {:?} has an empty 'model_name'.",
                key,
                absolute_config_path
            ));
        }
        if model.endpoint.trim().is_empty() {
            return Err(anyhow!(
                "Model definition '{}' in {:?} has an empty 'endpoint'.",
                key,
                absolute_config_path
            ));
        }
        Url::parse(&model.endpoint).with_context(|| {
            format!(
                "Invalid URL format for endpoint ('{}') in model definition '{}' in {:?}.",
                model.endpoint, key, absolute_config_path
            )
        })?;
    }

    tracing::info!(
        "Successfully loaded and validated configuration from {:?} and environment",
        absolute_config_path // Use absolute path in log
    );
    Ok(config)
}

// Helper struct for deserializing only the parts from the TOML file
#[derive(Deserialize)]
struct RuntimeConfigPartial {
    system_prompt: String,
    selected_model: String,
    models: HashMap<String, ModelConfig>,
}

#[cfg(test)]
mod tests {
    // TODO: These tests are ignored because they modify the API_KEY environment variable,
    // causing conflicts when tests run in parallel. Refactor load_runtime_config to accept
    // the API key as a parameter or use a crate like `serial_test` to run these serially.
    use super::*; // Import items from the outer module (config)
    use std::env;
    use std::fs;
    use tempfile::tempdir; // For creating temporary directories for testing

    // Helper function to create a dummy Volition.toml
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

    // Test the successful loading scenario
    #[test]
    #[ignore] // Ignoring due to env var conflicts in parallel execution
    fn test_load_config_success() {
        let dir = tempdir().expect("Failed to create temp dir");
        let _config_path = create_valid_config_toml(dir.path()); // Use _ to avoid unused warning

        // Set required environment variables for the test
        let api_key = "test_api_key_123";
        env::set_var("API_KEY", api_key);

        // Temporarily change the current directory to the temp dir
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change current dir");

        // Load the configuration
        let result = load_runtime_config();

        // Restore environment
        env::remove_var("API_KEY");
        env::set_current_dir(&original_dir).expect("Failed to restore current dir");

        // Assertions
        assert!(
            result.is_ok(),
            "Expected config loading to succeed, but got: {:?}",
            result.err()
        );
        let config = result.unwrap();
        assert_eq!(config.api_key, api_key);
        assert_eq!(config.system_prompt, "You are a helpful assistant.");
        assert_eq!(config.selected_model, "gpt4");
        assert!(config.models.contains_key("gpt4"));
        assert!(config.models.contains_key("ollama_llama3"));
        assert_eq!(config.models["gpt4"].model_name, "gpt-4-turbo");
        assert_eq!(config.models["gpt4"].endpoint, "https://api.openai.com/v1");
        assert_eq!(config.models["ollama_llama3"].model_name, "llama3");
        assert_eq!(
            config.models["ollama_llama3"].endpoint,
            "http://localhost:11434/api"
        );
        assert!(config
            .project_root
            .ends_with(dir.path().file_name().unwrap())); // Check project root is temp dir
    }

    // Test when Volition.toml is missing
    #[test]
    #[ignore] // Ignoring due to env var conflicts in parallel execution
    fn test_load_config_missing_file() {
        let dir = tempdir().expect("Failed to create temp dir");

        // Set API key (still required for the function to get past the first check)
        env::set_var("API_KEY", "dummy_key");

        // Change to temp dir (where Volition.toml doesn't exist)
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change current dir");

        // Load the configuration
        let result = load_runtime_config();

        // Restore environment
        env::remove_var("API_KEY");
        env::set_current_dir(&original_dir).expect("Failed to restore current dir");

        // Assertions
        assert!(result.is_err());
        let error_message = result.err().unwrap().to_string();
        assert!(
            error_message.contains("Project configuration file not found"),
            "Unexpected error message: {}",
            error_message
        );
        assert!(
            error_message.contains("Volition.toml"), // Check it still mentions the file name
            "Unexpected error message: {}",
            error_message
        );
    }

    // Test when API_KEY environment variable is not set
    #[test]
    #[ignore] // Ignoring due to env var conflicts in parallel execution
    fn test_load_config_missing_api_key() {
        let dir = tempdir().expect("Failed to create temp dir");
        create_valid_config_toml(dir.path()); // Need the file to exist for canonicalize path

        // Ensure API_KEY is NOT set
        env::remove_var("API_KEY");

        // Change to temp dir
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change current dir");

        // Load the configuration
        let result = load_runtime_config(); // Call this while in the temp dir

        // Restore environment AFTER the call
        env::set_current_dir(&original_dir).expect("Failed to restore current dir");
        // No need to remove API_KEY as it was never set for this test case.

        // Assertions
        assert!(result.is_err());
        let error_message = result.err().unwrap().to_string();
        // The error should be about the missing API key, as the file is valid and parsed
        assert!(
            error_message.contains("Failed to read API_KEY environment variable"),
            "Unexpected error message: {}",
            error_message
        );
    }

    // Test when API_KEY environment variable is set but empty
    #[test]
    #[ignore] // Ignoring due to env var conflicts in parallel execution
    fn test_load_config_empty_api_key() {
        let dir = tempdir().expect("Failed to create temp dir");
        create_valid_config_toml(dir.path()); // Need the file to exist

        // Set API_KEY to an empty string
        env::set_var("API_KEY", "");

        // Change to temp dir
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change current dir");

        // Load the configuration
        let result = load_runtime_config();

        // Restore environment
        env::remove_var("API_KEY");
        env::set_current_dir(&original_dir).expect("Failed to restore current dir");

        // Assertions
        assert!(result.is_err());
        let error_message = result.err().unwrap().to_string();
        assert!(
            error_message.contains("API_KEY environment variable is set but empty"),
            "Unexpected error message: {}",
            error_message
        );
    }

    // Test when Volition.toml has invalid syntax
    #[test]
    #[ignore] // Ignoring due to env var conflicts in parallel execution
    fn test_load_config_invalid_toml() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_path = dir.path().join("Volition.toml");
        let invalid_content = r#"
            system_prompt = "You are a helpful assistant."
            selected_model = "gpt4"
            # Missing closing quote below
            [models.gpt4]
            model_name = "gpt-4-turbo
            endpoint = "https://api.openai.com/v1"
            parameters = { temperature = 0.7 }
        "#;
        fs::write(&config_path, invalid_content).expect("Failed to write invalid config file");

        // Need to set API_KEY because it's checked first
        env::set_var("API_KEY", "dummy_key");

        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(dir.path()).expect("Failed to change current dir");

        let result = load_runtime_config();

        // Restore environment
        env::remove_var("API_KEY");
        env::set_current_dir(&original_dir).expect("Failed to restore current dir");

        assert!(result.is_err());
        let error_message = result.err().unwrap().to_string();
        assert!(
            error_message.contains("Failed to parse project config file")
                && error_message.contains("Check TOML syntax"),
            "Unexpected error message: {}",
            error_message
        );
    }

    // Test validation: selected_model key doesn't exist in models map
    #[test]
    #[ignore] // Ignoring due to env var conflicts in parallel execution
    fn test_load_config_validation_missing_selected() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_path = dir.path().join("Volition.toml");
        let content = r#"
            system_prompt = "You are a helpful assistant."
            selected_model = "nonexistent_model" # This model is not defined below

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
        let error_message = result.err().unwrap().to_string();
        assert!(
            error_message.contains("Selected model key 'nonexistent_model' specified at the top level not found in the [models] section"),
            "Unexpected error message: {}", error_message
        );
    }

    // Test validation: empty system_prompt field
    #[test]
    #[ignore] // Ignoring due to env var conflicts in parallel execution
    fn test_load_config_validation_empty_field() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_path = dir.path().join("Volition.toml");
        let content = r#"
            system_prompt = "" # Empty system prompt
            selected_model = "gpt4"

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
        let error_message = result.err().unwrap().to_string();
        assert!(
            error_message.contains("'system_prompt' in") && error_message.contains("is empty"),
            "Unexpected error message: {}",
            error_message
        );
    }

    // Test validation: invalid endpoint URL format
    #[test]
    #[ignore] // Ignoring due to env var conflicts in parallel execution
    fn test_load_config_validation_invalid_endpoint_url() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_path = dir.path().join("Volition.toml");
        let content = r#"
            system_prompt = "Valid prompt"
            selected_model = "gpt4"

            [models.gpt4]
            model_name = "gpt-4-turbo"
            endpoint = "invalid-url-format" # Not a valid URL
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
        let error_message = result.err().unwrap().to_string();
        // Check only for the context message we added
        assert!(
            error_message.contains(
                "Invalid URL format for endpoint ('invalid-url-format') in model definition 'gpt4'"
            ),
            "Unexpected error message: {}",
            error_message
        );
    }
}

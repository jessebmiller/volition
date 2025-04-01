// volition-agent-core/src/config.rs

//! Handles configuration structures and parsing for the agent library.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

// --- New Configuration Structures (MCP Plan) ---

#[derive(Deserialize, Debug, Clone)]
pub struct AgentConfig {
    pub system_prompt: String,
    pub default_provider: String,
    #[serde(default)]
    pub providers: HashMap<String, ProviderInstanceConfig>,
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
    #[serde(default)]
    pub strategies: HashMap<String, StrategyConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProviderInstanceConfig {
    // Use `type` in TOML, map to `provider_type`
    #[serde(rename = "type")]
    pub provider_type: String,
    pub api_key_env_var: String,
    pub model_config: ModelConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StrategyConfig {
    pub planning_provider: Option<String>,
    pub execution_provider: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModelConfig {
    pub model_name: String, 
    #[serde(default)]
    pub parameters: Option<toml::Value>,
    #[serde(default)]
    pub endpoint: Option<String>,
}

impl AgentConfig {
    pub fn from_toml_str(config_toml_content: &str) -> Result<AgentConfig> {
        let config: AgentConfig = match toml::from_str(config_toml_content) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::error!(error=%e, content=%config_toml_content, "Failed to parse TOML content");
                return Err(anyhow!(e)).context("Failed to parse configuration TOML content. Check TOML syntax.");
            }
        };

        // --- Basic Checks ---
        if config.system_prompt.trim().is_empty() {
            return Err(anyhow!("'system_prompt' in config content is empty."));
        }
        if config.default_provider.trim().is_empty() {
            return Err(anyhow!("'default_provider' key in config content is empty."));
        }
        if !config.providers.contains_key(&config.default_provider) {
             return Err(anyhow!(
                "Default provider '{}' not found in [providers] map.",
                config.default_provider
            ));
        }

        // --- Provider Validation ---
        for (key, provider) in &config.providers {
            // Check provider_type (which corresponds to `type` in TOML)
            if provider.provider_type.trim().is_empty() {
                return Err(anyhow!("Provider '{}' is missing 'type' (provider_type).", key));
            }
            if provider.model_config.model_name.trim().is_empty() {
                 return Err(anyhow!("Provider '{}' is missing 'model_config.model_name'.", key));
            }
             if provider.api_key_env_var.trim().is_empty() && provider.provider_type != "ollama" { // Allow empty for ollama
                 return Err(anyhow!("Provider '{}' is missing 'api_key_env_var'.", key));
            }
            if let Some(endpoint) = &provider.model_config.endpoint {
                 if endpoint.trim().is_empty() {
                    return Err(anyhow!("Provider '{}' has an empty 'model_config.endpoint'.", key));
                 }
                 Url::parse(endpoint).with_context(|| {
                    format!("Invalid URL format for endpoint ('{}') in provider '{}'.", endpoint, key)
                 })?;
            } else if provider.provider_type != "ollama" { 
                 // Allow missing endpoint if type is ollama (it has a default)
                 // Consider adding validation if endpoint is strictly required for other types
            }
            if let Some(params) = &provider.model_config.parameters {
                 if !params.is_table() && !params.is_str() {
                     return Err(anyhow!(
                        "Provider '{}' has invalid 'model_config.parameters'. Expected a TOML table or string.",
                        key
                    ));
                 }
            }
        }
        
        // --- MCP Server Validation ---
        for (key, server) in &config.mcp_servers {
             if server.command.trim().is_empty() {
                 return Err(anyhow!("MCP Server '{}' has an empty 'command'.", key));
            }
        }

        tracing::info!("Successfully parsed and validated agent configuration.");
        Ok(config)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // Ensure this fixture uses `type` as expected by the rename
    fn valid_mcp_config_content() -> String {
        r#"
            system_prompt = "You are Volition MCP."
            default_provider = "gemini_default"

            [providers.gemini_default]
            type = "gemini" # Use `type` here
            api_key_env_var = "GOOGLE_API_KEY"
            [providers.gemini_default.model_config]
                model_name = "gemini-2.5-pro"
                endpoint = "https://example.com/gemini"
                parameters = { temperature = 0.6 }
            
            [providers.openai_fast]
            type = "openai" # Use `type` here
            api_key_env_var = "OPENAI_API_KEY"
            [providers.openai_fast.model_config]
                model_name = "gpt-4o-mini"
                endpoint = "https://example.com/openai"
                parameters = { temperature = 0.1 }

            [mcp_servers.filesystem]
            command = "echo"
            args = ["fs"]
            
            [mcp_servers.shell]
            command = "echo"
            args = ["sh"]

            [strategies.plan_execute]
            planning_provider = "openai_fast"
            execution_provider = "gemini_default"
        "#
        .to_string()
    }

    #[test]
    fn test_mcp_config_parse_success() {
        let content = valid_mcp_config_content();
        let result = AgentConfig::from_toml_str(&content);
        // Add context to the assertion
        assert!(result.is_ok(), "Parse failed: {:?}\nContent:\n{}", result.err(), content);
        let config = result.unwrap();
        assert_eq!(config.default_provider, "gemini_default");
        assert_eq!(config.providers.len(), 2);
        assert!(config.providers.contains_key("gemini_default"));
        // Check provider_type after rename
        assert_eq!(config.providers["gemini_default"].provider_type, "gemini");
        assert_eq!(config.providers["openai_fast"].provider_type, "openai");
        assert_eq!(config.providers["openai_fast"].model_config.model_name, "gpt-4o-mini"); 
        assert!(config.providers["gemini_default"].model_config.parameters.is_some());
        assert_eq!(config.mcp_servers.len(), 2);
        assert_eq!(config.mcp_servers["filesystem"].command, "echo");
        assert_eq!(config.strategies.len(), 1);
        assert_eq!(config.strategies["plan_execute"].planning_provider, Some("openai_fast".to_string()));
    }

     #[test]
    fn test_mcp_config_missing_default_provider_def() {
        // Ensure this fixture also uses `type`
        let content = r#"
            system_prompt = "Valid"
            default_provider = "missing_provider"
            [providers.gemini_default]
            type = "gemini" # Use `type` here
            api_key_env_var = "GOOGLE_API_KEY"
            [providers.gemini_default.model_config]
                model_name = "gemini-2.5-pro"
                endpoint = "https://example.com"
        "#;
        let result = AgentConfig::from_toml_str(content);
        assert!(result.is_err());
        // Check the specific error message
        let error_string = result.err().unwrap().to_string();
        assert!(error_string.contains("Default provider 'missing_provider' not found"), "Unexpected error message: {}", error_string);
    }
    
    // Add more tests for other validation rules
}

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
    #[serde(rename = "type")]
    pub provider_type: String,
    // Removed model_name - get from embedded ModelConfig
    // pub model_name: String, 
    pub api_key_env_var: String,
    #[serde(flatten)]
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
    // Uncomment model_name
    pub model_name: String, 
    #[serde(default)]
    pub parameters: Option<toml::Value>,
    #[serde(default)]
    pub endpoint: Option<String>,
}

impl AgentConfig {
    pub fn from_toml_str(config_toml_content: &str) -> Result<AgentConfig> {
        let config: AgentConfig = toml::from_str(config_toml_content)
            .context("Failed to parse configuration TOML content. Check TOML syntax.")?;

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

        for (key, provider) in &config.providers {
            if provider.provider_type.trim().is_empty() {
                return Err(anyhow!("Provider '{}' is missing 'type'.", key));
            }
            // Validate model_name from embedded ModelConfig
            if provider.model_config.model_name.trim().is_empty() {
                 return Err(anyhow!("Provider '{}' is missing 'model_name'.", key));
            }
             if provider.api_key_env_var.trim().is_empty() {
                 return Err(anyhow!("Provider '{}' is missing 'api_key_env_var'.", key));
            }
            if let Some(endpoint) = &provider.model_config.endpoint {
                 if endpoint.trim().is_empty() {
                    return Err(anyhow!("Provider '{}' has an empty 'endpoint'.", key));
                 }
                 Url::parse(endpoint).with_context(|| {
                    format!("Invalid URL format for endpoint ('{}') in provider '{}'.", endpoint, key)
                 })?;
            }
            // Handle Option<toml::Value> for parameters
            if let Some(params) = &provider.model_config.parameters {
                 if !params.is_table() && !params.is_str() {
                     return Err(anyhow!(
                        "Provider '{}' has invalid 'parameters'. Expected a TOML table or string.",
                        key
                    ));
                 }
            }
        }
        
        for (key, server) in &config.mcp_servers {
             if server.command.trim().is_empty() {
                 return Err(anyhow!("MCP Server '{}' has an empty 'command'.", key));
            }
        }

        tracing::info!("Successfully parsed and validated agent configuration.");
        Ok(config)
    }
}

/* Old RuntimeConfig commented out */

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_mcp_config_content() -> String {
        r#"
            system_prompt = "You are Volition MCP."
            default_provider = "gemini_default"

            [providers.gemini_default]
            type = "gemini"
            model_name = "gemini-2.5-pro" # model_name now part of ModelConfig
            api_key_env_var = "GOOGLE_API_KEY"
            parameters = { temperature = 0.6 }
            
            [providers.openai_fast]
            type = "openai"
            model_name = "gpt-4o-mini" # model_name now part of ModelConfig
            api_key_env_var = "OPENAI_API_KEY"
            parameters = { temperature = 0.1 }

            [mcp_servers.filesystem]
            command = "cargo"
            args = ["run", "--bin", "volition-filesystem-server"]
            
            [mcp_servers.shell]
            command = "cargo"
            args = ["run", "--bin", "volition-shell-server"]

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
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let config = result.unwrap();
        assert_eq!(config.default_provider, "gemini_default");
        assert_eq!(config.providers.len(), 2);
        assert!(config.providers.contains_key("gemini_default"));
        // Check model_name within embedded ModelConfig
        assert_eq!(config.providers["openai_fast"].model_config.model_name, "gpt-4o-mini"); 
        assert_eq!(config.mcp_servers.len(), 2);
        assert_eq!(config.mcp_servers["filesystem"].command, "cargo");
        assert_eq!(config.strategies.len(), 1);
        assert_eq!(config.strategies["plan_execute"].planning_provider, Some("openai_fast".to_string()));
    }

     #[test]
    fn test_mcp_config_missing_default_provider_def() {
        let content = r#"
            system_prompt = "Valid"
            default_provider = "missing_provider"
            [providers.gemini_default]
            type = "gemini"
            model_name = "gemini-2.5-pro"
            api_key_env_var = "GOOGLE_API_KEY"
        "#;
        let result = AgentConfig::from_toml_str(content);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("Default provider 'missing_provider' not found"));
    }
}

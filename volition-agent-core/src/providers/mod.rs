// volition-agent-core/src/providers/mod.rs
use crate::models::chat::{ApiResponse, ChatMessage};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
// Removed unused Value import
use std::collections::HashMap;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn get_completion(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse>; 
    fn name(&self) -> &str;
}

pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
    default_provider: String,
}

impl ProviderRegistry {
    pub fn new(default_provider: String) -> Self {
        Self {
            providers: HashMap::new(),
            default_provider,
        }
    }

    pub fn register(&mut self, id: String, provider: Box<dyn Provider>) {
        self.providers.insert(id, provider);
    }

    pub fn get(&self, id: &str) -> Result<&dyn Provider> {
        self.providers
            .get(id)
            .map(|p| p.as_ref())
            .ok_or_else(|| anyhow!("Provider not found: {}", id))
    }

    pub fn default(&self) -> Result<&dyn Provider> {
        self.get(&self.default_provider)
    }

    pub fn default_provider_id(&self) -> &str {
        &self.default_provider
    }
}

pub mod gemini;
// Add ollama module
pub mod ollama; 
// pub mod openai;
// pub mod anthropic;

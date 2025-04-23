use super::*;
use crate::models::chat::ChatMessage;
use crate::models::tools::{ToolDefinition, ToolParametersDefinition};
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_empty_tool_parameters() -> ToolParametersDefinition {
        ToolParametersDefinition {
            param_type: "object".to_string(),
            properties: HashMap::new(),
            required: Vec::new(),
        }
    }

    #[test]
    fn test_gemini_provider_construction() {
        let provider = gemini::GeminiProvider::new(
            "test_key".to_string(),
            None,
            "gemini-pro".to_string(),
        );
        assert_eq!(
            provider.get_endpoint(),
            format!("{}/gemini-pro:generateContent", gemini::BASE_ENDPOINT)
        );

        let custom_endpoint = "https://custom-endpoint.com".to_string();
        let provider = gemini::GeminiProvider::new(
            "test_key".to_string(),
            Some(custom_endpoint.clone()),
            "gemini-pro".to_string(),
        );
        assert_eq!(provider.get_endpoint(), custom_endpoint);
    }

    #[test]
    fn test_openai_provider_construction() {
        // Test default endpoint
        let provider = openai::OpenAIProvider::new("test_key".to_string(), None);
        assert_eq!(provider.get_endpoint(), "https://api.openai.com/v1/chat/completions");

        // Test custom endpoint
        let custom_endpoint = "https://custom.openai.com/v1/chat/completions".to_string();
        let provider = openai::OpenAIProvider::new("test_key".to_string(), Some(custom_endpoint.clone()));
        assert_eq!(provider.get_endpoint(), custom_endpoint);
    }

    #[test]
    fn test_ollama_provider_construction() {
        // Test default endpoint
        let provider = ollama::OllamaProvider::new("http://localhost:11434/api/chat".to_string());
        assert_eq!(provider.get_endpoint(), "http://localhost:11434/api/chat");

        // Test custom endpoint
        let custom_endpoint = "http://custom:11434/api/chat".to_string();
        let provider = ollama::OllamaProvider::new(custom_endpoint.clone());
        assert_eq!(provider.get_endpoint(), custom_endpoint);
    }

    #[test]
    fn test_gemini_build_payload() {
        let provider = gemini::GeminiProvider::new(
            "test_key".to_string(),
            None,
            "gemini-pro".to_string(),
        );
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: Some("Hi there!".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let payload = provider.build_payload("gemini-pro", messages.clone(), None, None).unwrap();
        assert_eq!(payload["model"], "gemini-pro");
        assert_eq!(payload["contents"][0]["role"], "user");
        assert_eq!(payload["contents"][0]["parts"][0]["text"], "Hello");
        assert_eq!(payload["contents"][1]["role"], "assistant");
        assert_eq!(payload["contents"][1]["parts"][0]["text"], "Hi there!");

        let tools = vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: ToolParametersDefinition {
                param_type: "object".to_string(),
                properties: HashMap::new(),
                required: Vec::new(),
            },
        }];

        let payload = provider.build_payload("gemini-pro", messages, Some(&tools), None).unwrap();
        assert_eq!(payload["tools"][0]["name"], "test_tool");
        assert_eq!(payload["tools"][0]["description"], "A test tool");
    }

    #[test]
    fn test_openai_build_payload() {
        let provider = openai::OpenAIProvider::new("test_key".to_string(), None);
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        // Test basic payload
        let payload = provider.build_payload("gpt-4", messages.clone(), None, None).unwrap();
        assert_eq!(payload["model"], "gpt-4");
        assert_eq!(payload["messages"][0]["content"], "Hello");

        // Test payload with tools
        let tools = vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: "Test tool".to_string(),
            parameters: create_empty_tool_parameters(),
        }];
        let payload = provider.build_payload("gpt-4", messages, Some(&tools), None).unwrap();
        assert!(payload["tools"].is_array());
    }

    #[test]
    fn test_ollama_build_payload() {
        let provider = ollama::OllamaProvider::new("http://localhost:11434/api/chat".to_string());
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        }];

        // Test basic payload
        let payload = provider.build_payload("llama2", messages.clone(), None, None).unwrap();
        assert_eq!(payload["model"], "llama2");
        assert_eq!(payload["messages"][0]["content"], "Hello");

        // Test payload with tools
        let tools = vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: "Test tool".to_string(),
            parameters: create_empty_tool_parameters(),
        }];
        let payload = provider.build_payload("llama2", messages, Some(&tools), None).unwrap();
        assert!(payload["tools"].is_array());
    }

    #[test]
    fn test_gemini_parse_response() {
        let provider = gemini::GeminiProvider::new(
            "test_key".to_string(),
            None,
            "gemini-pro".to_string(),
        );
        let response_body = r#"
        {
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "Hello, world!"
                    }]
                },
                "finishReason": "stop"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5
            }
        }"#;

        let response = provider.parse_response(response_body).unwrap();
        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.finish_reason, "stop");
        assert_eq!(response.prompt_tokens, 10);
        assert_eq!(response.completion_tokens, 5);
        assert_eq!(response.total_tokens, 15);
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.content, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_openai_parse_response() {
        let provider = openai::OpenAIProvider::new("test_key".to_string(), None);
        
        // Test successful response
        let response = r#"{
            "id": "test_id",
            "choices": [{
                "message": {
                    "content": "Hello",
                    "role": "assistant"
                }
            }]
        }"#;
        let parsed = provider.parse_response(response).unwrap();
        assert_eq!(parsed.choices[0].message.content, Some("Hello".to_string()));

        // Test error response
        let error_response = r#"{
            "error": {
                "message": "Invalid API key"
            }
        }"#;
        assert!(provider.parse_response(error_response).is_err());
    }

    #[test]
    fn test_ollama_parse_response() {
        let provider = ollama::OllamaProvider::new("http://localhost:11434/api/chat".to_string());
        
        // Test successful response
        let response = r#"{
            "message": {
                "content": "Hello",
                "role": "assistant"
            }
        }"#;
        let parsed = provider.parse_response(response).unwrap();
        assert_eq!(parsed.choices[0].message.content, Some("Hello".to_string()));

        // Test error response
        let error_response = r#"{
            "error": "Invalid API key"
        }"#;
        assert!(provider.parse_response(error_response).is_err());
    }

    #[test]
    fn test_gemini_build_headers() {
        let provider = gemini::GeminiProvider::new(
            "test_key".to_string(),
            None,
            "gemini-pro".to_string(),
        );
        let headers = provider.build_headers().unwrap();
        assert_eq!(headers["Content-Type"], "application/json");
        assert_eq!(headers["x-goog-api-key"], "test_key");

        let provider = gemini::GeminiProvider::new(
            "".to_string(),
            None,
            "gemini-pro".to_string(),
        );
        let headers = provider.build_headers().unwrap();
        assert_eq!(headers["Content-Type"], "application/json");
        assert!(!headers.contains_key("x-goog-api-key"));
    }

    #[test]
    fn test_openai_build_headers() {
        let provider = openai::OpenAIProvider::new("test_key".to_string(), None);
        let headers = provider.build_headers().unwrap();
        
        assert_eq!(headers["Content-Type"], "application/json");
        assert_eq!(headers["Authorization"], "Bearer test_key");

        // Test without API key
        let provider = openai::OpenAIProvider::new("".to_string(), None);
        let headers = provider.build_headers().unwrap();
        assert!(!headers.contains_key("Authorization"));
    }

    #[test]
    fn test_ollama_build_headers() {
        let provider = ollama::OllamaProvider::new("http://localhost:11434/api/chat".to_string());
        let headers = provider.build_headers().unwrap();
        
        assert_eq!(headers["Content-Type"], "application/json");
        // Ollama doesn't use API key in headers
        assert!(!headers.contains_key("Authorization"));
    }
} 
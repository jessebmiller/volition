// volition-agent-core/src/api.rs

//! Handles interactions with external AI model APIs.

// Corrected Imports:
use crate::models::chat::{ApiResponse, ChatMessage, Choice}; // Use Choice, remove ToolCall, Function*, etc.
use crate::models::tools::{ToolCall, ToolDefinition, ToolFunction}; // Import necessary tool structs
use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, Method, Url}; // Removed RequestBuilder
use serde_json::{json, Map, Value};
use tracing::{error, trace, warn}; // Removed debug
use std::time::{SystemTime, UNIX_EPOCH}; // For generating IDs

/// Helper function to format headers for logging, excluding Authorization.
fn format_headers_for_log(headers: &header::HeaderMap) -> String {
    // ... (keep existing implementation)
    let mut formatted = String::from("{");
    for (name, value) in headers.iter() {
        if name != header::AUTHORIZATION {
            if formatted.len() > 1 {
                formatted.push_str(", ");
            }
            formatted.push_str(&format!(
                "\"{}\": \"{}\"",
                name.as_str(),
                value.to_str().unwrap_or("<invalid header value>")
            ));
        }
    }
    formatted.push('}');
    formatted
}

/// Maps our internal ChatMessage role to the Gemini API role.
fn map_role_to_gemini(role: &str) -> Option<&str> {
    match role {
        "user" => Some("user"),
        "assistant" => Some("model"),
        "tool" => Some("function"),
        "system" => None,
        _ => {
            warn!(role = %role, "Unknown role encountered for Gemini mapping, skipping message.");
            None
        }
    }
}

/// Generates a relatively unique ID string using nanoseconds.
fn generate_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}_{}", prefix, nanos)
}

/// Generic function to make a request to an AI chat completion API.
pub async fn call_chat_completion_api(
    http_client: &Client,
    endpoint_str: &str,
    api_key: &str,
    model_name: &str,
    messages: Vec<ChatMessage>,
    tools: Option<&[ToolDefinition]>,
    parameters: Option<&toml::Value>,
) -> Result<ApiResponse> {
    trace!(endpoint = %endpoint_str, model = %model_name, num_messages = messages.len(), "Entering call_chat_completion_api");

    let mut endpoint = Url::parse(endpoint_str)
        .with_context(|| format!("Failed to parse endpoint URL: {}", endpoint_str))?;

    // Clippy fix: Use is_some_and
    let is_google_api = endpoint.host_str().is_some_and(|h| h.contains("googleapis.com"));

    // --- Authentication Handling ---
    let mut use_query_param_key = false;
    if is_google_api {
        if api_key.is_empty() {
             warn!("API key is empty for Google API endpoint. Call will likely fail.");
        } else {
            trace!("API key is present (length: {}).", api_key.len());
            endpoint.query_pairs_mut().append_pair("key", api_key);
            use_query_param_key = true;
            trace!(endpoint = %endpoint.as_str(), "Added API key as query parameter for Google API.");
        }
    } else if api_key.is_empty() {
         warn!("API key is empty. API call might fail if endpoint requires authentication.");
    } else {
         trace!("API key is present (length: {}).", api_key.len());
    }

    // --- Payload Construction ---
    let payload: Value;

    if is_google_api {
        trace!("Constructing payload for Google Gemini API.");
        let mut gemini_payload = Map::new();
        let mut gemini_contents = Vec::new();
        let mut system_instruction_parts = Vec::new();

        for message in messages {
            match message.role.as_str() {
                "system" => {
                    if let Some(content) = message.content {
                         system_instruction_parts.push(json!({ "text": content }));
                         trace!("Extracted system instruction.");
                    }
                 },
                 "tool" => {
                    if let Some(role) = map_role_to_gemini(&message.role) {
                        if let Some(tool_call_id) = message.tool_call_id {
                             let response_content = message.content.unwrap_or_else(|| {
                                warn!(tool_call_id=%tool_call_id, "Tool response message has no content, sending empty string.");
                                "".to_string() // Send empty string content if tool output is None
                             });
                             // Try to parse as JSON, otherwise treat as plain string.
                             let response_json: Value = serde_json::from_str(&response_content).unwrap_or_else(|_| json!(response_content));
                             // *** FIX: Wrap the response_json in the required structure {"content": ...} ***
                             let gemini_response_object = json!({ "content": response_json });
                             gemini_contents.push(json!({
                                 "role": role,
                                 "parts": [{
                                     "functionResponse": {
                                         "name": tool_call_id,
                                         "response": gemini_response_object // Use the wrapped object
                                     }
                                 }]
                             }));
                             trace!(role=role, tool_call_id=%tool_call_id, "Added tool response to contents.");
                        } else {
                             warn!(role=message.role, "Tool message missing tool_call_id, skipping.");
                        }
                    }
                 }
                _ => { // user, assistant
                    if let Some(role) = map_role_to_gemini(&message.role) {
                        let mut parts = Vec::new();
                        if let Some(content) = message.content {
                            parts.push(json!({ "text": content }));
                        }
                        if let Some(tool_calls) = message.tool_calls {
                             for tool_call in tool_calls {
                                 // *** Fix: Parse arguments string to Value for Gemini ***
                                 let args_value: Value = match serde_json::from_str(&tool_call.function.arguments) {
                                      Ok(val) => val,
                                      Err(e) => {
                                          error!(error=%e, args_str=%tool_call.function.arguments, tool_name=%tool_call.function.name, "Failed to parse tool arguments string to JSON Value for Gemini payload. Skipping tool call.");
                                          // Skip this tool call part if args are invalid
                                          continue;
                                      }
                                 };
                                 parts.push(json!({
                                     "functionCall": {
                                         "name": tool_call.function.name,
                                         "args": args_value // Use parsed Value
                                     }
                                 }));
                             }
                             trace!(role=role, num_tool_calls=parts.len(), "Added tool calls to parts.");
                        }

                        if !parts.is_empty() {
                            gemini_contents.push(json!({ "role": role, "parts": parts }));
                             trace!(role=role, num_parts=parts.len(), "Added message to contents.");
                        } else {
                             warn!(role=role, "Message has no content or tool calls, skipping.");
                        }
                    }
                }
            }
        }
        gemini_payload.insert("contents".to_string(), json!(gemini_contents));

        if !system_instruction_parts.is_empty() {
            gemini_payload.insert("systemInstruction".to_string(), json!({ "parts": system_instruction_parts }));
             trace!("Added system instruction to payload.");
        }

        if let Some(tools) = tools {
            if !tools.is_empty() {
                 let function_declarations: Vec<Value> = tools.iter().map(|t| json!({
                     "name": t.name,
                     "description": t.description,
                     // *** Fix: Use 'parameters' field name ***
                     "parameters": t.parameters
                 })).collect();
                 gemini_payload.insert("tools".to_string(), json!([{ "functionDeclarations": function_declarations }]));
                 trace!(num_tools = tools.len(), "Added tools (functionDeclarations) to payload.");
            }
        }

        if let Some(params_value) = parameters {
             trace!("Processing model parameters for Gemini...");
             if let Some(params_table) = params_value.as_table() {
                 let mut generation_config = Map::new();
                 for (key, value) in params_table {
                     trace!(key = %key, value = ?value, "Converting TOML parameter for generationConfig");
                     let json_value: Value = match value.clone().try_into() {
                         Ok(v) => v,
                         Err(e) => {
                             error!(key=%key, value=?value, error=%e, "Failed to convert TOML parameter to JSON for generationConfig");
                             return Err(anyhow!(e)).context(format!("Failed to convert TOML parameter '{}' to JSON", key));
                         }
                     };
                     match key.as_str() {
                         "temperature" | "topP" | "topK" | "candidateCount" | "maxOutputTokens" | "stopSequences" => {
                            generation_config.insert(key.clone(), json_value);
                            trace!(key = %key, "Added parameter to generationConfig.");
                         },
                         _ => warn!(key = %key, "Unsupported parameter for Gemini generationConfig, skipping.")
                     }
                 }
                 if !generation_config.is_empty() {
                    gemini_payload.insert("generationConfig".to_string(), json!(generation_config));
                    trace!("Added generationConfig to payload.");
                 }
             } else {
                 trace!("Model parameters are not a table, skipping generationConfig.");
             }
        }

        payload = json!(gemini_payload);
        trace!("Final Gemini payload constructed.");

    } else { // OpenAI-compatible path
        trace!("Constructing payload for OpenAI-compatible API.");
        let mut openai_payload_map = Map::new();
        openai_payload_map.insert("model".to_string(), json!(model_name));
        openai_payload_map.insert("messages".to_string(), json!(messages));

        if let Some(tools) = tools {
            if !tools.is_empty() {
                openai_payload_map.insert("tools".to_string(), json!(tools));
                trace!(num_tools = tools.len(), "Added tools to OpenAI payload.");
            }
        }

        if let Some(params_value) = parameters {
            trace!("Processing model parameters for OpenAI...");
            if let Some(params_table) = params_value.as_table() {
                 for (key, value) in params_table {
                     trace!(key = %key, value = ?value, "Converting TOML parameter for OpenAI");
                     let json_value: Value = match value.clone().try_into() {
                          Ok(v) => v,
                          Err(e) => {
                               error!(key=%key, value=?value, error=%e, "Failed to convert TOML parameter to JSON");
                               return Err(anyhow!(e)).context(format!("Failed to convert TOML parameter '{}' to JSON", key));
                          }
                     };
                     openai_payload_map.insert(key.clone(), json_value);
                     trace!(key = %key, "Added parameter to OpenAI payload.");
                 }
            } else {
                 trace!("Model parameters are not a table, skipping merge.");
            }
        }
        payload = json!(openai_payload_map);
        trace!("Final OpenAI payload constructed.");
    }

    // --- Request Sending and Response Handling ---

    let payload_string = match serde_json::to_string_pretty(&payload) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to serialize payload before sending");
            return Err(anyhow!(e)).context("Failed to serialize payload");
        }
    };
    trace!(endpoint = %endpoint.as_str(), payload_len = payload_string.len(), "Prepared request payload (see full payload in next log if TRACE enabled)");
    if tracing::enabled!(tracing::Level::TRACE) {
        trace!(payload = %payload_string, "Full request payload");
    }

    trace!("Building request object...");
    let mut request_builder = http_client
        .request(Method::POST, endpoint.clone())
        .header(header::CONTENT_TYPE, "application/json");

    if !use_query_param_key && !api_key.is_empty() {
        trace!("Adding Bearer authentication header.");
        request_builder = request_builder.bearer_auth(api_key);
    }

    let request = match request_builder.json(&payload).build() {
        Ok(req) => {
            trace!("Request object built successfully.");
            req
        },
        Err(e) => {
            error!(error = %e, "Failed to build request object");
            return Err(anyhow!(e)).context("Failed to build request object");
        }
    };

    let request_details = format!(
        "Endpoint: {}\nMethod: {}\nHeaders: {}\n",
        request.url(),
        request.method(),
        format_headers_for_log(request.headers()),
    );
    trace!(%request_details, "Sending built API request");

    trace!("Executing HTTP request...");
    let response = match http_client.execute(request).await {
        Ok(resp) => {
            trace!("HTTP request executed successfully, received initial response.");
            resp
        },
        Err(e) => {
            error!(error = %e, endpoint = %endpoint.as_str(), "Failed to send request or receive response headers");
            return Err(anyhow!(e)).context(format!("HTTP request execution failed for endpoint: {}", endpoint.as_str()));
        }
    };

    let status = response.status();
    trace!(%status, "Received response status.");
    trace!("Reading response body...");
    let response_text = match response.text().await {
        Ok(text) => {
            trace!(len = text.len(), "Response body read successfully.");
            text
        },
        Err(e) => {
            error!(status = %status, error = %e, "Failed to read API response text");
            return Err(anyhow!(e)).context("Failed to read API response text");
        }
    };

     if tracing::enabled!(tracing::Level::TRACE) {
        trace!(status = %status, response_body = %response_text, "Full received API response");
     }

    if !status.is_success() {
        error!(status = %status, response_body = %response_text, "API request failed");
        return Err(anyhow!(
            "API request failed with status {}. Endpoint: {}. Response: {}\nCheck API key, endpoint, model name, and request payload.",
            status,
            endpoint.as_str(),
            response_text
        ));
    }

    // --- Response Parsing ---
    trace!("Attempting to parse successful API response JSON...");

    if is_google_api {
        // *** Fix: Parse Gemini response into ApiResponse { id, choices: [Choice { index, message, finish_reason }] } ***
        trace!("Parsing response for Google Gemini API.");
        match serde_json::from_str::<Value>(&response_text) {
             Ok(raw_response) => {
                 trace!(?raw_response, "Successfully parsed Gemini response into raw JSON Value.");
                 let mut choices = Vec::new();
                 let response_id = generate_id("gemini_resp"); // Generate an ID

                 if let Some(candidates) = raw_response.get("candidates").and_then(|c| c.as_array()) {
                     for (index, candidate) in candidates.iter().enumerate() { // Iterate over candidates if needed
                         if index > 0 {
                             warn!("Handling only the first candidate from Gemini response.");
                             break; // Only handle the first candidate for now
                         }

                         let finish_reason = candidate.get("finishReason")
                             .and_then(|fr| fr.as_str())
                             .unwrap_or("unknown") // Default finish reason
                             .to_string();

                         if let Some(content) = candidate.get("content") {
                             if let Some(role) = content.get("role").and_then(|r| r.as_str()) {
                                 if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                     let mut combined_text: Option<String> = None;
                                     let mut tool_calls: Option<Vec<ToolCall>> = None;

                                     let mut current_text = String::new();
                                     let mut current_tool_calls = Vec::new();

                                     for part in parts {
                                         if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                             current_text.push_str(text);
                                         } else if let Some(fc) = part.get("functionCall") {
                                              if let (Some(name), Some(args_value)) = (
                                                  fc.get("name").and_then(|n| n.as_str()),
                                                  fc.get("args") // args is a Value
                                              ) {
                                                  // *** Fix: Convert args Value back to String ***
                                                  let args_string = match serde_json::to_string(args_value) {
                                                      Ok(s) => s,
                                                      Err(e) => {
                                                          error!(error=%e, args_value=?args_value, tool_name=%name, "Failed to serialize Gemini function call args back to string. Skipping tool call.");
                                                          continue; // Skip this tool call
                                                      }
                                                  };

                                                  current_tool_calls.push(ToolCall {
                                                      id: generate_id(&format!("call_{}", name)), // Generate call ID
                                                      call_type: "function".to_string(),
                                                      function: ToolFunction {
                                                          name: name.to_string(),
                                                          arguments: args_string, // Use stringified args
                                                      },
                                                  });
                                              }
                                         }
                                     } // end for part in parts

                                     if !current_text.is_empty() {
                                         combined_text = Some(current_text);
                                     }
                                     if !current_tool_calls.is_empty() {
                                         tool_calls = Some(current_tool_calls);
                                     }

                                     let message_role = match role {
                                          "model" => "assistant".to_string(),
                                          _ => {
                                               warn!(gemini_role=%role, "Unexpected role from Gemini model content, using directly.");
                                               role.to_string()
                                          }
                                     };

                                     let message = ChatMessage {
                                         role: message_role,
                                         content: combined_text,
                                         // Clippy fix: Use field init shorthand
                                         tool_calls,
                                         tool_call_id: None,
                                     };

                                     choices.push(Choice {
                                         index: index as u32,
                                         message,
                                         finish_reason: finish_reason.clone(), // Use reason from candidate
                                     });
                                     trace!(choice_index=index, "Added choice from Gemini candidate.");

                                 } else {
                                      warn!(candidate_index=index, "Gemini candidate content has no 'parts'.");
                                 }
                             } else {
                                 warn!(candidate_index=index, "Gemini candidate content has no 'role'.");
                             }
                         } else {
                              warn!(candidate_index=index, "Gemini candidate has no 'content'.");
                         }
                     } // end for candidate in candidates
                 } else {
                     warn!("Gemini response has no 'candidates' array.");
                 }

                 if choices.is_empty() {
                     warn!("Could not extract any valid choices from Gemini response structure. Raw: {}", response_text);
                     Err(anyhow!("Failed to extract choices from Gemini response structure: {}", response_text))
                 } else {
                     Ok(ApiResponse { id: response_id, choices }) // *** Fix: Return ApiResponse ***
                 }
             },
             Err(e) => {
                 error!(status = %status, response_body = %response_text, error = %e, "Failed to parse successful Gemini API response JSON into Value");
                 Err(anyhow!(e)).with_context(|| format!("Failed to parse successful Gemini API response JSON: {}", response_text))
             }
        }

    } else { // OpenAI-compatible path
        trace!("Parsing response for OpenAI-compatible API.");
        match serde_json::from_str::<ApiResponse>(&response_text) {
            Ok(api_response) => {
                trace!("Successfully parsed OpenAI-compatible API response.");
                Ok(api_response)
            },
            Err(e) => {
                error!(status = %status, response_body = %response_text, error = %e, "Failed to parse successful OpenAI-compatible API response JSON");
                Err(anyhow!(e)).with_context(|| format!("Failed to parse successful OpenAI-compatible API response JSON: {}", response_text))
            }
        }
    }
}

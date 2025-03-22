use reqwest::Client;
use serde_json::from_str;
use crate::models::tools::ToolCall;
use crate::models::chat::ResponseMessage;

pub async fn handle_tool_calls(
    _client: &Client, //TODO remove unused argument
    _api_key: &str,   //TODO remove unused argument
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>,
) -> Result<(), anyhow::Error> {
    for tool_call in tool_calls {
        match tool_call.function.name.as_str() {
            "submit_quality_score" => {
                let args = from_str(&tool_call.function.arguments)?;
                let output = crate::tools::submit_quality_score(args).await?;
                messages.push(ResponseMessage {
                    role: "assistant".to_string(),
                    content: Some(format!("Quality score submitted successfully.")),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
            _ => {
                messages.push(ResponseMessage {
                    role: "assistant".to_string(),
                    content: Some(format!("Unknown tool call.")),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        }
    }
    Ok(())
}

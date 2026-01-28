use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::ipc::Channel;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeRateLimits {
    pub requests_limit: Option<u32>,
    pub requests_remaining: Option<u32>,
    pub requests_reset: Option<String>,
    pub tokens_limit: Option<u32>,
    pub tokens_remaining: Option<u32>,
    pub tokens_reset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeStreamEvent {
    pub event_type: String,
    pub content: Option<String>,
    pub usage: Option<ClaudeUsage>,
    pub rate_limits: Option<ClaudeRateLimits>,
    pub error: Option<String>,
}

fn parse_rate_limits(headers: &reqwest::header::HeaderMap) -> ClaudeRateLimits {
    let get_header = |name: &str| -> Option<String> {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };

    let parse_u32 = |name: &str| -> Option<u32> {
        get_header(name).and_then(|s| s.parse().ok())
    };

    ClaudeRateLimits {
        requests_limit: parse_u32("anthropic-ratelimit-requests-limit"),
        requests_remaining: parse_u32("anthropic-ratelimit-requests-remaining"),
        requests_reset: get_header("anthropic-ratelimit-requests-reset"),
        tokens_limit: parse_u32("anthropic-ratelimit-tokens-limit"),
        tokens_remaining: parse_u32("anthropic-ratelimit-tokens-remaining"),
        tokens_reset: get_header("anthropic-ratelimit-tokens-reset"),
    }
}

#[tauri::command]
pub async fn send_claude_message(
    api_key: String,
    model: String,
    messages: Vec<ClaudeMessage>,
    max_tokens: Option<u32>,
    on_event: Channel<ClaudeStreamEvent>,
) -> Result<(), String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("API key is required".to_string());
    }

    let client = Client::new();
    let max_tokens = max_tokens.unwrap_or(4096);

    // Convert messages to API format
    let api_messages: Vec<Value> = messages
        .iter()
        .map(|m| {
            json!({
                "role": m.role,
                "content": m.content
            })
        })
        .collect();

    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": api_messages,
        "stream": true
    });

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let rate_limits = parse_rate_limits(response.headers());

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_body = response.text().await.unwrap_or_default();
        let error_msg = format!("Claude API error {}: {}", status, error_body);
        let _ = on_event.send(ClaudeStreamEvent {
            event_type: "error".to_string(),
            content: None,
            usage: None,
            rate_limits: Some(rate_limits),
            error: Some(error_msg.clone()),
        });
        return Err(error_msg);
    }

    // Send rate limits immediately
    let _ = on_event.send(ClaudeStreamEvent {
        event_type: "rate_limits".to_string(),
        content: None,
        usage: None,
        rate_limits: Some(rate_limits),
        error: None,
    });

    // Process SSE stream
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut accumulated_text = String::new();
    let mut final_usage: Option<ClaudeUsage> = None;

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        let chunk_str = String::from_utf8_lossy(&chunk);
        buffer.push_str(&chunk_str);

        // Process complete SSE events
        while let Some(event_end) = buffer.find("\n\n") {
            let event_data = buffer[..event_end].to_string();
            buffer = buffer[event_end + 2..].to_string();

            // Parse SSE event
            for line in event_data.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                        let event_type = parsed
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        match event_type {
                            "content_block_delta" => {
                                if let Some(delta) = parsed.get("delta") {
                                    if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                        accumulated_text.push_str(text);
                                        let _ = on_event.send(ClaudeStreamEvent {
                                            event_type: "content_delta".to_string(),
                                            content: Some(text.to_string()),
                                            usage: None,
                                            rate_limits: None,
                                            error: None,
                                        });
                                    }
                                }
                            }
                            "message_delta" => {
                                if let Some(usage) = parsed.get("usage") {
                                    if let Ok(u) = serde_json::from_value::<ClaudeUsage>(usage.clone()) {
                                        final_usage = Some(u);
                                    }
                                }
                            }
                            "message_stop" => {
                                let _ = on_event.send(ClaudeStreamEvent {
                                    event_type: "message_complete".to_string(),
                                    content: Some(accumulated_text.clone()),
                                    usage: final_usage.clone(),
                                    rate_limits: None,
                                    error: None,
                                });
                            }
                            "error" => {
                                let error_msg = parsed
                                    .get("error")
                                    .and_then(|e| e.get("message"))
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("Unknown error")
                                    .to_string();
                                let _ = on_event.send(ClaudeStreamEvent {
                                    event_type: "error".to_string(),
                                    content: None,
                                    usage: None,
                                    rate_limits: None,
                                    error: Some(error_msg.clone()),
                                });
                                return Err(error_msg);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeNonStreamResponse {
    pub content: String,
    pub usage: ClaudeUsage,
    pub rate_limits: ClaudeRateLimits,
}

/// Non-streaming version for simple requests
#[tauri::command]
pub async fn send_claude_message_sync(
    api_key: String,
    model: String,
    messages: Vec<ClaudeMessage>,
    max_tokens: Option<u32>,
) -> Result<ClaudeNonStreamResponse, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("API key is required".to_string());
    }

    let client = Client::new();
    let max_tokens = max_tokens.unwrap_or(4096);

    let api_messages: Vec<Value> = messages
        .iter()
        .map(|m| {
            json!({
                "role": m.role,
                "content": m.content
            })
        })
        .collect();

    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": api_messages
    });

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let rate_limits = parse_rate_limits(response.headers());

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!("Claude API error {}: {}", status, error_body));
    }

    let parsed: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let content = parsed
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    let usage = parsed
        .get("usage")
        .and_then(|u| serde_json::from_value::<ClaudeUsage>(u.clone()).ok())
        .unwrap_or(ClaudeUsage {
            input_tokens: 0,
            output_tokens: 0,
        });

    Ok(ClaudeNonStreamResponse {
        content,
        usage,
        rate_limits,
    })
}

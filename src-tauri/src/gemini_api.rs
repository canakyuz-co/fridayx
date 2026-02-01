use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiNonStreamResponse {
    pub content: String,
}

#[tauri::command]
pub async fn send_gemini_message_sync(
    api_key: String,
    model: String,
    prompt: String,
) -> Result<GeminiNonStreamResponse, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("API key is required".to_string());
    }
    let model = model.trim();
    if model.is_empty() {
        return Err("Model is required".to_string());
    }

    let client = Client::new();
    let url = format!("{GEMINI_API_BASE}/models/{model}:generateContent");
    let body = json!({
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": prompt }]
            }
        ],
        "generationConfig": {
            "temperature": 0.2
        }
    });

    let response = client
        .post(url)
        .query(&[("key", api_key)])
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Gemini API request failed: {err}"))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!("Gemini API error {}: {}", status, error_body));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|err| format!("Gemini API response invalid: {err}"))?;

    let content = payload
        .get("candidates")
        .and_then(|value| value.as_array())
        .and_then(|arr| arr.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(|parts| parts.as_array())
        .and_then(|parts| parts.first())
        .and_then(|part| part.get("text"))
        .and_then(|text| text.as_str())
        .unwrap_or("")
        .to_string();

    Ok(GeminiNonStreamResponse { content })
}

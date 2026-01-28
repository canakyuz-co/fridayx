use reqwest::Client;
use serde_json::Value;

fn read_env(key: &str) -> Result<String, String> {
    std::env::var(key).map_err(|_| format!("{key} is not set"))
}

fn collect_unique_models(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut models = Vec::new();
    for item in items {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            models.push(trimmed.to_string());
        }
    }
    models
}

async fn list_claude_models(client: &Client) -> Result<Vec<String>, String> {
    let api_key = read_env("ANTHROPIC_API_KEY")?;
    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|err| format!("Claude API request failed: {err}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Claude API error: {}",
            response.status().as_u16()
        ));
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|err| format!("Claude API response invalid: {err}"))?;
    let models = payload
        .get("data")
        .and_then(|data| data.as_array())
        .map(|data| {
            data.iter()
                .filter_map(|item| item.get("id").and_then(|value| value.as_str()))
                .map(|value| value.to_string())
                .filter(|value| value.starts_with("claude-"))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    Ok(collect_unique_models(models))
}

async fn list_gemini_models(client: &Client) -> Result<Vec<String>, String> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .map_err(|_| "GEMINI_API_KEY or GOOGLE_API_KEY is not set".to_string())?;
    let response = client
        .get("https://generativelanguage.googleapis.com/v1beta/models")
        .query(&[("key", api_key)])
        .send()
        .await
        .map_err(|err| format!("Gemini API request failed: {err}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Gemini API error: {}",
            response.status().as_u16()
        ));
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|err| format!("Gemini API response invalid: {err}"))?;
    let models = payload
        .get("models")
        .and_then(|data| data.as_array())
        .map(|data| {
            data.iter()
                .filter_map(|item| item.get("name").and_then(|value| value.as_str()))
                .map(|value| value.strip_prefix("models/").unwrap_or(value).to_string())
                .filter(|value| value.starts_with("gemini-"))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    Ok(collect_unique_models(models))
}

#[tauri::command]
pub(crate) async fn list_other_ai_models(provider: String) -> Result<Vec<String>, String> {
    let normalized = provider.trim().to_lowercase();
    let client = Client::new();
    match normalized.as_str() {
        "claude" => list_claude_models(&client).await,
        "gemini" => list_gemini_models(&client).await,
        _ => Err("Unsupported provider".to_string()),
    }
}

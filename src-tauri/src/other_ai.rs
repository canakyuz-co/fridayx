use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

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

async fn list_claude_models(client: &Client, api_key: &str) -> Result<Vec<String>, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("API key is required".to_string());
    }
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

async fn list_gemini_models(client: &Client, api_key: &str) -> Result<Vec<String>, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("API key is required".to_string());
    }
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

fn extract_model_name(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(name) = value.get("name").and_then(|value| value.as_str()) {
        return Some(name.to_string());
    }
    if let Some(id) = value.get("id").and_then(|value| value.as_str()) {
        return Some(id.to_string());
    }
    if let Some(model) = value.get("model").and_then(|value| value.as_str()) {
        return Some(model.to_string());
    }
    if let Some(model_id) = value.get("modelId").and_then(|value| value.as_str()) {
        return Some(model_id.to_string());
    }
    None
}

fn collect_models_from_json(provider: &str, payload: &Value) -> Vec<String> {
    let mut models = Vec::new();
    let candidates = payload
        .get("models")
        .and_then(|value| value.as_array())
        .or_else(|| payload.get("data").and_then(|value| value.as_array()))
        .or_else(|| payload.as_array());
    if let Some(items) = candidates {
        for item in items {
            if let Some(name) = extract_model_name(item) {
                models.push(name);
            }
        }
    }
    let prefix = if provider == "claude" { "claude-" } else { "gemini-" };
    models
        .into_iter()
        .map(|value| value.strip_prefix("models/").unwrap_or(value.as_str()).to_string())
        .filter(|value| value.starts_with(prefix))
        .collect()
}

fn collect_models_from_text(provider: &str, output: &str) -> Vec<String> {
    let prefix = if provider == "claude" { "claude-" } else { "gemini-" };
    let mut models = Vec::new();
    let mut token = String::new();
    for line in output.lines() {
        let trimmed = line.trim().trim_start_matches("- ").trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with(prefix) {
            models.push(trimmed.to_string());
            continue;
        }
        if let Some(token) = trimmed
            .split_whitespace()
            .find(|token| token.starts_with(prefix))
        {
            models.push(token.to_string());
        }

        // Also extract provider-prefixed IDs embedded in JSON/structured text.
        token.clear();
        for ch in trimmed.chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                token.push(ch);
            } else if !token.is_empty() {
                if token.starts_with(prefix) {
                    models.push(token.clone());
                }
                token.clear();
            }
        }
        if !token.is_empty() && token.starts_with(prefix) {
            models.push(token.clone());
        }
    }
    models
}

fn list_claude_models_via_prompt(
    command: &str,
    env: &Option<HashMap<String, String>>,
) -> Result<Vec<String>, String> {
    let cli_version = detect_claude_cli_version(command, env).ok();
    let prompt = "List the available Claude model IDs for this account. \
Return ONLY a JSON array of model IDs.";
    let args = [
        "-p",
        "--output-format",
        "json",
        "--input-format",
        "text",
        prompt,
    ];
    let mut cli_error: Option<String> = None;
    let prompt_models = match run_cli_with_env(command, &args, env) {
        Ok(stdout) => {
            let parsed = serde_json::from_str::<Value>(&stdout).ok();
            if let Some(payload) = parsed {
                collect_models_from_json("claude", &payload)
            } else {
                collect_models_from_text("claude", &stdout)
            }
        }
        Err(error) => {
            cli_error = Some(error);
            Vec::new()
        }
    };
    let mut merged = Vec::new();
    merged.extend(prompt_models);
    merged.extend(list_claude_models_from_local_cache());
    merged.extend(known_claude_models_for_version(cli_version.as_ref()));
    merged = collect_unique_models(merged);
    if merged.is_empty() {
        if let Some(version) = cli_version {
            if is_claude_cli_outdated_for_latest_models(&version) {
                return Err(format!(
                    "Claude CLI {} is outdated for latest model discovery. Run `claude update`.",
                    format_claude_cli_version(&version)
                ));
            }
        }
        return Err(cli_error.unwrap_or_else(|| "Claude CLI returned no models.".to_string()));
    }
    Ok(merged)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ClaudeCliVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

fn parse_claude_cli_version(raw: &str) -> Option<ClaudeCliVersion> {
    let mut digits = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            digits.push(ch);
        } else if !digits.is_empty() {
            break;
        }
    }
    let mut parts = digits.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some(ClaudeCliVersion {
        major,
        minor,
        patch,
    })
}

fn detect_claude_cli_version(
    command: &str,
    env: &Option<HashMap<String, String>>,
) -> Result<ClaudeCliVersion, String> {
    let output = run_cli_with_env(command, &["--version"], env)?;
    parse_claude_cli_version(&output)
        .ok_or_else(|| format!("Unable to parse Claude CLI version from: {}", output.trim()))
}

fn format_claude_cli_version(version: &ClaudeCliVersion) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}

fn compare_claude_cli_versions(
    left: &ClaudeCliVersion,
    right: &ClaudeCliVersion,
) -> std::cmp::Ordering {
    left.major
        .cmp(&right.major)
        .then(left.minor.cmp(&right.minor))
        .then(left.patch.cmp(&right.patch))
}

fn is_claude_cli_outdated_for_latest_models(version: &ClaudeCliVersion) -> bool {
    compare_claude_cli_versions(
        version,
        &ClaudeCliVersion {
            major: 2,
            minor: 1,
            patch: 34,
        },
    ) == std::cmp::Ordering::Less
}

fn known_claude_models_for_version(version: Option<&ClaudeCliVersion>) -> Vec<String> {
    let mut models = vec![
        "claude-sonnet-4-5".to_string(),
        "claude-opus-4-5".to_string(),
        "claude-haiku-4-5".to_string(),
    ];
    if let Some(cli_version) = version {
        if !is_claude_cli_outdated_for_latest_models(cli_version) {
            models.extend([
                "claude-sonnet-4-6".to_string(),
                "claude-opus-4-6".to_string(),
                "claude-haiku-4-6".to_string(),
            ]);
        }
    }
    models
}

#[cfg(test)]
mod tests {
    use super::{
        known_claude_models_for_version, parse_claude_cli_version, ClaudeCliVersion,
    };

    #[test]
    fn parse_claude_cli_version_reads_semver_prefix() {
        let parsed = parse_claude_cli_version("2.1.34 (Claude Code)");
        assert_eq!(
            parsed,
            Some(ClaudeCliVersion {
                major: 2,
                minor: 1,
                patch: 34
            })
        );
    }

    #[test]
    fn known_models_include_46_on_new_cli() {
        let version = ClaudeCliVersion {
            major: 2,
            minor: 1,
            patch: 34,
        };
        let models = known_claude_models_for_version(Some(&version));
        assert!(models.iter().any(|model| model == "claude-sonnet-4-6"));
    }
}

fn list_claude_models_from_local_cache() -> Vec<String> {
    let home = match std::env::var_os("HOME") {
        Some(value) if !value.is_empty() => value,
        _ => return Vec::new(),
    };
    let stats_path = std::path::PathBuf::from(home).join(".claude/stats-cache.json");
    let raw = match fs::read_to_string(stats_path) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let payload = match serde_json::from_str::<Value>(&raw) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };

    let mut models = Vec::new();
    collect_claude_model_ids_recursive(&payload, &mut models);
    collect_unique_models(models)
}

fn collect_claude_model_ids_recursive(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.starts_with("claude-") {
                output.push(trimmed.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_claude_model_ids_recursive(item, output);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                if key.starts_with("claude-") {
                    output.push(key.to_string());
                }
                collect_claude_model_ids_recursive(item, output);
            }
        }
        _ => {}
    }
}

fn run_cli_with_env(
    command: &str,
    args: &[&str],
    env: &Option<HashMap<String, String>>,
) -> Result<String, String> {
    let mut cmd = Command::new(command);
    cmd.args(args);
    let mut has_path_override = false;
    if let Some(env_map) = env {
        has_path_override = env_map.contains_key("PATH");
        for (key, value) in env_map {
            cmd.env(key, value);
        }
    }
    if !has_path_override {
        cmd.env("PATH", crate::utils::tools_env_path());
    }
    let output = cmd.output().map_err(|err| format!("CLI spawn failed: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "CLI exited with code {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn normalize_claude_cli_model(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = trimmed.to_ascii_lowercase();
    if normalized.starts_with("claude-sonnet-")
        && normalized.chars().filter(|ch| *ch == '-').count() == 3
    {
        return "sonnet".to_string();
    }
    if normalized.starts_with("claude-opus-")
        && normalized.chars().filter(|ch| *ch == '-').count() == 3
    {
        return "opus".to_string();
    }
    if normalized.starts_with("claude-haiku-")
        && normalized.chars().filter(|ch| *ch == '-').count() == 3
    {
        return "haiku".to_string();
    }
    trimmed.to_string()
}

fn list_models_via_cli(
    provider: &str,
    command: &str,
    env: &Option<HashMap<String, String>>,
) -> Result<Vec<String>, String> {
    if provider == "claude" {
        return list_claude_models_via_prompt(command, env);
    }
    let attempts: Vec<Vec<&str>> = vec![
        vec!["models", "list", "--output-format", "json"],
        vec!["models", "list", "--output", "json"],
        vec!["models", "list", "--format", "json"],
        vec!["models", "list"],
        vec!["--list-models", "--output-format", "json"],
        vec!["--list-models"],
    ];
    let mut last_error = None;
    for args in attempts {
        match run_cli_with_env(command, &args, env) {
            Ok(stdout) => {
                let parsed = serde_json::from_str::<Value>(&stdout).ok();
                let mut models = if let Some(payload) = parsed {
                    collect_models_from_json(provider, &payload)
                } else {
                    collect_models_from_text(provider, &stdout)
                };
                models = collect_unique_models(models);
                if !models.is_empty() {
                    return Ok(models);
                }
                last_error = Some("CLI returned no models.".to_string());
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| "CLI model list failed.".to_string()))
}

fn preflight_claude_model_via_cli(
    command: &str,
    model: &str,
    env: &Option<HashMap<String, String>>,
) -> Result<String, String> {
    let normalized_model = normalize_claude_cli_model(model);
    if normalized_model.is_empty() {
        return Err("Model is required".to_string());
    }
    let args = [
        "-p",
        "--output-format",
        "json",
        "--input-format",
        "text",
        "--model",
        normalized_model.as_str(),
        "Reply with OK only.",
    ];
    run_cli_with_env(command, &args, env)?;
    Ok(normalized_model)
}

#[tauri::command]
pub(crate) async fn preflight_other_ai_model_cli(
    provider: String,
    command: String,
    model: String,
    env: Option<HashMap<String, String>>,
) -> Result<String, String> {
    let normalized_provider = provider.trim().to_lowercase();
    let command = command.trim();
    if command.is_empty() {
        return Err("CLI command is required".to_string());
    }
    let model = model.trim();
    if model.is_empty() {
        return Err("Model is required".to_string());
    }
    match normalized_provider.as_str() {
        "claude" => preflight_claude_model_via_cli(command, model, &env),
        // Gemini CLI model validation differs by distribution; keep no-op preflight for now.
        "gemini" => Ok(model.to_string()),
        _ => Err("Unsupported provider".to_string()),
    }
}

#[tauri::command]
pub(crate) async fn list_other_ai_models(
    provider: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    let normalized = provider.trim().to_lowercase();
    let client = Client::new();
    match normalized.as_str() {
        "claude" => list_claude_models(&client, &api_key).await,
        "gemini" => list_gemini_models(&client, &api_key).await,
        _ => Err("Unsupported provider".to_string()),
    }
}

#[tauri::command]
pub(crate) async fn list_other_ai_models_cli(
    provider: String,
    command: String,
    env: Option<HashMap<String, String>>,
) -> Result<Vec<String>, String> {
    let normalized = provider.trim().to_lowercase();
    let command = command.trim();
    if command.is_empty() {
        return Err("CLI command is required".to_string());
    }
    match normalized.as_str() {
        "claude" | "gemini" => list_models_via_cli(&normalized, command, &env),
        _ => Err("Unsupported provider".to_string()),
    }
}

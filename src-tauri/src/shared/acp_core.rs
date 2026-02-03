use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const MAX_MESSAGE_SIZE: usize = 8 * 1024 * 1024;

pub(crate) struct AcpHost {
    sessions: HashMap<String, AcpSessionKind>,
}

enum AcpSessionKind {
    Process(AcpSession),
    Internal(InternalSession),
}

struct AcpSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

enum InternalProvider {
    Claude,
    Gemini,
}

struct InternalSession {
    provider: InternalProvider,
    api_key: String,
}

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

impl AcpHost {
    pub(crate) fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub(crate) async fn start_session(
        &mut self,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> Result<String, String> {
        if let Some(provider) = internal_provider_from_command(&command) {
            let api_key = extract_api_key(&env)
                .ok_or_else(|| "ACP API key is required".to_string())?;
            let session_id = build_session_id();
            self.sessions.insert(
                session_id.clone(),
                AcpSessionKind::Internal(InternalSession { provider, api_key }),
            );
            return Ok(session_id);
        }
        let mut cmd = Command::new(&command);
        cmd.args(args);
        for (key, value) in env {
            cmd.env(key, value);
        }
        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|err| format!("ACP start failed: {err}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "ACP stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "ACP stdout unavailable".to_string())?;
        let session_id = build_session_id();
        self.sessions.insert(
            session_id.clone(),
            AcpSessionKind::Process(AcpSession {
                child,
                stdin,
                stdout: BufReader::new(stdout),
            }),
        );
        Ok(session_id)
    }

    pub(crate) async fn stop_session(&mut self, session_id: &str) -> Result<(), String> {
        if let Some(session) = self.sessions.remove(session_id) {
            if let AcpSessionKind::Process(mut process) = session {
                let _ = process.child.kill().await;
            }
        }
        Ok(())
    }

    pub(crate) async fn send(&mut self, session_id: &str, payload: Value) -> Result<Value, String> {
        self.send_stream(session_id, payload, |_| {}).await
    }

    pub(crate) async fn send_stream<F>(
        &mut self,
        session_id: &str,
        payload: Value,
        on_event: F,
    ) -> Result<Value, String>
    where
        F: FnMut(&Value),
    {
        let session_kind = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| "ACP session not found".to_string())?;
        match session_kind {
            AcpSessionKind::Process(session) => {
                send_process_request(session, payload, on_event).await
            }
            AcpSessionKind::Internal(session) => {
                send_internal_request(session, payload, on_event).await
            }
        }
    }
}

async fn send_process_request<F>(
    session: &mut AcpSession,
    payload: Value,
    mut on_event: F,
) -> Result<Value, String>
where
    F: FnMut(&Value),
{
    let request_id = extract_id(&payload);
    let body =
        serde_json::to_string(&payload).map_err(|err| format!("ACP serialize failed: {err}"))?;
    let header = format!("Content-Length: {}\r\n\r\n", body.as_bytes().len());
    session
        .stdin
        .write_all(header.as_bytes())
        .await
        .map_err(|err| format!("ACP write failed: {err}"))?;
    session
        .stdin
        .write_all(body.as_bytes())
        .await
        .map_err(|err| format!("ACP write failed: {err}"))?;
    session
        .stdin
        .flush()
        .await
        .map_err(|err| format!("ACP flush failed: {err}"))?;

    loop {
        let response = read_message(&mut session.stdout).await?;
        on_event(&response);
        if let Some(ref id) = request_id {
            let response_id = extract_id(&response);
            if response_id.as_deref() != Some(id) {
                continue;
            }
        }
        return Ok(response);
    }
}

async fn send_internal_request<F>(
    session: &mut InternalSession,
    payload: Value,
    mut on_event: F,
) -> Result<Value, String>
where
    F: FnMut(&Value),
{
    let (prompt, model) = extract_prompt_and_model(&payload)?;
    let content = match session.provider {
        InternalProvider::Claude => prompt_claude(&session.api_key, &model, &prompt).await?,
        InternalProvider::Gemini => prompt_gemini(&session.api_key, &model, &prompt).await?,
    };
    if !content.is_empty() {
        on_event(&json!({ "params": { "delta": content } }));
    }
    let response = json!({
        "jsonrpc": "2.0",
        "id": payload.get("id").cloned().unwrap_or_else(|| json!(null)),
        "result": { "content": content },
    });
    Ok(response)
}

fn internal_provider_from_command(command: &str) -> Option<InternalProvider> {
    match command.trim().to_lowercase().as_str() {
        "acp:claude" => Some(InternalProvider::Claude),
        "acp:gemini" => Some(InternalProvider::Gemini),
        _ => None,
    }
}

fn extract_api_key(env: &HashMap<String, String>) -> Option<String> {
    env.get("API_KEY")
        .or_else(|| env.get("ANTHROPIC_API_KEY"))
        .or_else(|| env.get("GEMINI_API_KEY"))
        .or_else(|| env.get("GOOGLE_API_KEY"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn extract_prompt_and_model(payload: &Value) -> Result<(String, String), String> {
    let params = payload
        .get("params")
        .and_then(|value| value.as_object())
        .ok_or_else(|| "ACP params missing".to_string())?;
    let prompt = params
        .get("prompt")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let model = params
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if prompt.is_empty() {
        return Err("ACP prompt missing".to_string());
    }
    if model.is_empty() {
        return Err("ACP model missing".to_string());
    }
    Ok((prompt, model))
}

async fn prompt_claude(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let client = Client::new();
    let body = json!({
        "model": model,
        "max_tokens": 4096,
        "messages": [{ "role": "user", "content": prompt }]
    });
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Claude API request failed: {err}"))?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!("Claude API error {status}: {error_body}"));
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|err| format!("Claude API response invalid: {err}"))?;
    let content = payload
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    Ok(content)
}

async fn prompt_gemini(api_key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
    );
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
        return Err(format!("Gemini API error {status}: {error_body}"));
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
    Ok(content)
}

fn build_session_id() -> String {
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("acp-{millis}-{counter}")
}

fn extract_id(value: &Value) -> Option<String> {
    value.get("id").and_then(|value| {
        value
            .as_i64()
            .map(|id| id.to_string())
            .or_else(|| value.as_str().map(|s| s.to_string()))
    })
}

async fn read_message(reader: &mut BufReader<ChildStdout>) -> Result<Value, String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .await
            .map_err(|err| format!("ACP read header failed: {err}"))?;
        if bytes == 0 {
            return Err("ACP stream closed".to_string());
        }
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            let parsed = rest.trim().parse::<usize>().map_err(|_| {
                "ACP invalid Content-Length".to_string()
            })?;
            content_length = Some(parsed);
        }
    }
    let length = content_length.ok_or_else(|| "ACP missing Content-Length".to_string())?;
    if length > MAX_MESSAGE_SIZE {
        return Err("ACP message too large".to_string());
    }
    let mut buffer = vec![0u8; length];
    reader
        .read_exact(&mut buffer)
        .await
        .map_err(|err| format!("ACP read body failed: {err}"))?;
    serde_json::from_slice::<Value>(&buffer).map_err(|err| format!("ACP parse failed: {err}"))
}

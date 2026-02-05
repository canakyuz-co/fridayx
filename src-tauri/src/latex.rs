use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::State;
use tokio::sync::Mutex;
use tokio::task;

use crate::state::AppState;

// Keep the cache small: PDF blobs are large and compilation is relatively slow.
const LATEX_CACHE_CAPACITY: usize = 8;
const LATEX_COMPILE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Deserialize)]
pub(crate) struct LatexCompileRequest {
    #[serde(rename = "workspaceId")]
    pub(crate) workspace_id: String,
    /// Workspace-relative path of the active .tex file (used to scope filesystem access).
    #[serde(rename = "path")]
    pub(crate) path: String,
    pub(crate) source: String,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct LatexDiagnostic {
    pub(crate) level: String, // "error" | "warning"
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) line: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct LatexCompileResponse {
    #[serde(rename = "pdfBase64")]
    pub(crate) pdf_base64: String,
    pub(crate) log: String,
    pub(crate) diagnostics: Vec<LatexDiagnostic>,
    #[serde(rename = "cacheHit")]
    pub(crate) cache_hit: bool,
}

#[derive(Default)]
struct LatexCompileCache {
    // digest(hex) -> response
    map: HashMap<String, LatexCompileResponse>,
    order: VecDeque<String>,
}

impl LatexCompileCache {
    fn get(&mut self, key: &str) -> Option<LatexCompileResponse> {
        self.map.get(key).cloned()
    }

    fn put(&mut self, key: String, value: LatexCompileResponse) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            return;
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
        while self.order.len() > LATEX_CACHE_CAPACITY {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
    }
}

static LATEX_CACHE: std::sync::OnceLock<Mutex<LatexCompileCache>> = std::sync::OnceLock::new();

fn cache() -> &'static Mutex<LatexCompileCache> {
    LATEX_CACHE.get_or_init(|| Mutex::new(LatexCompileCache::default()))
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    // Avoid an extra dependency for hex encoding.
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn resolve_filesystem_root(
    workspace_root: &Path,
    workspace_rel_path: &str,
) -> Result<PathBuf, String> {
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|_| "Workspace path gecersiz.".to_string())?;

    // Root for \input and friends: the directory containing the current .tex file.
    let file_abs = workspace_root.join(workspace_rel_path);
    let file_dir = file_abs.parent().unwrap_or(&workspace_root);

    let file_dir = file_dir
        .canonicalize()
        .map_err(|_| "Dosya dizini resolve edilemedi.".to_string())?;

    if !file_dir.starts_with(&workspace_root) {
        return Err("Guvensiz path: workspace disina cikis engellendi.".to_string());
    }

    Ok(file_dir)
}

fn parse_diagnostics_from_log(log: &str) -> Vec<LatexDiagnostic> {
    // Best-effort TeX log parsing. We intentionally keep this simple and stable:
    // - Errors typically start with '!' and may include a later 'l.<n>' line marker.
    let mut out = Vec::new();
    let mut lines = log.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

        if trimmed.starts_with('!') {
            let message = trimmed.trim_start_matches('!').trim().to_string();
            let mut found_line: Option<u32> = None;

            // TeX reports line markers in subsequent lines like: "l.42 ..."
            for _ in 0..6 {
                let Some(next) = lines.peek() else { break };
                let t = next.trim_start();
                if let Some(rest) = t.strip_prefix("l.") {
                    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if let Ok(n) = digits.parse::<u32>() {
                        found_line = Some(n);
                    }
                    break;
                }
                // Consume and continue scanning a few lines ahead.
                lines.next();
            }

            out.push(LatexDiagnostic {
                level: "error".to_string(),
                message,
                line: found_line,
            });
            continue;
        }

        // A lightweight warning capture; not all warnings have consistent formatting.
        if trimmed.contains("LaTeX Warning:") {
            out.push(LatexDiagnostic {
                level: "warning".to_string(),
                message: trimmed.to_string(),
                line: None,
            });
        }
    }

    out
}

fn compile_with_tectonic(source: &str, filesystem_root: &Path) -> Result<(Vec<u8>, String), String> {
    use tectonic::driver::{OutputFormat, ProcessingSessionBuilder};
    use tectonic::status::NoopStatusBackend;

    let mut status = NoopStatusBackend::default();
    let auto_create_config_file = false;
    let config = tectonic::config::PersistentConfig::open(auto_create_config_file)
        .map_err(|e| format!("Tectonic config acilamadi: {e}"))?;

    let only_cached = false;
    let bundle = config
        .default_bundle(only_cached, &mut status)
        .map_err(|e| format!("Tectonic bundle yuklenemedi: {e}"))?;

    let format_cache_path = config
        .format_cache_path()
        .map_err(|e| format!("Format cache path ayarlanamadi: {e}"))?;

    let mut sb = ProcessingSessionBuilder::default();
    sb.bundle(bundle)
        .primary_input_buffer(source.as_bytes())
        .tex_input_name("preview.tex")
        .filesystem_root(filesystem_root)
        .format_name("latex")
        .format_cache_path(format_cache_path)
        .keep_logs(true)
        .keep_intermediates(false)
        .print_stdout(false)
        .output_format(OutputFormat::Pdf)
        .do_not_write_output_files();

    let mut sess = sb
        .create(&mut status)
        .map_err(|e| format!("LaTeX oturumu baslatilamadi: {e}"))?;
    sess.run(&mut status)
        .map_err(|e| format!("LaTeX derleme basarisiz: {e}"))?;

    let mut files = sess.into_file_data();
    let pdf = files
        .remove("preview.pdf")
        .ok_or_else(|| "PDF olusmadi.".to_string())?
        .data;
    let log = files
        .remove("preview.log")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .unwrap_or_default();

    Ok((pdf, log))
}

#[tauri::command]
pub(crate) async fn latex_compile(
    state: State<'_, AppState>,
    req: LatexCompileRequest,
) -> Result<LatexCompileResponse, String> {
    let cache_key = sha256_hex(&format!("{}::{}", req.path, req.source));

    if let Some(hit) = cache().lock().await.get(&cache_key) {
        return Ok(LatexCompileResponse {
            cache_hit: true,
            ..hit
        });
    }

    let (workspace_root, filesystem_root) = {
        let workspaces = state.workspaces.lock().await;
        let entry = workspaces
            .get(&req.workspace_id)
            .cloned()
            .ok_or_else(|| "workspace not found".to_string())?;

        let workspace_root = PathBuf::from(entry.path);
        let filesystem_root = resolve_filesystem_root(&workspace_root, &req.path)?;
        (workspace_root, filesystem_root)
    };

    let source = req.source.clone();
    let compile_task = task::spawn_blocking(move || {
        // filesystem_root is scoped under workspace_root by resolve_filesystem_root().
        let _ = workspace_root; // keep ownership explicit for future hardening.
        compile_with_tectonic(&source, &filesystem_root)
    });

    let compile_result = tokio::time::timeout(LATEX_COMPILE_TIMEOUT, compile_task)
        .await
        .map_err(|_| "LaTeX derleme timeout (15s).".to_string())?
        .map_err(|_| "LaTeX derleme gorevi basarisiz.".to_string())?;

    let (pdf_bytes, log) = compile_result?;
    let diagnostics = parse_diagnostics_from_log(&log);

    let pdf_base64 = base64::engine::general_purpose::STANDARD.encode(pdf_bytes);
    let response = LatexCompileResponse {
        pdf_base64,
        log,
        diagnostics,
        cache_hit: false,
    };

    cache()
        .lock()
        .await
        .put(cache_key, response.clone());

    Ok(response)
}

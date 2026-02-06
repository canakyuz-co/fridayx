use base64::Engine;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::State;
use tokio::sync::Mutex;
use tokio::{fs, io::AsyncWriteExt, process::Command};

use crate::state::AppState;

// Keep the cache small: PDF blobs are large and compilation is relatively slow.
const LATEX_CACHE_CAPACITY: usize = 8;
// First-run compiles may need to download bundles/fonts; keep this generous.
const LATEX_COMPILE_TIMEOUT: Duration = Duration::from_secs(60);
const LATEX_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(45);
const LATEX_INSTALL_TIMEOUT: Duration = Duration::from_secs(90);

const TECTONIC_VERSION: &str = "0.15.0";
const TECTONIC_URL_AARCH64_APPLE_DARWIN: &str = "https://github.com/tectonic-typesetting/tectonic/releases/download/tectonic%400.15.0/tectonic-0.15.0-aarch64-apple-darwin.tar.gz";
const TECTONIC_URL_X86_64_APPLE_DARWIN: &str = "https://github.com/tectonic-typesetting/tectonic/releases/download/tectonic%400.15.0/tectonic-0.15.0-x86_64-apple-darwin.tar.gz";

// sha256(archive). Keep pinned so we can verify downloads.
const TECTONIC_SHA256_AARCH64_APPLE_DARWIN: &str =
    "24bd46566fa30d41101848405e9cbc4645edb92d8f857c9d21262174fb70cd33";
const TECTONIC_SHA256_X86_64_APPLE_DARWIN: &str =
    "dd42576eaa4c0df58c243dd78b7b864d9deb405ffdfcdadd1b79a31faceab747";

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
static LATEX_ENGINE_INSTALL_LOCK: std::sync::OnceLock<Mutex<()>> =
    std::sync::OnceLock::new();

fn cache() -> &'static Mutex<LatexCompileCache> {
    LATEX_CACHE.get_or_init(|| Mutex::new(LatexCompileCache::default()))
}

fn engine_install_lock() -> &'static Mutex<()> {
    LATEX_ENGINE_INSTALL_LOCK.get_or_init(|| Mutex::new(()))
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

fn app_data_dir(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    state
        .settings_path
        .parent()
        .map(|path| path.to_path_buf())
        .ok_or_else(|| "Unable to resolve app data dir.".to_string())
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

#[derive(Clone, Copy, Debug)]
enum LatexEngineKind {
    Tectonic,
    LatexMkXeLaTeX,
    XeLaTeX,
}

#[derive(Clone, Debug)]
struct LatexEngine {
    kind: LatexEngineKind,
    command: OsString,
}

async fn command_exists(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn detect_engine_on_path() -> Option<LatexEngine> {
    if command_exists("tectonic", &["--version"]).await {
        return Some(LatexEngine {
            kind: LatexEngineKind::Tectonic,
            command: OsString::from("tectonic"),
        });
    }
    if command_exists("latexmk", &["-v"]).await {
        return Some(LatexEngine {
            kind: LatexEngineKind::LatexMkXeLaTeX,
            command: OsString::from("latexmk"),
        });
    }
    if command_exists("xelatex", &["--version"]).await {
        return Some(LatexEngine {
            kind: LatexEngineKind::XeLaTeX,
            command: OsString::from("xelatex"),
        });
    }
    None
}

fn texinputs_env(filesystem_root: &Path) -> (OsString, OsString) {
    // Allow TeX to resolve \input/\includegraphics from the active file's directory.
    // The trailing "//" enables recursive search for some engines; ":" terminates the path list.
    let mut value = OsString::new();
    value.push(filesystem_root.as_os_str());
    value.push(OsString::from("//:"));
    (OsString::from("TEXINPUTS"), value)
}

fn bundled_tectonic_path(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let data_dir = app_data_dir(state)?;
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;

    // For now we only auto-install on macOS, where the app primarily targets.
    if os != "macos" {
        return Err("Auto-install yalnizca macOS icin aktif.".to_string());
    }

    let target = match arch {
        "aarch64" => "aarch64-apple-darwin",
        "x86_64" => "x86_64-apple-darwin",
        other => return Err(format!("Desteklenmeyen mimari: {other}")),
    };

    Ok(data_dir
        .join("tools")
        .join("latex")
        .join("tectonic")
        .join(TECTONIC_VERSION)
        .join(target)
        .join("tectonic"))
}

fn bundled_tectonic_download_spec() -> Option<(&'static str, &'static str)> {
    if std::env::consts::OS != "macos" {
        return None;
    }
    match std::env::consts::ARCH {
        "aarch64" => Some((
            TECTONIC_URL_AARCH64_APPLE_DARWIN,
            TECTONIC_SHA256_AARCH64_APPLE_DARWIN,
        )),
        "x86_64" => Some((
            TECTONIC_URL_X86_64_APPLE_DARWIN,
            TECTONIC_SHA256_X86_64_APPLE_DARWIN,
        )),
        _ => None,
    }
}

async fn ensure_bundled_tectonic(state: &State<'_, AppState>) -> Result<Option<LatexEngine>, String> {
    let Some((url, expected_sha)) = bundled_tectonic_download_spec() else {
        return Ok(None);
    };

    let exe_path = bundled_tectonic_path(state)?;
    if exe_path.exists() {
        return Ok(Some(LatexEngine {
            kind: LatexEngineKind::Tectonic,
            command: exe_path.into_os_string(),
        }));
    }

    // Serialize installation: avoids parallel downloads/extract races.
    let _guard = engine_install_lock().lock().await;

    let install_fut = async {
        if exe_path.exists() {
            return Ok(Some(LatexEngine {
                kind: LatexEngineKind::Tectonic,
                command: exe_path.into_os_string(),
            }));
        }

        let install_dir = exe_path
            .parent()
            .ok_or_else(|| "Install path gecersiz.".to_string())?;
        fs::create_dir_all(install_dir)
            .await
            .map_err(|e| format!("Install dizini olusturulamadi: {e}"))?;

        let tmp_archive = install_dir.join("tectonic.tmp.tar.gz");

        let client = reqwest::Client::builder()
            .timeout(LATEX_DOWNLOAD_TIMEOUT)
            .build()
            .map_err(|e| format!("HTTP client olusturulamadi: {e}"))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Tectonic indirilemedi: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("Tectonic indirilemedi (HTTP {}).", resp.status()));
        }

        let mut file = fs::File::create(&tmp_archive)
            .await
            .map_err(|e| format!("Temp arsiv yazilamadi: {e}"))?;

        // Stream download and compute sha256 incrementally to avoid holding large buffers.
        let mut hasher = Sha256::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| format!("Indirme akisi hatasi: {e}"))?;
            hasher.update(&bytes);
            file.write_all(&bytes)
                .await
                .map_err(|e| format!("Temp arsiv yazilamadi: {e}"))?;
        }
        file.flush()
            .await
            .map_err(|e| format!("Temp arsiv flush hatasi: {e}"))?;

        // Verify sha256.
        let digest = hasher.finalize();
        let mut got = String::with_capacity(digest.len() * 2);
        for b in digest {
            got.push_str(&format!("{:02x}", b));
        }
        if got != expected_sha {
            let _ = fs::remove_file(&tmp_archive).await;
            return Err("Tectonic checksum dogrulanamadi.".to_string());
        }

        // Extract tar.gz (blocking).
        let install_dir = install_dir.to_path_buf();
        let archive_bytes = fs::read(&tmp_archive)
            .await
            .map_err(|e| format!("Temp arsiv okunamadi: {e}"))?;
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut gz = flate2::read::GzDecoder::new(archive_bytes.as_slice());
            let mut archive = tar::Archive::new(&mut gz);
            archive
                .unpack(&install_dir)
                .map_err(|e| format!("Arsiv acilamadi: {e}"))?;
            Ok(())
        })
        .await
        .map_err(|_| "Arsiv acma gorevi basarisiz.".to_string())??;

        let _ = fs::remove_file(&tmp_archive).await;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&exe_path)
                .map_err(|e| format!("Binary metadata okunamadi: {e}"))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&exe_path, perms)
                .map_err(|e| format!("Binary permission ayarlanamadi: {e}"))?;
        }

        Ok(Some(LatexEngine {
            kind: LatexEngineKind::Tectonic,
            command: exe_path.into_os_string(),
        }))
    };

    tokio::time::timeout(LATEX_INSTALL_TIMEOUT, install_fut)
        .await
        .map_err(|_| "LaTeX motor kurulumu timeout.".to_string())?
}

async fn compile_with_engine(
    engine: LatexEngine,
    source: &str,
    filesystem_root: &Path,
) -> Result<(Vec<u8>, String), String> {
    let outdir = std::env::temp_dir().join(format!("friday-latex-preview-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&outdir)
        .await
        .map_err(|e| format!("Temp dizin olusturulamadi: {e}"))?;

    let tex_path = outdir.join("preview.tex");
    fs::write(&tex_path, source)
        .await
        .map_err(|e| format!("LaTeX kaynagi yazilamadi: {e}"))?;

    let (env_key, env_value) = texinputs_env(filesystem_root);

    let mut cmd = match engine.kind {
        LatexEngineKind::Tectonic => {
            // Tectonic: fast, self-contained (if installed).
            let mut c = Command::new(&engine.command);
            c.arg("-X")
                .arg("compile")
                .arg("--outdir")
                .arg(&outdir)
                .arg("--synctex")
                .arg("--keep-logs")
                .arg(&tex_path);
            c
        }
        LatexEngineKind::LatexMkXeLaTeX => {
            let mut c = Command::new(&engine.command);
            c.arg("-xelatex")
                .arg("-interaction=nonstopmode")
                .arg("-halt-on-error")
                .arg("-output-directory")
                .arg(&outdir)
                .arg(&tex_path);
            c
        }
        LatexEngineKind::XeLaTeX => {
            let mut c = Command::new(&engine.command);
            c.arg("-interaction=nonstopmode")
                .arg("-halt-on-error")
                .arg("-output-directory")
                .arg(&outdir)
                .arg(&tex_path);
            c
        }
    };

    // Set cwd to the active file's directory so relative paths resolve as expected.
    cmd.current_dir(filesystem_root);
    cmd.env(env_key, env_value);
    cmd.stdin(std::process::Stdio::null());

    let output = tokio::time::timeout(LATEX_COMPILE_TIMEOUT, cmd.output())
        .await
        .map_err(|_| "LaTeX derleme timeout (15s).".to_string())?
        .map_err(|e| format!("LaTeX calistirilamadi: {e}"))?;

    let log_path = outdir.join("preview.log");
    let log = fs::read_to_string(&log_path).await.unwrap_or_else(|_| {
        // Fallback: some engines only return stderr.
        String::from_utf8_lossy(&output.stderr).to_string()
    });

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let _ = fs::remove_dir_all(&outdir).await;
        return Err(format!(
            "LaTeX derleme basarisiz.\n\nSTDERR:\n{stderr}\n\nLOG:\n{log}"
        ));
    }

    let pdf_path = outdir.join("preview.pdf");
    let pdf_bytes = fs::read(&pdf_path)
        .await
        .map_err(|e| format!("PDF okunamadi: {e}"))?;

    let _ = fs::remove_dir_all(&outdir).await;
    Ok((pdf_bytes, log))
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

    let _ = workspace_root; // reserved for future hardening (e.g. stricter whitelists).

    let engine = if let Some(engine) = detect_engine_on_path().await {
        engine
    } else if let Some(engine) = ensure_bundled_tectonic(&state).await? {
        engine
    } else {
        return Err(
            "LaTeX motoru bulunamadi. \"tectonic\" veya \"latexmk\"/\"xelatex\" kurulu olmali."
                .to_string(),
        );
    };

    let (pdf_bytes, log) = compile_with_engine(engine, &req.source, &filesystem_root).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_tex_error_with_line() {
        let log = r#"
! Undefined control sequence.
l.12 \invalidcommand
            {}
"#;
        let diags = parse_diagnostics_from_log(log);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, "error");
        assert_eq!(diags[0].line, Some(12));
        assert!(diags[0].message.contains("Undefined control sequence"));
    }

    #[test]
    fn resolves_root_to_file_dir_and_blocks_escape() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("docs")).expect("mkdir docs");

        let ok = resolve_filesystem_root(root, "docs/main.tex").expect("ok root");
        assert!(ok.ends_with("docs"));

        // Absolute paths should not be allowed to override workspace scoping.
        let err = resolve_filesystem_root(root, "/etc/passwd").unwrap_err();
        assert!(err.to_lowercase().contains("guvensiz") || err.to_lowercase().contains("workspace"));
    }
}

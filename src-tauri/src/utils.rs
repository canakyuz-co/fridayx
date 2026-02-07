use std::env;
use std::ffi::OsString;
use std::cmp::Ordering;
use std::path::PathBuf;

#[allow(dead_code)]
pub(crate) fn normalize_git_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub(crate) fn resolve_git_binary() -> Result<PathBuf, String> {
    if let Some(path) = find_in_path("git") {
        return Ok(path);
    }
    if cfg!(windows) {
        if let Some(path) = find_in_path("git.exe") {
            return Ok(path);
        }
    }

    let candidates: &[&str] = if cfg!(windows) {
        &[
            "C:\\Program Files\\Git\\bin\\git.exe",
            "C:\\Program Files (x86)\\Git\\bin\\git.exe",
        ]
    } else {
        &[
            "/opt/homebrew/bin/git",
            "/usr/local/bin/git",
            "/usr/bin/git",
            "/opt/local/bin/git",
            "/run/current-system/sw/bin/git",
        ]
    };

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(format!(
        "Git not found. Install Git or ensure it is on PATH. Tried: {}",
        candidates.join(", ")
    ))
}

pub(crate) fn git_env_path() -> String {
    let mut paths: Vec<PathBuf> = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect())
        .unwrap_or_default();

    let defaults: &[&str] = if cfg!(windows) {
        &["C:\\Windows\\System32"]
    } else {
        &[
            "/usr/bin",
            "/bin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/opt/local/bin",
            "/run/current-system/sw/bin",
        ]
    };

    for candidate in defaults {
        let path = PathBuf::from(candidate);
        if !paths.contains(&path) {
            paths.push(path);
        }
    }

    let joined = env::join_paths(paths).unwrap_or_else(|_| OsString::new());
    joined.to_string_lossy().to_string()
}

pub(crate) fn tools_env_path() -> String {
    let mut paths: Vec<PathBuf> = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect())
        .unwrap_or_default();
    let mut prioritized_node_bins: Vec<PathBuf> = Vec::new();

    let defaults: &[&str] = if cfg!(windows) {
        &["C:\\Windows\\System32"]
    } else {
        &[
            "/usr/bin",
            "/bin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/opt/local/bin",
            "/run/current-system/sw/bin",
        ]
    };

    for candidate in defaults {
        let path = PathBuf::from(candidate);
        if !paths.contains(&path) {
            paths.push(path);
        }
    }

    // GUI apps often start with a minimal PATH; make a best-effort attempt to include Node bins from
    // popular version managers (NVM/Volta/asdf/mise/fnm) so Node-backed CLIs work reliably.
    if !cfg!(windows) {
        if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
            let home = PathBuf::from(home);

            let push_if_node = |paths: &mut Vec<PathBuf>, dir: PathBuf| {
                if dir.join("node").is_file() && !paths.contains(&dir) {
                    paths.push(dir);
                }
            };

            // NVM: ~/.nvm/versions/node/<version>/bin
            let nvm_base = home.join(".nvm/versions/node");
            if let Ok(entries) = std::fs::read_dir(&nvm_base) {
                let mut version_dirs = entries.flatten().map(|entry| entry.path()).collect::<Vec<_>>();
                version_dirs.sort_by(|left, right| compare_node_version_dirs(right, left));
                for version_dir in version_dirs {
                    push_if_node(&mut prioritized_node_bins, version_dir.join("bin"));
                }
            }

            // Volta: ~/.volta/bin
            push_if_node(&mut prioritized_node_bins, home.join(".volta/bin"));

            // asdf: ~/.asdf/shims
            push_if_node(&mut prioritized_node_bins, home.join(".asdf/shims"));

            // mise: ~/.local/share/mise/shims
            push_if_node(&mut prioritized_node_bins, home.join(".local/share/mise/shims"));

            // fnm: ~/.fnm/node-versions/<version>/installation/bin
            let fnm_base = home.join(".fnm/node-versions");
            if let Ok(entries) = std::fs::read_dir(&fnm_base) {
                let mut version_dirs = entries.flatten().map(|entry| entry.path()).collect::<Vec<_>>();
                version_dirs.sort_by(|left, right| compare_node_version_dirs(right, left));
                for version_dir in version_dirs {
                    push_if_node(&mut prioritized_node_bins, version_dir.join("installation/bin"));
                }
            }
        }
    }

    for path in prioritized_node_bins.into_iter().rev() {
        if let Some(index) = paths.iter().position(|candidate| candidate == &path) {
            paths.remove(index);
        }
        paths.insert(0, path);
    }

    let joined = env::join_paths(paths).unwrap_or_else(|_| OsString::new());
    joined.to_string_lossy().to_string()
}

fn parse_node_version_parts(value: &str) -> Option<Vec<u32>> {
    let normalized = value.strip_prefix('v').unwrap_or(value);
    let mut parts = Vec::new();
    for token in normalized.split('.') {
        let mut digits = String::new();
        for ch in token.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else {
                break;
            }
        }
        if digits.is_empty() {
            break;
        }
        let parsed = digits.parse::<u32>().ok()?;
        parts.push(parsed);
    }
    if parts.is_empty() { None } else { Some(parts) }
}

fn compare_semver_parts(left: &[u32], right: &[u32]) -> Ordering {
    let max_len = left.len().max(right.len());
    for idx in 0..max_len {
        let left_part = left.get(idx).copied().unwrap_or(0);
        let right_part = right.get(idx).copied().unwrap_or(0);
        if left_part != right_part {
            return left_part.cmp(&right_part);
        }
    }
    Ordering::Equal
}

fn compare_node_version_dirs(left: &PathBuf, right: &PathBuf) -> Ordering {
    let left_name = left.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let right_name = right.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    match (
        parse_node_version_parts(left_name),
        parse_node_version_parts(right_name),
    ) {
        (Some(left_parts), Some(right_parts)) => compare_semver_parts(&left_parts, &right_parts),
        _ => left_name.cmp(right_name),
    }
}

#[cfg(test)]
mod tests {
    use super::{compare_node_version_dirs, normalize_git_path, parse_node_version_parts};
    use std::cmp::Ordering;
    use std::path::PathBuf;

    #[test]
    fn normalize_git_path_replaces_backslashes() {
        assert_eq!(normalize_git_path("foo\\bar\\baz"), "foo/bar/baz");
    }

    #[test]
    fn parse_node_version_parts_extracts_semver() {
        assert_eq!(parse_node_version_parts("v22.18.0"), Some(vec![22, 18, 0]));
        assert_eq!(parse_node_version_parts("22.16.1"), Some(vec![22, 16, 1]));
        assert_eq!(parse_node_version_parts("release"), None);
    }

    #[test]
    fn compare_node_version_dirs_prefers_newer_versions() {
        let newer = PathBuf::from("/tmp/v22.18.0");
        let older = PathBuf::from("/tmp/v22.16.0");
        assert_eq!(compare_node_version_dirs(&newer, &older), Ordering::Greater);
    }
}

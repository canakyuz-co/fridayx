use std::env;
use std::ffi::OsString;
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
        for entry in entries.flatten() {
          push_if_node(&mut paths, entry.path().join("bin"));
        }
      }

      // Volta: ~/.volta/bin
      push_if_node(&mut paths, home.join(".volta/bin"));

      // asdf: ~/.asdf/shims
      push_if_node(&mut paths, home.join(".asdf/shims"));

      // mise: ~/.local/share/mise/shims
      push_if_node(&mut paths, home.join(".local/share/mise/shims"));

      // fnm: ~/.fnm/node-versions/<version>/installation/bin
      let fnm_base = home.join(".fnm/node-versions");
      if let Ok(entries) = std::fs::read_dir(&fnm_base) {
        for entry in entries.flatten() {
          push_if_node(
            &mut paths,
            entry.path().join("installation/bin"),
          );
        }
      }
    }
  }

    let joined = env::join_paths(paths).unwrap_or_else(|_| OsString::new());
    joined.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_git_path;

    #[test]
    fn normalize_git_path_replaces_backslashes() {
        assert_eq!(normalize_git_path("foo\\bar\\baz"), "foo/bar/baz");
    }
}

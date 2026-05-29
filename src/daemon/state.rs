use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProxyEntry {
    pub provider: String,
    pub upstream: String,
    pub proxy_port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_port: Option<u16>,
    pub data_dir: PathBuf,
    pub started_at: String,
    pub restart_count: u32,
}

/// Version of the `cc-switch` binary that built this crate. Recorded into the
/// daemon state at start time so a newer CLI can detect a stale running daemon.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DaemonState {
    pub schema_version: u32,
    /// `cc-switch` version that started the daemon. Empty for state files
    /// written before version tracking existed (treated as a mismatch).
    #[serde(default)]
    pub version: String,
    pub pid: u32,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub data_root: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agg_port: Option<u16>,
    pub proxies: Vec<ProxyEntry>,
}

impl DaemonState {
    /// Load state from disk. Returns `Ok(None)` when the file does not exist;
    /// returns `Err` with the path on corrupt JSON or other IO errors.
    pub fn load(path: &Path) -> Result<Option<DaemonState>> {
        let raw = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed to read daemon state at {}", path.display()));
            }
        };
        let state: DaemonState = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse daemon state at {}", path.display()))?;
        Ok(Some(state))
    }

    /// Save state atomically: write to `<path>.tmp` (mode 0600 on Unix),
    /// fsync, then rename over `path`.
    pub fn save(&self, path: &Path) -> Result<()> {
        let tmp_path = PathBuf::from(format!("{}.tmp", path.display()));
        let json = serde_json::to_string_pretty(self)
            .context("failed to serialize daemon state to JSON")?;
        write_tmp_then_rename(&tmp_path, path, json.as_bytes())
    }

    /// True when the running daemon was started by a binary whose version
    /// differs from this binary (or predates version tracking). Used to warn
    /// the user that the daemon should be restarted.
    pub fn version_mismatch(&self) -> bool {
        self.version != CURRENT_VERSION
    }

    /// Exact-match lookup. No URL normalization.
    pub fn find_proxy(&self, provider: &str, upstream: &str) -> Option<&ProxyEntry> {
        self.proxies
            .iter()
            .find(|entry| entry.provider == provider && entry.upstream == upstream)
    }
}

fn write_tmp_then_rename(tmp_path: &Path, final_path: &Path, bytes: &[u8]) -> Result<()> {
    {
        let mut file = open_tmp_for_write(tmp_path)?;
        file.write_all(bytes)
            .with_context(|| format!("failed to write daemon state to {}", tmp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to fsync daemon state at {}", tmp_path.display()))?;
    }
    std::fs::rename(tmp_path, final_path).with_context(|| {
        format!(
            "failed to rename {} -> {}",
            tmp_path.display(),
            final_path.display()
        )
    })
}

#[cfg(unix)]
fn open_tmp_for_write(tmp_path: &Path) -> Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(tmp_path)
        .with_context(|| format!("failed to open {} for write", tmp_path.display()))
}

#[cfg(not(unix))]
fn open_tmp_for_write(tmp_path: &Path) -> Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(tmp_path)
        .with_context(|| format!("failed to open {} for write", tmp_path.display()))
}

#[cfg(test)]
mod tests {
    use super::{DaemonState, ProxyEntry};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn sample_proxy(provider: &str, upstream: &str, proxy_port: u16) -> ProxyEntry {
        ProxyEntry {
            provider: provider.to_owned(),
            upstream: upstream.to_owned(),
            proxy_port,
            api_port: Some(9000),
            data_dir: PathBuf::from("/tmp/ccs"),
            started_at: "2026-05-28T00:00:00Z".to_owned(),
            restart_count: 0,
        }
    }

    fn sample_state(proxies: Vec<ProxyEntry>) -> DaemonState {
        DaemonState {
            schema_version: 2,
            version: super::CURRENT_VERSION.to_owned(),
            pid: 4242,
            started_at: "2026-05-28T00:00:00Z".to_owned(),
            stopped_at: None,
            data_root: PathBuf::from("/tmp/ccs"),
            agg_port: None,
            proxies,
        }
    }

    #[test]
    fn load_save_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let state = sample_state(vec![
            sample_proxy("claude", "https://api.anthropic.com", 8080),
            sample_proxy("codex", "https://api.openai.com", 8081),
        ]);
        state.save(&path).unwrap();
        let loaded = DaemonState::load(&path).unwrap().expect("file exists");
        assert_eq!(state, loaded);
    }

    #[test]
    fn load_save_round_trip_with_none_ports() {
        // Regression: api_port/agg_port use skip_serializing_if, so when they are
        // None the fields are omitted on save. Without #[serde(default)] the
        // reload fails with "missing field". Guard the None path explicitly.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let mut proxy = sample_proxy("claude", "https://api.anthropic.com", 8080);
        proxy.api_port = None;
        let mut state = sample_state(vec![proxy]);
        state.agg_port = None;
        state.save(&path).unwrap();
        let loaded = DaemonState::load(&path).unwrap().expect("file exists");
        assert_eq!(state, loaded);
    }

    #[test]
    fn version_mismatch_detection() {
        let mut state = sample_state(vec![]);
        // sample_state stamps CURRENT_VERSION → matches.
        assert!(!state.version_mismatch());
        state.version = "0.0.1-old".to_owned();
        assert!(state.version_mismatch());
        // Pre-version state files deserialize to "" → treated as a mismatch.
        state.version = String::new();
        assert!(state.version_mismatch());
    }

    #[test]
    fn load_pre_version_state_defaults_version_empty() {
        // A state file written before version tracking has no `version` key.
        let json = r#"{
            "schema_version": 2,
            "pid": 100,
            "started_at": "2026-05-28T00:00:00Z",
            "stopped_at": null,
            "data_root": "/tmp/ccs",
            "proxies": []
        }"#;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        std::fs::write(&path, json).unwrap();
        let loaded = DaemonState::load(&path).unwrap().expect("file exists");
        assert_eq!(loaded.version, "");
        assert!(loaded.version_mismatch());
    }

    #[test]
    fn load_missing_file_returns_none() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does_not_exist.json");
        assert!(DaemonState::load(&path).unwrap().is_none());
    }

    #[test]
    fn load_corrupt_json_returns_err_with_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("corrupt.json");
        std::fs::write(&path, "{not json").unwrap();
        let err = DaemonState::load(&path).unwrap_err();
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains(path.to_string_lossy().as_ref()),
            "error message should contain path; got: {rendered}"
        );
    }

    #[test]
    fn find_proxy_exact_match() {
        let entry = sample_proxy("claude", "https://api.anthropic.com", 8080);
        let state = sample_state(vec![entry.clone()]);
        assert_eq!(
            state.find_proxy("claude", "https://api.anthropic.com"),
            Some(&entry)
        );
        assert_eq!(
            state.find_proxy("claude", "https://api.anthropic.com/"),
            None
        );
        assert_eq!(state.find_proxy("codex", "https://api.anthropic.com"), None);
    }

    #[test]
    fn save_atomic_no_partial_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let first = sample_state(vec![sample_proxy("claude", "https://a.example", 8080)]);
        first.save(&path).unwrap();
        let second = sample_state(vec![sample_proxy("codex", "https://b.example", 8081)]);
        second.save(&path).unwrap();

        let loaded = DaemonState::load(&path).unwrap().expect("file exists");
        assert_eq!(second, loaded);

        let tmp_path = PathBuf::from(format!("{}.tmp", path.display()));
        assert!(
            !tmp_path.exists(),
            "temp file {tmp_path:?} should be renamed away after save"
        );
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_unix_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let state = sample_state(vec![]);
        state.save(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "expected 0600, got {:o}", mode & 0o777);
    }
}

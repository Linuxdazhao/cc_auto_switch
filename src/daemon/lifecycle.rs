use anyhow::Result;
use std::path::PathBuf;

/// `(provider, upstream_url)` pair identifying an upstream the daemon should proxy.
pub type Upstream = (String, String);

pub struct LifecycleConfig {
    pub state_path: PathBuf,
    pub pidfile_path: PathBuf,
    pub data_root: PathBuf,
    pub upstreams: Vec<Upstream>,
}

pub fn run_daemon_blocking(_cfg: LifecycleConfig) -> Result<()> {
    unimplemented!("Task 7")
}

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProxyEntry {
    pub provider: String,
    pub upstream: String,
    pub proxy_port: u16,
    pub api_port: u16,
    pub data_dir: PathBuf,
    pub started_at: String,
    pub restart_count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DaemonState {
    pub schema_version: u32,
    pub pid: u32,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub data_root: PathBuf,
    pub proxies: Vec<ProxyEntry>,
}

impl DaemonState {
    pub fn load(_path: &Path) -> anyhow::Result<Option<DaemonState>> {
        unimplemented!("Task 3")
    }
    pub fn save(&self, _path: &Path) -> anyhow::Result<()> {
        unimplemented!("Task 3")
    }
    pub fn find_proxy(&self, _provider: &str, _upstream: &str) -> Option<&ProxyEntry> {
        unimplemented!("Task 3")
    }
}

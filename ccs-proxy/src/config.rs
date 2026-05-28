use crate::provider::ProviderKind;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16, // 0 = OS auto
    pub api_port: u16,   // 0 = OS auto
    pub data_dir: PathBuf,
    pub redact: bool,
    pub cors_allow: Option<String>,
    pub api_server: bool,
}

impl ServeConfig {
    pub fn new(provider: ProviderKind, upstream: Url, data_dir: PathBuf) -> Self {
        Self {
            provider,
            upstream,
            proxy_port: 0,
            api_port: 0,
            data_dir,
            redact: true,
            cors_allow: None,
            api_server: true,
        }
    }
}

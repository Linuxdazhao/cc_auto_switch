use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServeError {
    #[error("failed to bind proxy port: {0}")]
    BindProxy(#[source] std::io::Error),

    #[error("failed to bind api port: {0}")]
    BindApi(#[source] std::io::Error),

    #[error("data dir creation failed at {path}: {source}")]
    DataDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(&'static str),

    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

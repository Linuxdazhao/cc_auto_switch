pub mod fs;

use crate::capture::CaptureRecord;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use fs::FsStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub provider: String,
    pub upstream: String,
    pub proxy_port: u16,
    pub api_port: u16,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub request_count: u64,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestSummary {
    pub seq: u64,
    pub session_id: String,
    pub request_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub model: Option<String>,
    pub status: Option<u16>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub has_error: bool,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Json(#[from] serde_json::Error),
    #[error("session not found: {0}")]
    SessionNotFound(String),
}

#[async_trait]
pub trait Store: Send + Sync + 'static {
    async fn init_session(&self, meta: SessionMeta) -> Result<(), StoreError>;
    async fn finalize_session(&self, session_id: &str) -> Result<(), StoreError>;
    async fn append(&self, rec: CaptureRecord) -> Result<(), StoreError>;
    async fn list_sessions(&self) -> Result<Vec<SessionMeta>, StoreError>;
    async fn list_requests(&self, session_id: &str)
    -> Result<Vec<RequestSummary>, StoreError>;
    async fn get_request(
        &self,
        session_id: &str,
        seq: u64,
    ) -> Result<Option<CaptureRecord>, StoreError>;
}

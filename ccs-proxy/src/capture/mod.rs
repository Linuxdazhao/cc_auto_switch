pub mod extract;
pub mod redact;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRecord {
    pub seq: u64,
    pub session_id: String,
    pub request_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub ttft_ms: Option<u64>,
    pub request: RequestPart,
    pub response: Option<ResponsePart>,
    pub usage: Option<Usage>,
    pub model: Option<String>,
    pub error: Option<CaptureError>,
    #[serde(default)]
    pub partial: bool,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPart {
    pub method: String,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePart {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body_reassembled: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_sse_text: Option<String>,
    pub raw_sse_frames_count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    UpstreamUnreachable,
    TlsHandshakeFailed,
    SseTruncated,
    ReassembleFailed,
    UpstreamHttpError,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CaptureEvent {
    RequestStarted {
        session_id: String,
        seq: u64,
        started_at: DateTime<Utc>,
        model: Option<String>,
    },
    RequestCompleted {
        session_id: String,
        seq: u64,
        duration_ms: u64,
        status: u16,
        request_id: Option<String>,
        usage: Option<Usage>,
        has_error: bool,
    },
}

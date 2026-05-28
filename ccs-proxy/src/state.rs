use crate::capture::CaptureEvent;
use crate::provider::ProviderKind;
use crate::session::SessionId;
use crate::store::Store;
use std::sync::Arc;
use tokio::sync::broadcast;
use url::Url;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn Store>,
    pub events: broadcast::Sender<CaptureEvent>,
    pub provider: ProviderKind,
    pub upstream: Url,
    pub session_id: SessionId,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub redact: bool,
}

impl AppState {
    pub fn new(
        store: Arc<dyn Store>,
        provider: ProviderKind,
        upstream: Url,
        session_id: SessionId,
        redact: bool,
    ) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            store,
            events: tx,
            provider,
            upstream,
            session_id,
            started_at: chrono::Utc::now(),
            redact,
        }
    }
}

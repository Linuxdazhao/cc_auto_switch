use ccs_proxy::api::build_api_app;
use ccs_proxy::capture::CaptureEvent;
use ccs_proxy::store::FsStore;
use ccs_proxy::{AppState, ProviderKind, SessionId};
use chrono::Utc;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use url::Url;

#[tokio::test]
async fn stream_pushes_request_started_event() {
    let dir = tempdir().unwrap();
    let store = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    let state = AppState::new(
        store,
        ProviderKind::Claude,
        Url::parse("https://api.anthropic.com").unwrap(),
        SessionId::new(),
        true,
    );
    let events = state.events.clone();
    let app = build_api_app(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    use tokio::io::AsyncWriteExt;
    stream
        .write_all(b"GET /api/stream HTTP/1.1\r\nHost: x\r\nAccept: text/event-stream\r\n\r\n")
        .await
        .unwrap();

    // Give server a tick to enroll subscriber
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    events
        .send(CaptureEvent::RequestStarted {
            session_id: "s".into(),
            seq: 1,
            started_at: Utc::now(),
            model: None,
        })
        .unwrap();

    // The SSE response is delivered as: HTTP headers, then chunked event frames.
    // Read repeatedly until we see the event marker or the 2s overall budget elapses.
    let mut buf = vec![0u8; 4096];
    let mut accum = String::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            panic!("timed out before event arrived; got: {accum}");
        }
        let n = tokio::time::timeout(remaining, stream.read(&mut buf))
            .await
            .unwrap_or_else(|_| panic!("read timed out; got: {accum}"))
            .unwrap();
        if n == 0 {
            panic!("connection closed; got: {accum}");
        }
        accum.push_str(&String::from_utf8_lossy(&buf[..n]));
        if accum.contains("event: request_started") {
            break;
        }
    }
}

//! End-to-end proxy round-trip: spin up wiremock as the upstream, point a
//! `build_proxy_app` at it, POST to `/v1/messages`, and assert that the
//! forwarded response body reaches the client AND that the background capture
//! task writes a `CaptureRecord` to the `FsStore` with the expected
//! `request_id`, status, and redacted `authorization` header.

use ccs_proxy::proxy::build_proxy_app;
use ccs_proxy::store::{FsStore, SessionMeta, Store};
use ccs_proxy::{AppState, ProviderKind, SessionId};
use chrono::Utc;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn proxies_messages_endpoint_and_captures_record() {
    let dir = tempdir().unwrap();

    let mock = MockServer::start().await;
    let sse_body = std::fs::read_to_string("tests/fixtures/claude_stream.sse").unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .insert_header("anthropic-request-id", "req_test_1")
                .set_body_string(sse_body),
        )
        .mount(&mock)
        .await;

    let store: Arc<dyn Store> = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    let session_id = SessionId::new();
    store
        .init_session(SessionMeta {
            session_id: session_id.as_str().to_string(),
            provider: "claude".into(),
            upstream: mock.uri(),
            proxy_port: 0,
            api_port: 0,
            started_at: Utc::now(),
            ended_at: None,
            request_count: 0,
            schema_version: 1,
            cwd: None,
            models: vec![],
        })
        .await
        .unwrap();

    let state = AppState::new(
        store.clone(),
        ProviderKind::Claude,
        Url::parse(&mock.uri()).unwrap(),
        session_id.clone(),
        true,
    );
    let app = build_proxy_app(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/v1/messages"))
        .header("content-type", "application/json")
        .header("authorization", "Bearer sk-test")
        .body(r#"{"model":"claude-sonnet-4-6","messages":[{"role":"user","content":"hi"}]}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("Hello"),
        "expected SSE body forwarded, got: {body}"
    );

    // Give the background capture task a moment to write.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let recs = store.list_requests(session_id.as_str()).await.unwrap();
    assert_eq!(recs.len(), 1, "expected exactly one captured record");

    let full = store
        .get_request(session_id.as_str(), 1)
        .await
        .unwrap()
        .expect("record at seq=1 should exist");
    assert_eq!(full.request_id.as_deref(), Some("req_test_1"));
    assert_eq!(full.response.as_ref().unwrap().status, 200);
    // Authorization redacted on disk (header key is lowercased per http crate norms):
    assert_eq!(
        full.request
            .headers
            .get("authorization")
            .map(|s| s.as_str()),
        Some("<redacted>")
    );
}

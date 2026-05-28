//! End-to-end smoke test of the top-level `serve()` entry point: bind both
//! ports, proxy a captured request, and verify the API `/health` endpoint
//! responds from the same handle.

use ccs_proxy::{ProviderKind, ServeConfig, serve};
use tempfile::tempdir;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn serve_binds_and_proxies() {
    let dir = tempdir().unwrap();
    let mock = MockServer::start().await;
    let sse = std::fs::read_to_string("tests/fixtures/claude_stream.sse").unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .insert_header("anthropic-request-id", "req_e2e")
                .set_body_string(sse),
        )
        .mount(&mock)
        .await;

    let cfg = ServeConfig::new(
        ProviderKind::Claude,
        Url::parse(&mock.uri()).unwrap(),
        dir.path().to_path_buf(),
    );
    let handle = serve(cfg).await.expect("serve");

    let body = reqwest::Client::new()
        .post(format!(
            "http://127.0.0.1:{}/v1/messages",
            handle.proxy_port
        ))
        .header("content-type", "application/json")
        .body(r#"{"model":"claude-sonnet-4-6","messages":[]}"#)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(body.contains("Hello"), "expected SSE body, got: {body}");

    let health: serde_json::Value = reqwest::get(format!(
        "http://127.0.0.1:{}/api/health",
        handle.api_port.unwrap()
    ))
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    assert_eq!(health["status"], "ok");

    handle.shutdown().await;
}

//! End-to-end test of the upstream-unreachable error path: when the configured
//! upstream URL points at a closed port, the proxy returns 502 with a JSON
//! envelope `{"error":{"type":"upstream_unreachable", ...}}` instead of
//! propagating an opaque connection error.

use ccs_proxy::{ProviderKind, ServeConfig, serve};
use tempfile::tempdir;
use url::Url;

#[tokio::test]
async fn returns_502_when_upstream_unreachable() {
    let dir = tempdir().unwrap();
    let cfg = ServeConfig::new(
        ProviderKind::Claude,
        Url::parse("http://127.0.0.1:1").unwrap(),
        dir.path().to_path_buf(),
    );
    let handle = serve(cfg).await.expect("serve");

    let resp = reqwest::Client::new()
        .post(format!(
            "http://127.0.0.1:{}/v1/messages",
            handle.proxy_port
        ))
        .header("content-type", "application/json")
        .body(r#"{"model":"claude-sonnet-4-6","messages":[]}"#)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 502);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["type"], "upstream_unreachable");

    handle.shutdown().await;
}

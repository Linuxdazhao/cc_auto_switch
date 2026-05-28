use axum::body::to_bytes;
use ccs_proxy::api::build_api_app;
use ccs_proxy::store::FsStore;
use ccs_proxy::{AppState, ProviderKind, SessionId};
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;
use url::Url;

#[tokio::test]
async fn health_returns_ok() {
    let dir = tempdir().unwrap();
    let store = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    let state = AppState::new(
        store,
        ProviderKind::Claude,
        Url::parse("https://api.anthropic.com").unwrap(),
        SessionId::new(),
        true,
    );
    let app = build_api_app(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["provider"], "claude");
    assert_eq!(json["store"], "ok");
}
